use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{self, Pid, Signal};

use rustix::termios::{self, LocalModes};

use super::{stage_release, temp_dir};
use crate::pty::{open_terminal, resize_to};

fn interact(
    project: &Path,
    rows: u16,
    input: impl FnOnce(&mut File, &Child) -> io::Result<()>,
) -> io::Result<(ExitStatus, String)> {
    let home = temp_dir()?;
    let release = stage_release()?;
    interact_with_home(project, rows, home.path(), release.executable(), input)
}

fn interact_with_home(
    project: &Path,
    rows: u16,
    home: &Path,
    binary: &Path,
    input: impl FnOnce(&mut File, &Child) -> io::Result<()>,
) -> io::Result<(ExitStatus, String)> {
    let (mut master, slave) = open_terminal(rows)?;
    let input_modes = LocalModes::ICANON | LocalModes::ISIG | LocalModes::ECHO;
    let initial_modes = termios::tcgetattr(&slave)?.local_modes & input_modes;
    let mut child = Command::new(binary)
        .arg("init")
        .env("XDG_CONFIG_HOME", home)
        .current_dir(project)
        .stdin(slave.try_clone()?)
        .stderr(slave.try_clone()?)
        .stdout(Stdio::null())
        .spawn()?;
    let mut bytes = Vec::new();
    let mut send = Some(input);
    let start = Instant::now();
    let status = loop {
        let status = child.try_wait()?;
        match master.read_to_end(&mut bytes) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(err),
        }
        if let Some(status) = status {
            break status;
        }
        if !termios::tcgetattr(&slave)?
            .local_modes
            .contains(LocalModes::ICANON)
            && let Some(send) = send.take()
        {
            send(&mut master, &child)?;
        }
        let timeout = if send.is_some()
            || !termios::tcgetattr(&slave)?
                .local_modes
                .contains(LocalModes::ICANON)
        {
            Duration::from_secs(10)
        } else {
            Duration::from_secs(120)
        };
        if start.elapsed() > timeout {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::other(format!(
                "picker timed out: {}",
                String::from_utf8_lossy(&bytes)
            )));
        }
        thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(
        termios::tcgetattr(&slave)?.local_modes & input_modes,
        initial_modes
    );
    Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
}

#[test]
fn selects_rust_in_a_short_terminal_and_after_resizing() -> io::Result<()> {
    for rows in [2, 24] {
        let project = temp_dir()?;
        let (status, output) = interact(project.path(), rows, |terminal, _| {
            resize_to(terminal, 2, 14)?;
            terminal.write_all(b"\r\x1b[B\r\x1b[B \r")
        })?;
        assert!(status.success(), "{status}: {output}");
        let packages = project.path().join(".ardent/plugins");
        assert!(packages.join("rust/format.wasm").is_file());
        assert!(!packages.join("markdown").exists());
    }
    Ok(())
}

#[test]
fn scope_picker_redraws_with_compact_labels_after_resize() -> io::Result<()> {
    let project = temp_dir()?;
    let (status, output) = interact(project.path(), 24, |term, _| {
        term.write_all(b"\r")?;
        let mut display = Vec::new();
        let start = Instant::now();
        while !String::from_utf8_lossy(&display).contains("In this project") {
            match term.read_to_end(&mut display) {
                Err(err) if err.kind() != io::ErrorKind::WouldBlock => return Err(err),
                _ => {}
            }
            if start.elapsed() > Duration::from_secs(5) {
                return Err(io::Error::other("scope picker did not appear"));
            }
            thread::sleep(Duration::from_millis(5));
        }
        resize_to(term, 2, 14)?;
        term.write_all(b"\x1b[B\r \r")
    })?;
    assert!(status.success(), "{output}");
    assert!(output.contains("Per-project"), "{output}");
    assert!(
        project
            .path()
            .join(".ardent/plugins/markdown/format.wasm")
            .is_file()
    );
    Ok(())
}

#[test]
fn reuses_a_user_package_under_a_symlinked_configuration_directory() -> io::Result<()> {
    let release = stage_release()?;
    let project = temp_dir()?;
    let home = temp_dir()?;
    let managed = tempfile::tempdir_in(release.path())?;
    fs::rename(
        release.path().join("plugins"),
        managed.path().join("plugins"),
    )?;
    std::os::unix::fs::symlink(managed.path(), home.path().join("ardent"))?;
    let before = super::snapshot(managed.path())?;
    let (status, output) = interact_with_home(
        project.path(),
        24,
        home.path(),
        release.executable(),
        |term, _| term.write_all(b"\r\r\x1b[B \r"),
    )?;
    assert!(status.success(), "{output}");
    assert!(fs::read_to_string(project.path().join(".ardent/config.yaml"))?.contains("rust: {}"));
    assert_eq!(fs::read_dir(project.path().join(".ardent"))?.count(), 2);
    assert_eq!(super::snapshot(managed.path())?, before);
    Ok(())
}

