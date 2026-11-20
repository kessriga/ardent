//! Inline modules, items inside function bodies, and declared modules.
#![allow(dead_code)]

mod config;

/// The largest number of files in one scan.
pub const MAX_FILES: usize = 10_000;

static mut SCANS: u32 = 0;

macro_rules! log {
    ($($arg:tt)*) => { eprintln!($($arg)*) };
}

/// Runs a scan.
#[inline]
pub fn run(paths: &[&str]) -> usize {
    fn count(paths: &[&str]) -> usize {
        paths.len()
    }
    log!("scanning");
    count(paths)
}

pub mod walk {
    //! Walks a directory.

    pub type Depth = u8;

    pub fn walk(root: &str) {
        let _ = root;
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn walks() {
            walk(".");
        }
    }
}
