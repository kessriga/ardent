#![cfg(any(target_os = "linux", target_os = "macos"))]

mod common;
#[path = "common/terminal.rs"]
mod pty;

use rustix::process::{self, Pid, Signal};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::num::NonZeroU16;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use rustix::termios;
use vt100::{Parser, Screen};

struct Session {
    child: Child,
    master: File,
    terminal: Parser,
}

impl Session {
    fn open(dir: &Path, slave: &File, master: File) -> io::Result<Self> {
        let mut terminal = Parser::default();
        set_terminal_size(&mut terminal, slave)?;
        Ok(Self {
            child: Command::new(env!("CARGO_BIN_EXE_ardent"))
                .current_dir(dir)
                .arg("browse")
                .stdin(slave.try_clone()?)
                .stdout(slave.try_clone()?)
                .stderr(slave.try_clone()?)
                .spawn()?,
            master,
            terminal,
        })
    }

    fn signal(&self, signal: Signal) -> io::Result<()> {
        let pid = Pid::from_child(&self.child);
        Ok(process::kill_process(pid, signal)?)
    }
    fn keys(&mut self, keys: &[u8]) -> io::Result<()> {
        self.master.write_all(keys)
    }

    fn resize(&mut self, slave: &File, rows: u16, columns: u16) -> io::Result<()> {
        pty::resize_to(slave, rows, columns)?;
        set_terminal_size(&mut self.terminal, slave)?;
        self.signal(Signal::WINCH)
    }

    fn wait_for(&mut self, text: &str) -> io::Result<String> {
        self.wait_screen(&[text], &[])
    }

    fn wait_screen(&mut self, present: &[&str], absent: &[&str]) -> io::Result<String> {
        self.wait_until(&format!("show {present:?}, hide {absent:?}"), |screen| {
            let text = screen.contents();
            present.iter().all(|s| text.contains(s)) && absent.iter().all(|s| !text.contains(s))
        })
    }

    fn wait_restored(&mut self) -> io::Result<()> {
        self.wait_until("restore the previous screen and cursor", |screen| {
            !screen.alternate_screen() && !screen.hide_cursor()
        })?;
        Ok(())
    }

    // Ratatui updates only the cells that changed. Assertions need the assembled screen.
    fn wait_until(&mut self, goal: &str, ready: impl Fn(&Screen) -> bool) -> io::Result<String> {
        let start = Instant::now();
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            match self.master.read_to_end(&mut bytes) {
                Ok(_) => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                Err(err) => return Err(err),
            }
            self.terminal.process(&bytes);
            let screen = self.terminal.screen();
            if ready(screen) {
                return Ok(screen.contents());
            }
            if start.elapsed() > Duration::from_secs(15) {
                return Err(io::Error::other(format!(
                    "waiting to {goal}, saw:\n{}",
                    screen.contents()
                )));
            }
            thread::sleep(Duration::from_millis(15));
        }
    }
}

fn set_terminal_size(terminal: &mut Parser, slave: &File) -> io::Result<()> {
    let size = termios::tcgetwinsize(slave)?;
    let rows = NonZeroU16::try_from(size.ws_row).map_err(io::Error::other)?;
    let columns = NonZeroU16::try_from(size.ws_col).map_err(io::Error::other)?;
    terminal.set_size(rows, columns);
    Ok(())
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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

    let (master, slave) = pty::open_terminal(12)?;
    let original = termios::tcgetattr(&slave)?.local_modes;
    let mut session = Session::open(&docs, &slave, master)?;
    session.wait_screen(&["Explorer", "docs/", "z.rs !", "a.md"], &["skip.md"])?;
    session.keys(b"\x1b[B\x1b[B\r")?;
    session.wait_for("# Intro")?;
    session.keys(b"\x1b[C\x1b[C\x1b[C")?;
    session.wait_for(">       paragraph")?;
    fs::write(&markdown, source.replace("first passage", "edited passage"))?;
    let edited = session.wait_for("edited passage")?;
    // A terminal would consume "[31m" if the source's escape byte reached the output.
    assert!(
        edited.contains(">       paragraph") && edited.contains("[31m"),
        "{edited}"
    );
    assert!(!edited.contains("source-line-00"));
    session.keys(b"\x1b[D]")?;
    let heading = session.wait_for(">   + Next")?;
    assert!(!heading.contains("source-line-04"));
    session.keys(b"\x1b[6~")?;
    session.wait_for("source-line-04")?;
    session.keys(b"\x1b")?;
    session.wait_screen(&["Explorer"], &["source-line-04"])?;
    session.keys(b"\x1b[A\r")?;
    session.wait_for("syntax error")?;
    fs::write(&rust, "fn healthy() {}\n")?;
    session.wait_for("fn healthy() {}")?;
    session.keys(b"\x1b[C")?;
    session.wait_for(">     fn healthy")?;
    fs::write(&rust, "fn healthy(")?;
    let broken = session.wait_for("syntax error")?;
    assert!(!broken.contains("fn healthy() {}"), "{broken}");
    fs::write(&rust, "fn first() {}\nfn healthy() {}\n")?;
    let repaired = session.wait_for(">     fn healthy")?;
    assert!(
        repaired.contains("fn healthy() {}") && !repaired.contains("fn first() {}"),
        "{repaired}"
    );
    fs::remove_file(&rust)?;
    session.wait_screen(&["Explorer", "a.md"], &["z.rs", "fn healthy"])?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
    assert_eq!(termios::tcgetattr(&slave)?.local_modes, original);
    Ok(())
}

