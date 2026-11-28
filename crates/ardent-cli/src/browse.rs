//! Connect the terminal browser to project discovery, sync, and the source files on disk.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::process::ExitCode;

use ardent_core::layout::{self, ARDENT_DIR};
use ardent_core::structure::{self, Structure};
use ardent_format::MAX_SOURCE_BYTES;
use ardent_tui::{BrowserUpdate, Document};

use crate::sync;

mod details;
mod refresh;
mod theme;
mod watch;

use crate::sync::source;
use details::DocumentPlugins;
use refresh::{Preview, Refresh, Revision};
use theme::ThemeWatcher;

pub(crate) fn run(working_dir: &Path) -> io::Result<ExitCode> {
    ardent_tui::require_terminal()?;
    let root = sync::find_root(working_dir).map_err(|err| io::Error::other(err.to_string()))?;
    let (themes, palette) = ThemeWatcher::start(&root)?;
    let mut refresh = Refresh::start(root.clone())?;
    let initial = refresh.initial()?;
    let mut invalid_selector = false;
    let mut failed = initial.failed;
    let plugins = RefCell::new(DocumentPlugins::new(initial.formats));
    let observed = Cell::new(None);
    ardent_tui::run(
        initial.files,
        palette,
        |path| {
            load(&root, path, &mut plugins.borrow_mut(), |preview| {
                observed.set(Some(preview))
            })
        },
        |open_path| {
            if open_path.is_none() {
                plugins.borrow_mut().clear();
            }
            if let Some(update) = themes.ready()? {
                invalid_selector = update.invalid_selector;
                return Ok(Some(BrowserUpdate::Theme(update.palette)));
            }
            Ok(refresh.ready(open_path, observed.take())?.map(|update| {
                failed = update.failed;
                plugins.borrow_mut().formats = update.formats;
                themes.recheck();
                BrowserUpdate::Files(update.files)
            }))
        },
    )?;
    Ok(if failed || invalid_selector {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Read the source and its cached structure as a pair. A failed scan can leave an older structure
/// with ranges that still fit inside the current file. Only a matching source hash makes them usable.
fn load(
    root: &Path,
    path: &str,
    plugins: &mut DocumentPlugins,
    observed: impl FnOnce(Preview),
) -> io::Result<Document> {
    let cached = root.join(ARDENT_DIR).join(layout::structure_path(path));
    let structure = Structure::from_yaml(&io::read_to_string(source::open_regular(&cached)?)?)
        .map_err(io::Error::other)?;
    if !structure.outline.matches_source_path(path) {
        return Err(io::Error::other(
            "structure belongs to a different source file",
        ));
    }
    let file = match source::open_regular(&root.join(path)) {
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            observed(Preview::recheck(path));
            return Err(err);
        }
        result => result?,
    };
    let metadata = file.metadata()?;
    let bytes = sync::source::read(file, metadata.len(), MAX_SOURCE_BYTES)?;
    let hash = structure::content_hash(&bytes);
    if hash != structure.stamp.hash {
        observed(Preview::recheck(path));
        return Err(io::Error::other(
            "source changed since its last successful scan",
        ));
    }
    if structure
        .outline
        .nodes
        .iter()
        .any(|node| bytes.get(node.range.clone()).is_none())
    {
        return Err(io::Error::other("node range is outside the source file"));
    }
    observed(Preview::loaded(
        path,
        Revision::from_source(&metadata, hash),
    ));
    let details = plugins.details(&structure.stamp, &bytes, structure.outline.nodes.len());
    Ok(Document {
        highlights: details.highlights,
        properties: details.properties,
        outline: structure.outline,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ardent_core::highlight::Category;
    use ardent_core::outline::{Outline, ScannedNode};
    use ardent_core::structure::Stamp;
    use ardent_format::FormatPlugin;
    use std::fs;
    use std::sync::Arc;
    use std::time::SystemTime;

    #[test]
    fn refresh_reuses_details_only_for_matching_source_and_verified_format()
    -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        let dir = project.path().join(ARDENT_DIR);
        let installed = dir.join("plugins/markdown");
        fs::create_dir_all(&installed)?;
        let release = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/markdown");
        for file in ["format.wasm", "plugin.yaml"] {
            fs::copy(release.join(file), installed.join(file))?;
        }
        let mut plugin = FormatPlugin::load(&dir, "markdown")?.expect("installed Markdown package");
        let path = "sample.md";
        let source = b"# Title\n\n- [ ] One\n";
        let stamp = Stamp {
            size: source.len() as u64,
            mtime: SystemTime::UNIX_EPOCH,
            hash: structure::content_hash(source),
            format: plugin.name.clone(),
            format_version: plugin.version.clone(),
            format_hash: plugin.format_hash.clone(),
        };
        fs::write(project.path().join(path), source)?;
        let mut structure = Structure::new(
            stamp,
            Outline::build(path, source, &plugin.scan(path, source)?),
        );
        let mut plugins = DocumentPlugins::new(vec![plugin]);
        let preview =
            |structure: &Structure, plugins: &mut DocumentPlugins| -> io::Result<Document> {
                layout::write_atomic(
                    &dir,
                    layout::structure_path(path),
                    structure.to_yaml()?.as_bytes(),
                )?;
                load(project.path(), path, plugins, |_| {})
            };
        let first = preview(&structure, &mut plugins)?;
        assert_eq!(first.highlights.len(), 1);
        let checkbox = |document: &Document| {
            assert_eq!(document.properties.len(), 1);
            let (&index, properties) = document
                .properties
                .first_key_value()
                .expect("checkbox properties");
            assert_eq!(document.outline.nodes[index].kind, "item");
            assert_eq!(
                document.outline.nodes[index].range,
                document.outline.nodes[index - 1].range
            );
            assert_eq!(properties[0].name, "checked");
            properties[0].value.clone()
        };
        assert_eq!(checkbox(&first), "false");
        // An unrelated save replaces the installed formats with freshly verified handles.
        plugins.formats = FormatPlugin::load(&dir, "markdown")?.into_iter().collect();
        let refreshed = preview(&structure, &mut plugins)?;
        assert!(Arc::ptr_eq(&first.highlights, &refreshed.highlights));
        assert!(Arc::ptr_eq(&first.properties, &refreshed.properties));
        let formats = std::mem::take(&mut plugins.formats);
        let absent = preview(&structure, &mut plugins)?;
        assert!(absent.highlights.is_empty());
        assert!(absent.properties.is_empty());
        plugins.formats = formats;
        let changed = b"A `code`\n\n- [x] One\n";
        fs::write(project.path().join(path), changed)?;
        assert!(preview(&structure, &mut plugins).is_err());
        structure.stamp.hash = structure::content_hash(changed);
        structure.outline = Outline::build(path, changed, &plugins.formats[0].scan(path, changed)?);
        let document = preview(&structure, &mut plugins)?;
        assert_eq!(checkbox(&document), "true");
        let changed_spans = document.highlights;
        assert_eq!(changed_spans[0].range, 2..8);
        assert_eq!(changed_spans[0].category, Category::Code);
        // An empty Wasm custom section gives the installed component a new digest.
        let component = installed.join("format.wasm");
        let mut rebuilt = fs::read(&component)?;
        rebuilt.extend_from_slice(&[0, 1, 0]);
        fs::write(component, &rebuilt)?;
        plugins.formats = FormatPlugin::load(&dir, "markdown")?.into_iter().collect();
        let plain = preview(&structure, &mut plugins)?;
        assert!(plain.highlights.is_empty());
        assert!(plain.properties.is_empty());
        assert_eq!(plain.bytes, changed);
        structure.stamp.format_hash = structure::content_hash(&rebuilt);
        let refreshed = preview(&structure, &mut plugins)?;
        assert_eq!(checkbox(&refreshed), "true");
        assert!(!Arc::ptr_eq(&document.properties, &refreshed.properties));
        let updated = refreshed.highlights;
        assert_eq!(updated, changed_spans);
        assert!(!Arc::ptr_eq(&updated, &changed_spans));
        Ok(())
    }

    #[test]
    fn preview_uses_only_matching_source_bytes_and_valid_ranges() -> io::Result<()> {
        let project = tempfile::tempdir()?;
        let path = "notes.md";
        let source = "é\nhello";
        let child = ScannedNode {
            kind: "paragraph".into(),
            label: "hello".into(),
            range: 3..8,
            label_ranges: vec![],
            children: vec![],
        };
        let stamp = Stamp {
            size: 8,
            mtime: SystemTime::UNIX_EPOCH,
            hash: structure::content_hash(source.as_bytes()),
            format: "markdown".into(),
            format_version: "1".into(),
            format_hash: String::new(),
        };
        let mut structure =
            Structure::new(stamp, Outline::build(path, source.as_bytes(), &[child]));
        let dir = project.path().join(ARDENT_DIR);
        fs::create_dir(&dir)?;
        let store = |s: &Structure| {
            layout::write_atomic(&dir, layout::structure_path(path), s.to_yaml()?.as_bytes())
        };
        let mut plugins = DocumentPlugins::default();
        store(&structure)?;
        assert!(load(project.path(), path, &mut plugins, |_| {}).is_err());
        fs::write(project.path().join(path), source)?;
        let document = load(project.path(), path, &mut plugins, |_| {})?;
        assert_eq!(
            document.bytes.get(document.outline.nodes[1].range.clone()),
            Some(b"hello".as_slice())
        );
        fs::write(project.path().join(path), "é\nworld")?;
        assert!(load(project.path(), path, &mut plugins, |_| {}).is_err());
        fs::write(project.path().join(path), source)?;
        structure.outline.nodes[1].range.end += 1;
        store(&structure)?;
        assert!(load(project.path(), path, &mut plugins, |_| {}).is_err());
        structure.outline = Outline::build(path, b"", &[]);
        structure.stamp.hash = structure::content_hash(b"");
        fs::write(project.path().join(path), "")?;
        store(&structure)?;
        assert_eq!(
            load(project.path(), path, &mut plugins, |_| {})?
                .outline
                .nodes[0]
                .range,
            0..0
        );
        Ok(())
    }
}
