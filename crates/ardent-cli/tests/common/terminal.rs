use std::fs::File;
use std::io;

use rustix::fs::{self as posix, Mode, OFlags};
use rustix::io::{self as fd, FdFlags};
use rustix::pty::{self, OpenptFlags};
use rustix::termios::{self, Winsize};

pub fn resize(terminal: &File, rows: u16) -> io::Result<()> {
    let size = Winsize {
        ws_row: rows,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    Ok(termios::tcsetwinsize(terminal, size)?)
}

/// Return the nonblocking master used by the test and the slave attached to the child process.
pub fn open_terminal(rows: u16) -> io::Result<(File, File)> {
    let master = pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    fd::fcntl_setfd(&master, FdFlags::CLOEXEC)?;
    pty::grantpt(&master)?;
    pty::unlockpt(&master)?;
    let name = pty::ptsname(&master, Vec::new())?;
    let slave = File::from(posix::open(
        name.as_c_str(),
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let master = File::from(master);
    resize(&slave, rows)?;
    posix::fcntl_setfl(&master, posix::fcntl_getfl(&master)? | OFlags::NONBLOCK)?;
    Ok((master, slave))
}
