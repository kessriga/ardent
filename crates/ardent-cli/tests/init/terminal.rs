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
use crate::pty::{open_terminal, resize};

fn interact(
    project: &Path,
    rows: u16,
    input: impl FnOnce(&mut File, &Child) -> io::Result<()>,
) -> io::Result<(ExitStatus, String)> {
    let release = stage_release()?;
    let (mut master, slave) = open_terminal(rows)?;
    let input_modes = LocalModes::ICANON | LocalModes::ISIG | LocalModes::ECHO;
    let initial_modes = termios::tcgetattr(&slave)?.local_modes & input_modes;
    let mut child = Command::new(release.executable())
        .arg("init")
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
        if start.elapsed() > Duration::from_secs(10) {
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
            resize(terminal, 2)?;
            terminal.write_all(b"\x1b[B \r")
        })?;
        assert!(status.success(), "{status}: {output}");
        let packages = project.path().join(".ardent/plugins");
        assert!(packages.join("rust/format.wasm").is_file());
        assert!(!packages.join("markdown").exists());
    }
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
        terminal.write_all(b"\r")
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
            terminal.write_all(b"\x1b")?;
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
        terminal.write_all(b"\r")
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
