use std::fmt;
use std::path::PathBuf;

/// A tracked file.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The path from the project root.
    pub path: PathBuf,
    size: u64,
}

pub struct Id(#[allow(unused)] pub u64, u32);

/// What a path points to.
pub enum Kind {
    File,
    /// A directory and its depth.
    Dir { depth: u8 },
    Link(PathBuf, bool),
}

pub trait Scan {
    type Output;
    const LIMIT: usize;

    fn scan(&self) -> Self::Output;

    fn limit(&self) -> usize {
        Self::LIMIT
    }
}

impl Entry {
    pub fn new(path: PathBuf) -> Self {
        Self { path, size: 0 }
    }
}

impl Scan for Entry {
    type Output = u64;
    const LIMIT: usize = 1;

    fn scan(&self) -> u64 {
        self.size
    }
}

impl<T: fmt::Debug> From<Vec<T>> for Kind where T: Clone {
    fn from(_: Vec<T>) -> Self {
        Kind::File
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl !Send for Id {}

unsafe impl Sync for Id {}

union Bits {
    int: u32,
    float: f32,
}

extern "C" {
    fn abs(x: i32) -> i32;
}