#[test]
fn keeps_the_directory_explorer_beside_the_open_outline() -> io::Result<()> {
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown"])?
            .status
            .success()
    );
    let guide = root.join("docs/guide");
    fs::create_dir_all(&guide)?;
    fs::write(guide.join("alpha.md"), "# First passage\n")?;
    fs::write(root.join("beta.md"), "# Second passage\n")?;
    let (master, slave) = pty::open_terminal(18)?;
    let mut session = Session::open(root, &slave, master)?;

    session.wait_screen(&["Explorer", "docs/", "guide/", "alpha.md", "beta.md"], &[])?;
    session.keys(b"\x1b[D")?;
    session.wait_screen(&["docs/", "beta.md"], &["guide/", "alpha.md"])?;
    // Reopen docs/, descend through guide/, then open its file.
    session.keys(b"\r\x1b[C\x1b[C\r")?;
    let opened = session.wait_screen(
        &[
            "Explorer",
            "Outline",
            "Source",
            "docs/guide/alpha.md",
            "# First passage",
            "beta.md",
        ],
        &[],
    )?;
    assert!(
        opened
            .lines()
            .any(|line| line.contains("Explorer") && line.contains("Outline")),
        "{opened}"
    );

    // With focus in the explorer, move to beta.md and refresh the project without replacing the document.
    session.keys(b"\t\x1b[B")?;
    fs::write(root.join("z.md"), "# New file\n")?;
    session.wait_screen(&["z.md", "# First passage"], &["# Second passage"])?;
    session.resize(&slave, 12, 32)?;
    session.wait_screen(&["Explorer", "beta.md"], &["# First passage"])?;
    session.keys(b"\t")?;
    session.wait_screen(&["Source", "# First passage"], &["Explorer", "beta.md"])?;
    session.keys(b"\t\r")?;
    session.wait_screen(&["Source", "# Second passage"], &["# First passage"])?;
    session.resize(&slave, 18, 80)?;
    session.wait_screen(&["Explorer", "docs/", "Outline", "# Second passage"], &[])?;
    session.keys(b"\x1b")?;
    session.wait_screen(&["Explorer", "beta.md"], &["# Second passage"])?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
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
    let (master, slave) = pty::open_terminal(12)?;
    let original = termios::tcgetattr(&slave)?.local_modes;
    let mut session = Session::open(project.path(), &slave, master)?;
    session.wait_for("No tracked source files")?;
    session.signal(Signal::TERM)?;
    session.wait_restored()?;
    assert_eq!(session.child.wait()?.signal(), Some(Signal::TERM.as_raw()));
    assert_eq!(termios::tcgetattr(&slave)?.local_modes, original);
    Ok(())
}

#[test]
fn quit_during_refresh_restores_terminal_before_the_sync_lock_is_released() -> io::Result<()> {
    let project = common::temp_dir()?;
    let dir = project.path();
    assert!(
        common::run_init(dir, &["--plugins", "markdown"])?
            .status
            .success()
    );
    fs::write(dir.join("notes.md"), "# First\n")?;
    let (master, slave) = pty::open_terminal(12)?;
    let original = termios::tcgetattr(&slave)?.local_modes;
    let mut session = Session::open(dir, &slave, master)?;
    session.wait_for("Explorer")?;
    fs::write(dir.join("watch-ready.md"), "# Ready\n")?;
    session.wait_for("watch-ready.md")?;
    let lock = File::options()
        .write(true)
        .open(dir.join(".ardent/cache/lock"))?;
    lock.lock()?;
    fs::write(dir.join("notes.md"), "# Second\n")?;
    // Allow the polling backend's one-second interval and the event batch deadline to elapse.
    thread::sleep(Duration::from_secs(2));
    session.keys(b"\x1b[B\r")?;
    session.wait_for("# Ready")?;
    session.keys(b"q")?;
    session.wait_restored()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = session.child.try_wait()? {
            assert!(status.success(), "{status}");
            break;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other("quit waited on the sync lock"));
        }
        thread::sleep(Duration::from_millis(15));
    }
    assert_eq!(termios::tcgetattr(&slave)?.local_modes, original);
    Ok(())
}