#[test]
fn interrupt_leaves_the_cursor_visible_and_creates_no_project() -> io::Result<()> {
    let project = temp_dir()?;
    let (status, output) = interact(project.path(), 24, |terminal, _| {
        terminal.write_all(b"\x03")
    })?;
    assert_eq!(status.code(), Some(1), "{status}: {output}");
    assert!(output.contains("cancelled"), "{output}");
    assert!(
        output.rfind("\x1b[?25l") <= output.rfind("\x1b[?25h"),
        "{output}"
    );
    assert!(!project.path().join(".ardent").exists());
    Ok(())
}

#[test]
fn refuses_a_parent_project_created_during_selection() -> io::Result<()> {
    let parent = temp_dir()?;
    let child = parent.path().join("child");
    fs::create_dir(&child)?;
    let (status, output) = interact(&child, 24, |terminal, _| {
        fs::create_dir(parent.path().join(".ardent"))?;
        terminal.write_all(b"n")
    })?;
    assert_eq!(status.code(), Some(1), "{status}: {output}");
    assert!(
        output.contains(&format!(
            "inside the Ardent project at {}",
            parent.path().canonicalize()?.display()
        )),
        "{output}"
    );
    assert!(!child.join(".ardent").exists());
    Ok(())
}

#[test]
fn accepts_split_arrows_in_normal_and_application_cursor_modes() -> io::Result<()> {
    for prefix in [b'[', b'O'] {
        let project = temp_dir()?;
        let (status, output) = interact(project.path(), 24, |terminal, _| {
            terminal.write_all(b"y\x1b[B\r\x1b")?;
            thread::sleep(Duration::from_millis(100));
            terminal.write_all(&[prefix])?;
            thread::sleep(Duration::from_millis(100));
            terminal.write_all(b"B \r")
        })?;
        assert!(status.success(), "{status}: {output}");
        let packages = project.path().join(".ardent/plugins");
        assert!(packages.join("rust/format.wasm").is_file());
        assert!(!packages.join("markdown").exists());
    }
    Ok(())
}

#[test]
fn refuses_a_working_directory_replaced_during_selection() -> io::Result<()> {
    let parent = temp_dir()?;
    let project = parent.path().join("checkout");
    let moved = parent.path().join("moved");
    fs::create_dir(&project)?;
    let (status, output) = interact(&project, 24, |terminal, _| {
        fs::rename(&project, &moved)?;
        fs::create_dir(&project)?;
        terminal.write_all(b"n")
    })?;
    assert!(!status.success(), "{status}: {output}");
    assert!(
        output.contains("moved during plugin selection"),
        "{status}: {output}"
    );
    assert!(!project.join(".ardent").exists());
    assert!(!moved.join(".ardent").exists());
    Ok(())
}

#[test]
fn termination_signals_restore_the_terminal() -> io::Result<()> {
    for signal in [Signal::TERM, Signal::INT, Signal::HUP] {
        let project = temp_dir()?;
        let (status, _) = interact(project.path(), 24, |_, child| {
            let pid = Pid::from_child(child);
            Ok(process::kill_process(pid, signal)?)
        })?;
        assert_eq!(status.signal(), Some(signal.as_raw()));
        assert!(!project.path().join(".ardent").exists());
    }
    Ok(())
}

#[test]
fn defaults_and_cancellation_leave_user_configuration_untouched() -> io::Result<()> {
    let release = stage_release()?;
    let home = temp_dir()?;
    for (keys, stages, done) in [
        ("n", 1, true),
        ("\r\r\r", 3, true),
        ("q", 1, false),
        ("\rq", 2, false),
        ("\r\rq", 3, false),
    ] {
        let project = temp_dir()?;
        let (status, output) = interact_with_home(
            project.path(),
            24,
            home.path(),
            release.executable(),
            |term, _| term.write_all(keys.as_bytes()),
        )?;
        assert_eq!(status.success(), done, "{output}");
        assert!(output.contains("Install format plugins? [Y/n]"));
        assert_eq!(output.contains("> For my user account"), stages >= 2);
        assert_eq!(output.contains("In this project"), stages >= 2);
        assert_eq!(output.contains("> [ ] markdown"), stages == 3);
        assert_eq!(output.contains("  [ ] rust"), stages == 3);
        let ardent = project.path().join(".ardent");
        if done {
            assert!(fs::read_to_string(ardent.join("config.yaml"))?.contains("formats: {}"));
            assert_eq!(fs::read_dir(ardent)?.count(), 2);
        } else {
            assert!(!ardent.exists());
        }
        assert_eq!(fs::read_dir(home.path())?.count(), 0);
    }
    Ok(())
}

