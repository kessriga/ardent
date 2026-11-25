#![cfg(any(target_os = "linux", target_os = "macos"))]

mod common;
#[path = "common/terminal.rs"]
mod pty;

use rustix::process::{self, Pid, Signal};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use rustix::termios;

struct Session(Child);

impl Session {
    fn open(dir: &Path, slave: &File) -> io::Result<Self> {
        Ok(Self(
            Command::new(env!("CARGO_BIN_EXE_ardent"))
                .current_dir(dir)
                .arg("browse")
                .stdin(slave.try_clone()?)
                .stdout(slave.try_clone()?)
                .stderr(slave.try_clone()?)
                .spawn()?,
        ))
    }

    fn signal(&self, signal: Signal) -> io::Result<()> {
        let pid = Pid::from_child(&self.0);
        Ok(process::kill_process(pid, signal)?)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for(master: &mut File, expected: &str) -> io::Result<String> {
    let start = Instant::now();
    let mut bytes = Vec::new();
    loop {
        match master.read_to_end(&mut bytes) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(err),
        }
        let text = String::from_utf8_lossy(&bytes);
        if text.contains(expected) {
            return Ok(text.into_owned());
        }
        if start.elapsed() > Duration::from_secs(15) {
            return Err(io::Error::other(format!(
                "waiting for {expected:?}, saw {text:?}"
            )));
        }
        thread::sleep(Duration::from_millis(15));
    }
}

#[test]
fn browses_from_a_subdirectory_and_refreshes_while_open() -> io::Result<()> {
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown,rust"])?
            .status
            .success()
    );
    let docs = root.join("docs");
    fs::create_dir(&docs)?;
    let markdown = root.join("a.md");
    let rust = docs.join("z.rs");
    let lines: Vec<_> = (0..35).map(|n| format!("source-line-{n:02}")).collect();
    let source = format!(
        "# Intro\n\nfirst passage\x1b[31m\n\n# Next\n\n{}\n",
        lines.join("\n")
    );
    fs::write(&markdown, &source)?;
    fs::write(&rust, "fn incomplete(")?;
    fs::write(root.join(".gitignore"), "skip.md\n")?;
    fs::write(root.join("skip.md"), "# Ignored\n")?;

    let (mut master, slave) = pty::open_terminal(12)?;
    let original = termios::tcgetattr(&slave)?.local_modes;
    let mut session = Session::open(&docs, &slave)?;
    let list = wait_for(&mut master, "Enter open")?;
    assert!(
        list.contains("a.md") && list.contains("docs/z.rs") && list.contains("syntax error"),
        "{list}"
    );
    assert!(!list.contains("skip.md"));
    master.write_all(b"\r")?;
    wait_for(&mut master, "PgUp/PgDn")?;
    master.write_all(b"\x1b[C\x1b[C\x1b[C")?;
    wait_for(&mut master, ">       paragraph")?;
    fs::write(&markdown, source.replace("first passage", "edited passage"))?;
    let edited = wait_for(&mut master, "edited passage")?;
    assert!(
        edited.contains(">       paragraph") && edited.contains("edited passage�[31m"),
        "{edited}"
    );
    assert!(!edited.contains("\x1b[31m") && !edited.contains("source-line-00"));
    master.write_all(b"\x1b[D]")?;
    let heading = wait_for(&mut master, ">   + Next")?;
    assert!(!heading.contains("source-line-06"));
    master.write_all(b"\x1b[6~")?;
    wait_for(&mut master, "source-line-06")?;
    master.write_all(b"\x1b")?;
    wait_for(&mut master, "Tracked files")?;
    master.write_all(b"\x1b[B\r")?;
    wait_for(&mut master, "syntax error")?;
    fs::write(&rust, "fn healthy() {}\n")?;
    wait_for(&mut master, "fn healthy() {}")?;
    fs::write(&rust, "fn healthy(")?;
    let broken = wait_for(&mut master, "syntax error")?;
    assert!(!broken.contains("fn healthy() {}"), "{broken}");
    fs::remove_file(&rust)?;
    wait_for(&mut master, "Tracked files")?;
    pty::resize(&slave, 8)?;
    session.signal(Signal::WINCH)?;
    wait_for(&mut master, "\x1b[8;1HUp/Down")?;
    master.write_all(b"q")?;
    wait_for(&mut master, "\x1b[?1049l")?;
    assert!(session.0.wait()?.success());
    assert_eq!(termios::tcgetattr(&slave)?.local_modes, original);
    Ok(())
}

#[test]
fn interruption_restores_an_empty_browser() -> io::Result<()> {
    let project = common::temp_dir()?;
    assert!(
        common::run_init(project.path(), &["--plugins", ""])?
            .status
            .success()
    );
    let (mut master, slave) = pty::open_terminal(12)?;
    let original = termios::tcgetattr(&slave)?.local_modes;
    let mut session = Session::open(project.path(), &slave)?;
    wait_for(&mut master, "No tracked source files")?;
    session.signal(Signal::TERM)?;
    wait_for(&mut master, "\x1b[?1049l")?;
    assert_eq!(session.0.wait()?.signal(), Some(Signal::TERM.as_raw()));
    assert_eq!(termios::tcgetattr(&slave)?.local_modes, original);
    Ok(())
}
