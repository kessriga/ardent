use std::io::{self, Read};

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