#[test]
fn concurrent_user_installs_share_one_rust_package_and_both_projects_sync() -> io::Result<()> {
    let home = temp_dir()?;
    let release = stage_release()?;
    let projects = [temp_dir()?, temp_dir()?];
    let start = std::sync::Barrier::new(2);
    let init = |project: &Path| -> io::Result<()> {
        let (status, output) =
            interact_with_home(project, 24, home.path(), release.executable(), |term, _| {
                start.wait();
                term.write_all(b"\r\r\x1b[B \r")
            })?;
        assert!(status.success(), "{output}");
        assert!(!project.join(".ardent/plugins").exists());
        let config = fs::read_to_string(project.join(".ardent/config.yaml"))?;
        assert_eq!(
            config,
            "schema: 1\nformats:\n  rust: {}\nviews: {}\nexclude: []\n"
        );
        Ok(())
    };
    thread::scope(|scope| -> io::Result<()> {
        let first = scope.spawn(|| init(projects[0].path()));
        init(projects[1].path())?;
        first.join().expect("first init")
    })?;
    assert_eq!(fs::read_dir(home.path().join("ardent/plugins"))?.count(), 1);
    assert_eq!(
        super::snapshot(&home.path().join("ardent/plugins/rust"))?,
        super::snapshot(&release.path().join("plugins/rust"))?
    );
    for project in projects {
        fs::write(project.path().join("main.rs"), "fn main() {}\n")?;
        let sync = super::common::ardent_command(release.executable())
            .arg("sync")
            .current_dir(project.path())
            .env("XDG_CONFIG_HOME", home.path())
            .output()?;
        assert!(
            sync.status.success(),
            "{}",
            String::from_utf8_lossy(&sync.stderr)
        );
        assert!(
            project
                .path()
                .join(".ardent/cache/files/main.rs/structure.yaml")
                .is_file()
        );
    }
    Ok(())
}

#[test]
fn failed_user_install_preserves_complete_packages_and_allows_retry() -> io::Result<()> {
    let home = temp_dir()?;
    let release = stage_release()?;
    let project = temp_dir()?;
    let user = home.path().join("ardent");
    fs::create_dir(&user)?;
    fs::write(user.join("config.yaml"), "# keep this comment\nschema: 1\n")?;
    let run = || {
        interact_with_home(
            project.path(),
            24,
            home.path(),
            release.executable(),
            |term, _| term.write_all(b"y\ra\r"),
        )
    };
    let release_wasm = release.path().join("plugins/rust/format.wasm");
    let component = fs::read(&release_wasm)?;
    fs::remove_file(&release_wasm)?;
    let (status, output) = run()?;
    assert!(!status.success(), "{output}");
    assert!(
        output.contains("retained user plugins: markdown"),
        "{output}"
    );
    assert!(output.contains("format.wasm"), "{output}");
    assert!(!project.path().join(".ardent").exists());
    assert!(!user.join("plugins/rust").exists());
    let markdown = user.join("plugins/markdown");
    fs::write(markdown.join("user-notes"), "my package")?;
    let retained = super::snapshot(&markdown)?;
    fs::remove_file(release.path().join("plugins/markdown/plugin.yaml"))?;
    fs::write(release_wasm, component)?;
    // A broken user package takes precedence over the now-complete release.
    let rust = user.join("plugins/rust");
    fs::create_dir(&rust)?;
    fs::write(rust.join("plugin.yaml"), "broken")?;
    let before = super::snapshot(&user)?;
    let (status, output) = run()?;
    assert!(!status.success(), "{output}");
    assert!(output.contains("plugins/rust"), "{output}");
    assert!(!project.path().join(".ardent").exists());
    assert_eq!(super::snapshot(&user)?, before);
    fs::remove_dir_all(rust)?;
    let (status, output) = run()?;
    assert!(status.success(), "{output}");
    assert_eq!(super::snapshot(&markdown)?, retained);
    assert_eq!(
        fs::read_to_string(user.join("config.yaml"))?,
        "# keep this comment\nschema: 1\n"
    );
    assert!(!project.path().join(".ardent/plugins").exists());
    Ok(())
}
