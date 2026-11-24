//! Decode terminal key sequences from stdin. Both CSI and SS3 cursors can arrive across separate reads.

use std::io::{self, Stdin};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use console::Key;
use rustix::event::{self, PollFd, PollFlags, Timespec};
use rustix::io as fd;
use rustix::termios::{self, OptionalActions};
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::{backend::SignalDelivery, exfiltrator::SignalOnly};
use signal_hook::{flag, low_level};

const TERMINATION: [i32; 4] = [SIGHUP, SIGINT, SIGQUIT, SIGTERM];
const ESCAPE_TIMEOUT: Timespec = Timespec {
    tv_sec: 0,
    tv_nsec: 400_000_000,
};

pub(super) fn with_raw_input<T>(
    pick: impl FnOnce(&mut dyn FnMut() -> io::Result<Key>) -> io::Result<T>,
) -> io::Result<T> {
    let input = io::stdin();
    let original = termios::tcgetattr(&input)?;
    let (read, write) = UnixStream::pair()?;
    let mut signals = SignalDelivery::with_pipe(read, write, SignalOnly, TERMINATION)?;
    // After the picker returns, the process must still terminate on these signals.
    let restored = Arc::new(AtomicBool::new(false));
    for signal in TERMINATION {
        flag::register_conditional_default(signal, Arc::clone(&restored))?;
    }
    let mut raw = original.clone();
    raw.make_raw();
    raw.output_modes = original.output_modes;
    termios::tcsetattr(&input, OptionalActions::Now, &raw)?;
    let result = pick(&mut || read_key(&input, signals.get_read()));
    let restore = termios::tcsetattr(&input, OptionalActions::Now, &original);
    restored.store(true, Ordering::SeqCst);
    if let Some(signal) = signals.pending().next() {
        low_level::emulate_default_handler(signal)?;
    }
    restore?;
    result
}

fn read_key(input: &Stdin, signals: &UnixStream) -> io::Result<Key> {
    let byte = read_byte(input, signals, None)?.ok_or(io::ErrorKind::UnexpectedEof)?;
    Ok(match byte {
        0x1b => return read_escape(input, signals),
        0x03 => Key::CtrlC,
        b'\t' => Key::Tab,
        b'\r' | b'\n' => Key::Enter,
        byte if byte.is_ascii() => Key::Char(char::from(byte)),
        _ => Key::Unknown,
    })
}

fn read_escape(input: &Stdin, signals: &UnixStream) -> io::Result<Key> {
    match read_byte(input, signals, Some(&ESCAPE_TIMEOUT))? {
        None => return Ok(Key::Escape),
        Some(0x03) => return Ok(Key::CtrlC),
        Some(b'[' | b'O') => {}
        _ => return Ok(Key::Unknown),
    }

    let mut has_parameters = false;
    while let Some(byte) = read_byte(input, signals, Some(&ESCAPE_TIMEOUT))? {
        if byte == 0x03 {
            return Ok(Key::CtrlC);
        }
        if (0x40..=0x7e).contains(&byte) {
            return Ok(match (has_parameters, byte) {
                (false, b'A') => Key::ArrowUp,
                (false, b'B') => Key::ArrowDown,
                (false, b'C') => Key::ArrowRight,
                (false, b'D') => Key::ArrowLeft,
                (false, b'Z') => Key::BackTab,
                _ => Key::Unknown,
            });
        }
        if !(0x20..=0x3f).contains(&byte) {
            break;
        }
        has_parameters = true;
    }
    Ok(Key::Unknown)
}

fn read_byte(
    input: &Stdin,
    signals: &UnixStream,
    timeout: Option<&Timespec>,
) -> io::Result<Option<u8>> {
    let mut waiting = [
        PollFd::new(input, PollFlags::IN),
        PollFd::new(signals, PollFlags::IN),
    ];
    if fd::retry_on_intr(|| event::poll(&mut waiting, timeout))? == 0 {
        return Ok(None);
    }
    if !waiting[1].revents().is_empty() {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let mut byte = [0];
    if fd::retry_on_intr(|| fd::read(input, &mut byte))? == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(Some(byte[0]))
}
