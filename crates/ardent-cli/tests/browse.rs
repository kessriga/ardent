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
use std::sync::LazyLock;
use std::thread;
use std::time::{Duration, Instant};

use rustix::termios;
use vt100::{Parser, Screen};

// Keep cold component compilation outside the deadlines for terminal interaction.
static BROWSER_CACHE: LazyLock<io::Result<()>> = LazyLock::new(|| {
    let project = common::temp_dir()?;
    let release = common::stage_release()?;
    for args in [&["init", "--plugins", "markdown,rust"][..], &["sync"]] {
        let output = common::ardent_command(release.executable())
            .args(args)
            .current_dir(project.path())
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
    }
    Ok(())
});

struct Session {
    child: Child,
    master: File,
    terminal: Parser,
}

impl Session {
    fn open(dir: &Path, slave: &File, master: File) -> io::Result<Self> {
        Self::open_command(
            common::ardent_command(env!("CARGO_BIN_EXE_ardent")),
            dir,
            slave,
            master,
        )
    }

    fn open_command(
        mut command: Command,
        dir: &Path,
        slave: &File,
        master: File,
    ) -> io::Result<Self> {
        BROWSER_CACHE.as_ref().map_err(io::Error::other)?;
        let mut terminal = Parser::default();
        set_terminal_size(&mut terminal, slave)?;
        Ok(Self {
            child: command
                .current_dir(dir)
                .env_remove("NO_COLOR")
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

    fn wait_source_color(
        &mut self,
        line: &str,
        column: u16,
        color: vt100::Color,
    ) -> io::Result<()> {
        self.wait_until(line, |screen| {
            screen
                .rows(0, screen.size().1.get())
                .enumerate()
                .any(|(row, text)| {
                    text.starts_with(line)
                        && screen
                            .cell(row as u16, column)
                            .is_some_and(|cell| cell.fgcolor() == color)
                })
        })?;
        Ok(())
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
fn refreshes_when_user_format_patterns_change_outside_the_project() -> io::Result<()> {
    let project = common::temp_dir()?;
    let home = common::temp_dir()?;
    let root = project.path();
    let init = common::run_init(root, &["--plugins", "markdown"])?;
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    fs::write(root.join("first.md"), "# First\n")?;
    fs::write(root.join("second.txt"), "# Second\n")?;
    let (master, slave) = pty::open_terminal(24)?;
    let mut command = common::ardent_command(env!("CARGO_BIN_EXE_ardent"));
    command.env("XDG_CONFIG_HOME", home.path());
    let mut session = Session::open_command(command, root, &slave, master)?;
    session.wait_screen(&["first.md"], &["second.txt"])?;
    let settings = home.path().join("ardent/config.yaml");
    fs::create_dir(home.path().join("ardent"))?;
    fs::write(
        &settings,
        "schema: 1\nformats: {markdown: {files: ['*.txt']}}\n",
    )?;
    session.wait_screen(&["second.txt"], &["first.md"])?;
    fs::remove_file(settings)?;
    let screen = session.wait_screen(&["first.md"], &["second.txt"])?;
    session.keys(b"q")?;
    let status = session.child.wait()?;
    assert!(status.success(), "{status}\n{screen}");
    Ok(())
}

#[test]
fn discovers_an_enabled_plugin_installed_while_browsing() -> io::Result<()> {
    let release = common::stage_release()?;
    let home = tempfile::tempdir_in(release.path())?;
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(common::run_init(root, &["--plugins", ""])?.status.success());
    fs::write(
        root.join(".ardent/config.yaml"),
        "schema: 1\nformats: {markdown: {}}\n",
    )?;
    fs::write(root.join("notes.md"), "# Notes\n")?;
    let packages = home.path().join("ardent/plugins");
    let (master, slave) = pty::open_terminal(24)?;
    let mut command = common::ardent_command(release.executable());
    command.env("XDG_CONFIG_HOME", home.path());
    let mut session = Session::open_command(command, root, &slave, master)?;
    session.wait_screen(&["no plugin"], &["notes.md"])?;
    fs::create_dir_all(&packages)?;
    fs::rename(
        release.path().join("plugins/markdown"),
        packages.join("markdown"),
    )?;
    session.wait_screen(&["notes.md"], &["no plugin"])?;
    let package = packages.join("markdown");
    for input in ["plugin.yaml", "format.wasm"] {
        let input = package.join(input);
        let original = fs::read(&input)?;
        fs::write(&input, b"unfinished")?;
        session.wait_screen(&["No tracked source files"], &["notes.md"])?;
        fs::write(input, original)?;
        session.wait_screen(&["notes.md"], &["No tracked source files"])?;
    }
    fs::rename(
        packages.join("markdown"),
        release.path().join("plugins/markdown"),
    )?;
    session.wait_screen(&["no plugin"], &["notes.md"])?;
    session.keys(b"q")?;
    assert!(session.child.wait()?.success());
    Ok(())
}

#[test]
fn notices_a_rebuilt_component_and_clears_after_external_update() -> io::Result<()> {
    let release = common::stage_release()?;
    for args in [vec![], vec!["browse"]] {
        let project = common::temp_dir()?;
        let root = project.path();
        assert!(
            common::run_init(root, &["--plugins", "markdown,rust"])?
                .status
                .success()
        );
        if !args.is_empty() {
            fs::write(
                root.join(".ardent/config.yaml"),
                "schema: 1\nformats: {rust: {}}\n",
            )?;
        }
        fs::write(root.join("notes.rs"), "fn notes() {}\n")?;
        // A custom section changes the bytes without changing the scanner or its manifest version.
        fs::OpenOptions::new()
            .append(true)
            .open(root.join(".ardent/plugins/markdown/format.wasm"))?
            .write_all(&[0, 1, 0])?;
        let synced = common::ardent_command(release.executable())
            .arg("sync")
            .current_dir(root)
            .output()?;
        assert!(
            synced.status.success(),
            "{}",
            String::from_utf8_lossy(&synced.stderr)
        );
        let (master, slave) = pty::open_terminal(16)?;
        let mut command = common::ardent_command(release.executable());
        command.args(args);
        let mut session = Session::open_command(command, root, &slave, master)?;
        session.wait_for("Plugin updates: markdown. Run ardent update")?;
        session.resize(&slave, 16, 30)?;
        session.wait_screen(&["ardent update", "q quit"], &[])?;
        session.keys(b"\r")?;
        session.wait_for("fn notes()")?;
        session.resize(&slave, 16, 80)?;
        session.wait_screen(
            &["fn notes()", "Plugin updates: markdown. Run ardent update"],
            &[],
        )?;
        let updated = common::ardent_command(release.executable())
            .arg("update")
            .current_dir(root)
            .output()?;
        assert!(
            updated.status.success(),
            "{}",
            String::from_utf8_lossy(&updated.stderr)
        );
        session.wait_screen(
            &["fn notes()", "q quit"],
            &["Plugin updates", "ardent update"],
        )?;
        session.keys(b"q")?;
        assert!(session.child.wait()?.success());
    }
    Ok(())
}

#[test]
fn shows_available_updates_alongside_a_bundle_error() -> io::Result<()> {
    let project = common::temp_dir()?;
    let release = common::stage_release()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown,rust"])?
            .status
            .success()
    );
    fs::write(release.path().join("plugins/markdown/guide"), "new")?;
    fs::remove_file(release.path().join("plugins/rust/README.md"))?;
    let (master, slave) = pty::open_terminal(24)?;
    let mut session = Session::open_command(
        common::ardent_command(release.executable()),
        root,
        &slave,
        master,
    )?;
    session.wait_screen(
        &[
            "Plugin updates: markdown. Run ardent update",
            "cannot update rust",
        ],
        &[],
    )?;
    session.keys(b"q")?;
    assert!(session.child.wait()?.success());
    Ok(())
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
    session.wait_screen(&["Explorer", "docs/", "! z.rs", "a.md"], &["skip.md"])?;
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
            "│────",
            "docs/guide/alpha.md",
            "# First passage",
            "beta.md",
        ],
        &["Source"],
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
    session.wait_screen(
        &["────", "# First passage"],
        &["Explorer", "beta.md", "Source"],
    )?;
    session.keys(b"\t\r")?;
    session.wait_screen(
        &["────", "# Second passage"],
        &["# First passage", "Source"],
    )?;
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
fn help_lists_commands_outside_a_project() -> io::Result<()> {
    let dir = common::temp_dir()?;
    let output = common::ardent_command(env!("CARGO_BIN_EXE_ardent"))
        .current_dir(dir.path())
        .arg("--help")
        .output()?;
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for command in ["init", "sync", "browse"] {
        assert!(
            help.lines()
                .any(|line| line.split_whitespace().next() == Some(command)),
            "{help}"
        );
    }
    Ok(())
}

#[test]
fn numbers_list_items_within_each_list_and_previews_the_selection() -> io::Result<()> {
    let project = common::temp_dir()?;
    let root = project.path();
    assert!(
        common::run_init(root, &["--plugins", "markdown"])?
            .status
            .success()
    );
    fs::write(
        root.join("steps.md"),
        "- First step\n  - Only child\n- Last step\n",
    )?;
    let (master, slave) = pty::open_terminal(24)?;
    let mut session = Session::open(root, &slave, master)?;
    session.wait_for("steps.md")?;
    session.keys(b"\r\x1b[C\x1b[C")?;
    session.wait_screen(&["item.1", "item.2"], &[])?;
    // Enter the first item, expand it, enter its list, expand, then select the lone child.
    session.keys(b"\x1b[C\x1b[C\x1b[C\x1b[C\x1b[C")?;
    session.wait_until(
        "select the lone child and display only its source",
        |screen| {
            let text = screen.contents();
            ["- Only child", "item.1", "item.2"]
                .iter()
                .all(|s| text.contains(s))
                && !text.contains("- First step")
                && !text.contains("- Last step")
                && text
                    .lines()
                    .any(|row| row.contains('>') && row.trim_end().ends_with(" item"))
        },
    )?;
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
    let output = common::ardent_command(env!("CARGO_BIN_EXE_ardent"))
        .current_dir(root)
        .arg("browse")
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires an interactive terminal"));
    assert!(!root.join(".ardent/cache").exists());
    Ok(())
}

#[test]
fn highlights_packaged_examples_and_navigates_a_scanner_only_file() -> io::Result<()> {
    use vt100::Color::{Default, Idx};
    let project = common::temp_dir()?;
    let root = project.path();
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let packages = root.join(".ardent/plugins");
    for name in ["markdown", "rust"] {
        let dir = packages.join(name);
        fs::create_dir_all(&dir)?;
        let example = examples.join(".ardent/plugins").join(name);
        for file in ["plugin.yaml", "format.wasm"] {
            fs::copy(example.join(file), dir.join(file))?;
        }
    }
    fs::write(
        root.join(".ardent/config.yaml"),
        "schema: 1\nformats: {markdown: {}, rust: {}, probe: {}}\n",
    )?;
    let probe = packages.join("probe");
    fs::create_dir(&probe)?;
    fs::write(
        probe.join("plugin.yaml"),
        "schema: 1\nkind: format\nname: probe\nversion: 0.1.0\ncomponent: format.wasm\nfiles: ['*.probe']\n",
    )?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/components/test-scanner.wasm"),
        probe.join("format.wasm"),
    )?;
    fs::copy(examples.join("rust/gauge.rs"), root.join("a.rs"))?;
    fs::copy(examples.join("markdown/shift-log.md"), root.join("b.md"))?;
    fs::write(root.join("c.probe"), "fn plain() {}")?;
    let (master, slave) = pty::open_terminal(32)?;
    pty::resize_to(&slave, 32, 58)?;
    let user = common::temp_dir()?;
    let user_packages = user.path().join("ardent/plugins");
    fs::create_dir_all(&user_packages)?;
    fs::rename(packages.join("rust"), user_packages.join("rust"))?;
    let mut command = common::ardent_command(env!("CARGO_BIN_EXE_ardent"));
    command.env("XDG_CONFIG_HOME", user.path());
    let mut session = Session::open_command(command, root, &slave, master)?;
    session.wait_for("c.probe")?;
    session.keys(b"\r")?;
    session.wait_source_color("//! Heights", 0, Idx(6))?;
    session.wait_source_color("pub mod gauge", 0, Idx(5))?;
    session.keys(b"\t\x1b[B\r")?;
    session.wait_source_color("# North Pier shift log", 0, Idx(3))?;
    fs::write(root.join("b.md"), "Now `code` here.\n")?;
    session.wait_source_color("Now `code` here.", 4, Idx(2))?;
    session.wait_source_color("Now `code` here.", 0, Default)?;
    session.keys(b"\t\x1b[B\r\x1b[C")?;
    session.wait_source_color("fn plain() {}", 0, Default)?;
    session.wait_until("select the scanner-only child", |screen| {
        screen
            .contents()
            .lines()
            .any(|line| line.starts_with('>') && line.trim_end().ends_with(" text"))
    })?;
    session.keys(b"q")?;
    session.wait_restored()?;
    assert!(session.child.wait()?.success());
    Ok(())
}
