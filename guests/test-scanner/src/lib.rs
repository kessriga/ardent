//! A component for exercising sync's per-file recovery through the public scanner interface.

use std::cell::Cell;
use std::hash::{BuildHasher, RandomState};
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};

use ardent_format_guest as guest;
use guest::scanner::{ByteRange, Guest, Metadata, Node, ScanError};

pub struct Scanner;
#[cfg(target_arch = "wasm32")]
guest::export!(Scanner with_types_in guest);

thread_local! {
    static TRAPPED: Cell<bool> = const { Cell::new(false) };
}

impl Guest for Scanner {
    fn describe() -> Metadata {
        Metadata {
            name: "probe".into(),
            version: "0.1.0".into(),
            files: vec!["*.probe".into()],
            prefixes: vec![],
        }
    }

    fn scan(source: Vec<u8>) -> Result<Vec<Node>, ScanError> {
        if TRAPPED.get() {
            return Err(ScanError::Failure("reused a trapped instance".into()));
        }
        let mut label = String::from("text");
        let end = match source.as_slice() {
            b"deterministic" => {
                let clock = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| ScanError::Failure(error.to_string()))?;
                let hash = RandomState::new().hash_one("sample");
                label = format!(
                    "{} arguments, {} variables, {} ns, hash {hash}",
                    std::env::args_os().count(),
                    std::env::vars_os().count(),
                    clock.as_nanos()
                );
                source.len() as u32
            }
            b"filesystem" => {
                std::fs::metadata(".").map_err(|error| ScanError::Failure(error.to_string()))?;
                source.len() as u32
            }
            b"socket" => {
                UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
                    .map_err(|error| ScanError::Failure(error.to_string()))?;
                source.len() as u32
            }
            #[cfg(target_arch = "wasm32")]
            b"memory" => {
                std::hint::black_box(core::arch::wasm32::memory_grow::<0>(1024));
                source.len() as u32
            }
            b"trap" => {
                TRAPPED.set(true);
                std::process::abort()
            }
            b"invalid" => u32::MAX,
            _ => u32::try_from(source.len()).map_err(|e| ScanError::Failure(e.to_string()))?,
        };
        Ok(vec![Node {
            parent: None,
            kind: "text".into(),
            label,
            span: ByteRange { start: 0, end },
            label_ranges: vec![],
        }])
    }
}
