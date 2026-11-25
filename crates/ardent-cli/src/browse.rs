use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::ExitCode;

use ardent_core::layout::{self, ARDENT_DIR};
use ardent_core::structure::{self, Structure};
use ardent_format::MAX_SOURCE_BYTES;
use ardent_tui::{Document, FileList};

use crate::sync;

mod watch;

pub(crate) fn run(working_dir: &Path) -> io::Result<ExitCode> {
    let root = sync::find_root(working_dir).map_err(|err| io::Error::other(err.to_string()))?;
    let mut changes = watch::Changes::new(&root)?;
    let mut failed = false;
    let mut sync_files = || {
        let report = sync::run(&root).map_err(|err| io::Error::other(err.to_string()))?;
        failed = !report.errors.is_empty() || report.files.iter().any(|(_, error)| error.is_some());
        Ok(FileList {
            files: report.files,
            messages: report.errors.into_iter().chain(report.warnings).collect(),
        })
    };
    let files = sync_files()?;
    ardent_tui::run(
        files,
        |path| load(&root, path),
        || {
            if changes.ready()? {
                sync_files().map(Some)
            } else {
                Ok(None)
            }
        },
    )?;
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn load(root: &Path, path: &str) -> io::Result<Document> {
    let cached = root.join(ARDENT_DIR).join(layout::structure_path(path));
    let structure = Structure::from_yaml(&fs::read_to_string(cached)?).map_err(io::Error::other)?;
    if !structure.outline.matches_source_path(path) {
        return Err(io::Error::other(
            "structure belongs to a different source file",
        ));
    }
    let file = File::open(root.join(path))?;
    let size = file.metadata()?.len();
    let bytes = sync::source::read(file, size, MAX_SOURCE_BYTES)?;
    if structure::content_hash(&bytes) != structure.stamp.hash {
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
    Ok(Document {
        outline: structure.outline,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ardent_core::outline::{Outline, ScannedNode};
    use ardent_core::structure::Stamp;
    use std::time::SystemTime;

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
        store(&structure)?;
        assert!(load(project.path(), path).is_err());
        fs::write(project.path().join(path), source)?;
        let document = load(project.path(), path)?;
        assert_eq!(
            document.bytes.get(document.outline.nodes[1].range.clone()),
            Some(b"hello".as_slice())
        );
        fs::write(project.path().join(path), "é\nworld")?;
        assert!(load(project.path(), path).is_err());
        fs::write(project.path().join(path), source)?;
        structure.outline.nodes[1].range.end += 1;
        store(&structure)?;
        assert!(load(project.path(), path).is_err());
        structure.outline = Outline::build(path, b"", &[]);
        structure.stamp.hash = structure::content_hash(b"");
        fs::write(project.path().join(path), "")?;
        store(&structure)?;
        assert_eq!(load(project.path(), path)?.outline.nodes[0].range, 0..0);
        Ok(())
    }
}
