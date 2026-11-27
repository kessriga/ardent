use std::fs::File;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;

// Keep shared-directory users from renaming a staging ancestor during verification and publication.
pub(super) fn pin_directory(path: &Path) -> io::Result<File> {
    const FILE_SHARE_READ_WRITE: u32 = 0x3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x02000000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x00200000;
    let file = File::options()
        .read(true)
        .share_mode(FILE_SHARE_READ_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    if !file.metadata()?.is_dir() {
        return Err(io::Error::other(format!(
            "{} must be a directory",
            path.display()
        )));
    }
    Ok(file)
}

fn permissions(action: &str, path: &Path, previous: Option<&Path>) -> io::Result<()> {
    let system =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("missing SystemRoot"))?;
    let shell = Path::new(&system).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut cmd = Command::new(shell);
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        include_str!("permissions.ps1"),
    ])
    .env("ARDENT_ACL_ACTION", action)
    .env("ARDENT_ACL_PATH", path)
    .env_remove("ARDENT_PREVIOUS_ACL");
    if let Some(previous) = previous {
        cmd.env("ARDENT_PREVIOUS_ACL", previous);
    }
    let output = cmd.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

pub(super) fn private_directory(path: &Path) -> io::Result<File> {
    permissions("create", path, None)?;
    let pin = pin_directory(path)?;
    permissions("verify", path, None)?;
    Ok(pin)
}

pub(super) fn restore_access(installed: &Path, previous: &Path) -> io::Result<()> {
    permissions("restore", installed, Some(previous))
}
