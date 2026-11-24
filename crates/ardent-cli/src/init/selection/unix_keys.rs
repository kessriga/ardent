//! Decode terminal key sequences from stdin. Both CSI and SS3 cursors can arrive across separate reads.

use std::io::{self, Stdin};

use console::Key;
use rustix::io as fd;
use rustix::termios::{self, OptionalActions, SpecialCodeIndex, Termios};

pub(super) fn read() -> io::Result<Key> {
    let input = io::stdin();
    let original = termios::tcgetattr(&input)?;
    let mut raw = original.clone();
    raw.make_raw();
    raw.output_modes = original.output_modes;
    termios::tcsetattr(&input, OptionalActions::Now, &raw)?;
    let key = read_key(&input, &mut raw);
    termios::tcsetattr(&input, OptionalActions::Now, &original)?;
    key
}

fn read_key(input: &Stdin, raw: &mut Termios) -> io::Result<Key> {
    let byte = read_byte(input)?.ok_or(io::ErrorKind::UnexpectedEof)?;
    Ok(match byte {
        0x1b => {
            // VTIME is in tenths of a second. Give a split arrow time to arrive before cancelling.
            raw.special_codes[SpecialCodeIndex::VMIN] = 0;
            raw.special_codes[SpecialCodeIndex::VTIME] = 4;
            termios::tcsetattr(input, OptionalActions::Now, raw)?;
            return read_escape(input);
        }
        0x03 => Key::CtrlC,
        b'\t' => Key::Tab,
        b'\r' | b'\n' => Key::Enter,
        byte if byte.is_ascii() => Key::Char(char::from(byte)),
        _ => Key::Unknown,
    })
}

fn read_escape(input: &Stdin) -> io::Result<Key> {
    match read_byte(input)? {
        None => return Ok(Key::Escape),
        Some(0x03) => return Ok(Key::CtrlC),
        Some(b'[' | b'O') => {}
        _ => return Ok(Key::Unknown),
    }

    let mut has_parameters = false;
    while let Some(byte) = read_byte(input)? {
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

fn read_byte(input: &Stdin) -> io::Result<Option<u8>> {
    let mut byte = [0];
    let count = fd::retry_on_intr(|| fd::read(input, &mut byte))?;
    Ok((count != 0).then_some(byte[0]))
}
