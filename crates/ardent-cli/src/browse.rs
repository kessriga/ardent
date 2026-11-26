//! Connect the terminal browser to project discovery, sync, and the source files on disk.

use std::cell::Cell;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use ardent_core::layout::{self, ARDENT_DIR};
use ardent_core::structure::{self, Structure};
use ardent_format::MAX_SOURCE_BYTES;
use ardent_tui::Document;

use crate::sync;

mod refresh;
mod watch;

use crate::sync::source;
use refresh::{Preview, Refresh, Revision};

pub(crate) fn run(working_dir: &Path) -> io::Result<ExitCode> {
    ardent_tui::require_terminal()?;
    let root = sync::find_root(working_dir).map_err(|err| io::Error::other(err.to_string()))?;
    let mut refresh = Refresh::start(root.clone())?;
    let (files, mut failed) = refresh.initial()?;
    let observed = Cell::new(None);
    ardent_tui::run(
        files,
        |path| load(&root, path, |preview| observed.set(Some(preview))),
        |open_path| {
            Ok(refresh
                .ready(open_path, observed.take())?
                .map(|(files, errors)| {
                    failed = errors;
                    files
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
fn load(root: &Path, path: &str, observed: impl FnOnce(Preview)) -> io::Result<Document> {
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
        outline: structure.outline,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ardent_core::outline::{Outline, ScannedNode};
    use ardent_core::structure::Stamp;
    use std::fs;
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
        assert!(load(project.path(), path, |_| {}).is_err());
        fs::write(project.path().join(path), source)?;
        let document = load(project.path(), path, |_| {})?;
        assert_eq!(
            document.bytes.get(document.outline.nodes[1].range.clone()),
            Some(b"hello".as_slice())
        );
        fs::write(project.path().join(path), "é\nworld")?;
        assert!(load(project.path(), path, |_| {}).is_err());
        fs::write(project.path().join(path), source)?;
        structure.outline.nodes[1].range.end += 1;
        store(&structure)?;
        assert!(load(project.path(), path, |_| {}).is_err());
        structure.outline = Outline::build(path, b"", &[]);
        structure.stamp.hash = structure::content_hash(b"");
        fs::write(project.path().join(path), "")?;
        store(&structure)?;
        assert_eq!(
            load(project.path(), path, |_| {})?.outline.nodes[0].range,
            0..0
        );
        Ok(())
    }
}
