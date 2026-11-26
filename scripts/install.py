"""Install Ardent with Cargo and refresh the bundled packages next to its executable."""

import os
import shutil
import subprocess
import sys
import tempfile
from contextlib import ExitStack
from pathlib import Path


def install_binary(root=None):
    command = [
        "cargo", "install", "--locked", "--path", "crates/ardent-cli", "--force",
        "--color", "never", "--config", "term.quiet=false",
    ]
    if root:
        command.extend(["--root", root])

    # Cargo's JSON artifact names the build output. Its install status names the final executable.
    executable = None
    with subprocess.Popen(command, stderr=subprocess.PIPE) as cargo:
        for line in cargo.stderr:
            sys.stderr.buffer.write(line)
            sys.stderr.buffer.flush()
            status, _, detail = os.fsdecode(line).lstrip().rstrip("\r\n").partition(" ")
            if status in {"Installing", "Replacing"}:
                path = Path(detail)
                if path.name in {"ardent", "ardent.exe"} and path.parent.name == "bin":
                    executable = path
        if cargo.wait():
            raise SystemExit(cargo.returncode)
    if executable is None or not executable.is_file():
        raise SystemExit("Cargo did not report the installed ardent executable")
    return executable


def install_plugins(source, destination):
    destination.mkdir(exist_ok=True)
    # Stage complete packages on the destination filesystem before moving the old copies aside.
    with tempfile.TemporaryDirectory(dir=destination) as temp, ExitStack() as rollback:
        stage = Path(temp)
        names = ("markdown", "rust")
        for name in names:
            shutil.copytree(source / name, stage / name)
        for name in names:
            target = destination / name
            backup = stage / (name + ".old")
            if target.exists() or target.is_symlink():
                target.rename(backup)
                rollback.callback(backup.rename, target)
            (stage / name).rename(target)
            rollback.callback(target.rename, stage / name)
        rollback.pop_all()


if __name__ == "__main__":
    binary = install_binary(*sys.argv[1:])
    install_plugins(Path("plugins"), binary.parent / "plugins")
