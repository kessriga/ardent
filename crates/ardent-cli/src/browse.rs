//! Connect the terminal browser to project discovery, sync, and the source files on disk.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::process::ExitCode;

use ardent_core::layout::{self, ARDENT_DIR};
use ardent_core::structure::{self, Structure};
use ardent_format::MAX_SOURCE_BYTES;
use ardent_tui::Document;

use crate::sync;

mod highlights;
mod refresh;
mod theme;
mod watch;

use crate::sync::source;
use highlights::Highlighter;
use refresh::{Preview, Refresh, Revision};

pub(crate) fn run(working_dir: &Path) -> io::Result<ExitCode> {
    ardent_tui::require_terminal()?;
    let root = sync::find_root(working_dir).map_err(|err| io::Error::other(err.to_string()))?;
    let mut refresh = Refresh::start(root.clone())?;
    let initial = refresh.initial()?;
    let mut failed = initial.failed;
    let highlighter = RefCell::new(Highlighter::new(initial.formats));
    let observed = Cell::new(None);
    ardent_tui::run(
        initial.files,
        |path| {
            load(&root, path, &mut highlighter.borrow_mut(), |preview| {
                observed.set(Some(preview))
            })
        },
        |open_path| {
            if open_path.is_none() {
                highlighter.borrow_mut().clear();
            }
            Ok(refresh.ready(open_path, observed.take())?.map(|update| {
                failed = update.failed;
                highlighter.borrow_mut().formats = update.formats;
                update.files
            }))
        },
    )?;
    Ok(if failed {
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
    highlighter: &mut Highlighter,
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
    Ok(Document {
        highlights: highlighter.spans(&structure.stamp, &bytes),
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
    fn refresh_reuses_highlights_only_for_matching_source_and_verified_format()
    -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        let dir = project.path().join(ARDENT_DIR);
        let installed = dir.join("plugins/markdown");
        fs::create_dir_all(&installed)?;
        let release = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/markdown");
        for file in ["format.wasm", "plugin.yaml"] {
            fs::copy(release.join(file), installed.join(file))?;
        }
        let plugin = FormatPlugin::load(&dir, "markdown")?.expect("installed Markdown package");
        let path = "sample.md";
        let source = b"# Title\n";
        let stamp = Stamp {
            size: source.len() as u64,
            mtime: SystemTime::UNIX_EPOCH,
            hash: structure::content_hash(source),
            format: plugin.name.clone(),
            format_version: plugin.version.clone(),
            format_hash: plugin.format_hash.clone(),
        };
        fs::write(project.path().join(path), source)?;
        let mut structure = Structure::new(stamp, Outline::build(path, source, &[]));
        let mut highlighter = Highlighter::new(vec![plugin]);
        let preview =
            |structure: &Structure, highlighter: &mut Highlighter| -> io::Result<Document> {
                layout::write_atomic(
                    &dir,
                    layout::structure_path(path),
                    structure.to_yaml()?.as_bytes(),
                )?;
                load(project.path(), path, highlighter, |_| {})
            };
        let first = preview(&structure, &mut highlighter)?.highlights;
        assert_eq!(first.len(), 1);
        // An unrelated save replaces the installed formats with freshly verified handles.
        highlighter.formats = FormatPlugin::load(&dir, "markdown")?.into_iter().collect();
        let refreshed = preview(&structure, &mut highlighter)?.highlights;
        assert!(Arc::ptr_eq(&first, &refreshed));
        let formats = std::mem::take(&mut highlighter.formats);
        assert!(preview(&structure, &mut highlighter)?.highlights.is_empty());
        highlighter.formats = formats;
        let changed = b"A `code`";
        fs::write(project.path().join(path), changed)?;
        assert!(preview(&structure, &mut highlighter).is_err());
        structure.stamp.hash = structure::content_hash(changed);
        let changed_spans = preview(&structure, &mut highlighter)?.highlights;
        assert_eq!(changed_spans[0].range, 2..8);
        assert_eq!(changed_spans[0].category, Category::Code);
        // An empty Wasm custom section gives the installed component a new digest.
        let component = installed.join("format.wasm");
        let mut rebuilt = fs::read(&component)?;
        rebuilt.extend_from_slice(&[0, 1, 0]);
        fs::write(component, &rebuilt)?;
        highlighter.formats = FormatPlugin::load(&dir, "markdown")?.into_iter().collect();
        let plain = preview(&structure, &mut highlighter)?;
        assert!(plain.highlights.is_empty());
        assert_eq!(plain.bytes, changed);
        structure.stamp.format_hash = structure::content_hash(&rebuilt);
        let updated = preview(&structure, &mut highlighter)?.highlights;
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
        let mut highlighter = Highlighter::default();
        store(&structure)?;
        assert!(load(project.path(), path, &mut highlighter, |_| {}).is_err());
        fs::write(project.path().join(path), source)?;
        let document = load(project.path(), path, &mut highlighter, |_| {})?;
        assert_eq!(
            document.bytes.get(document.outline.nodes[1].range.clone()),
            Some(b"hello".as_slice())
        );
        fs::write(project.path().join(path), "é\nworld")?;
        assert!(load(project.path(), path, &mut highlighter, |_| {}).is_err());
        fs::write(project.path().join(path), source)?;
        structure.outline.nodes[1].range.end += 1;
        store(&structure)?;
        assert!(load(project.path(), path, &mut highlighter, |_| {}).is_err());
        structure.outline = Outline::build(path, b"", &[]);
        structure.stamp.hash = structure::content_hash(b"");
        fs::write(project.path().join(path), "")?;
        store(&structure)?;
        assert_eq!(
            load(project.path(), path, &mut highlighter, |_| {})?
                .outline
                .nodes[0]
                .range,
            0..0
        );
        Ok(())
    }
}