#[test]
fn healthy_file_refreshes_with_an_unreadable_ignored_directory() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown"])?
            .status
            .success()
    );
    fs::write(root.join(".gitignore"), "private/\n")?;
    let private = root.join("private");
    fs::create_dir(&private)?;
    fs::set_permissions(&private, fs::Permissions::from_mode(0o000))?;
    let notes = root.join("notes.md");
    fs::write(&notes, "# Before\n")?;
    let (master, slave) = pty::open_terminal(12)?;
    let mut session = Session::open(root, &slave, master)?;
    session.wait_for("Explorer")?;
    session.keys(b"\r")?;
    session.wait_for("# Before")?;
    fs::write(notes, "# After\n")?;
    session.wait_for("# After")?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
    fs::set_permissions(private, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[test]
fn losing_a_nested_project_never_syncs_the_enclosing_project() -> io::Result<()> {
    let project = common::temp_dir()?;
    let outer = project.path();
    let inner = outer.join("nested");
    fs::create_dir(&inner)?;
    for root in [inner.as_path(), outer] {
        assert!(
            common::run_init(root, &["--plugins", "markdown"])?
                .status
                .success()
        );
    }
    fs::write(outer.join("parent.md"), "# Parent\n")?;
    let (master, slave) = pty::open_terminal(12)?;
    let mut session = Session::open(&inner, &slave, master)?;
    session.wait_for("Explorer")?;
    fs::rename(inner.join(".ardent"), outer.join("moved-inner"))?;
    session.wait_restored()?;
    assert!(!session.child.wait()?.success());
    assert!(!outer.join(".ardent/cache").exists());
    Ok(())
}

#[test]
fn rechecks_an_open_file_when_a_restore_preserves_its_stamp() -> io::Result<()> {
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown"])?
            .status
            .success()
    );
    let notes = root.join("notes.md");
    fs::write(&notes, "# First\n")?;
    let time = File::open(&notes)?.metadata()?.modified()?;
    let (master, slave) = pty::open_terminal(12)?;
    let mut session = Session::open(root, &slave, master)?;
    session.wait_for("Explorer")?;
    session.keys(b"\r")?;
    session.wait_for("# First")?;
    let replacement = root.join("replacement");
    fs::write(&replacement, "# Other\n")?;
    File::options()
        .write(true)
        .open(&replacement)?
        .set_modified(time)?;
    fs::rename(replacement, notes)?;
    session.wait_for("# Other")?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
    Ok(())
}

#[test]
fn restoring_read_permission_recovers_without_another_edit() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown"])?
            .status
            .success()
    );
    let notes = root.join("private.md");
    fs::write(&notes, "# Readable\n")?;
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o000))?;
    if File::open(&notes).is_ok() {
        return Ok(()); // A privileged process can read a mode-000 file.
    }
    let (master, slave) = pty::open_terminal(12)?;
    let mut session = Session::open(root, &slave, master)?;
    session.wait_for("Explorer")?;
    session.keys(b"\r")?;
    session.wait_for("Permission denied")?;
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o600))?;
    session.wait_for("# Readable")?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
    Ok(())
}

#[test]
fn repairs_an_open_syntax_error_without_a_size_or_mtime_change() -> io::Result<()> {
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "rust"])?
            .status
            .success()
    );
    let source = root.join("notes.rs");
    fs::write(&source, "fn item(  \n")?;
    let time = source.metadata()?.modified()?;
    let (master, slave) = pty::open_terminal(12)?;
    let mut session = Session::open(root, &slave, master)?;
    session.wait_for("Explorer")?;
    session.keys(b"\r")?;
    session.wait_for("syntax error")?;
    fs::write(&source, "fn item(){}")?;
    File::options()
        .write(true)
        .open(&source)?
        .set_modified(time)?;
    session.wait_for("fn item(){}")?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
    Ok(())
}

#[test]
fn redirected_browse_rejects_the_terminal_before_creating_a_cache() -> io::Result<()> {
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown"])?
            .status
            .success()
    );
    fs::write(root.join("notes.md"), "# Hello\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_ardent"))
        .current_dir(root)
        .arg("browse")
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires an interactive terminal"));
    assert!(!root.join(".ardent/cache").exists());
    Ok(())
}
