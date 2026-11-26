use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// A tracked path can change type after discovery. Open it without waiting for a pipe writer,
/// then check the handle that the caller will actually read.
pub(crate) fn open_regular(path: &Path) -> io::Result<File> {
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use rustix::fs::OFlags;
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(OFlags::NONBLOCK.bits() as i32);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a regular file",
        ));
    }
    Ok(file)
}

/// Read the full source for a scan or preview, returning an error if it exceeds `limit` bytes.
/// Pass the observed file size in `size` for an early size check.
pub(crate) fn read(mut file: impl Read, size: u64, limit: usize) -> io::Result<Vec<u8>> {
    let too_large = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("source exceeds the {limit}-byte scanner limit"),
        )
    };
    if size > limit as u64 {
        return Err(too_large());
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.by_ref().take(limit as u64).read_to_end(&mut bytes)?;
    // The file may have grown since the metadata check.
    if bytes.len() == limit && file.read(&mut [0])? != 0 {
        return Err(too_large());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn regular_file_reads_are_nonblocking_and_reject_other_file_types() -> io::Result<()> {
        use rustix::fs::{self, OFlags};
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("notes.md");
        File::create(&path)?;
        let source = open_regular(&path)?;
        assert!(fs::fcntl_getfl(&source)?.contains(OFlags::NONBLOCK));
        let pipe = dir.path().join("pipe");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&pipe)
                .status()?
                .success()
        );
        assert_eq!(
            open_regular(&pipe).expect_err("named pipe").kind(),
            io::ErrorKind::InvalidInput
        );
        Ok(())
    }

    #[test]
    fn accepts_the_exact_limit_and_checks_for_growth_after_stat() -> io::Result<()> {
        assert_eq!(read(&b"abc"[..], 1, 3)?, b"abc");
        let mut grown = &b"abcdef"[..];
        let error = read(&mut grown, 1, 3).expect_err("source grew past the limit");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(grown, b"ef");
        Ok(())
    }
}
