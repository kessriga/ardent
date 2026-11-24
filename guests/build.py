#!/usr/bin/env python3
"""Build the release format components and the integration-test scanner."""

import os
from pathlib import Path
import shlex
import subprocess


def build():
    root = Path(__file__).resolve().parents[1]
    if "WASI_SDK_PATH" not in os.environ:
        raise SystemExit("Set WASI_SDK_PATH to the unpacked WASI SDK 34 directory.")
    sdk = Path(os.environ["WASI_SDK_PATH"]).resolve()
    executable = ".exe" if os.name == "nt" else ""
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.96.0", CARGO_HOME=str(cargo_home))
    sysroot = subprocess.check_output(["rustc", "--print", "sysroot"], env=env, text=True).strip()
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join([
        f"--remap-path-prefix={root}=/ardent",
        f"--remap-path-prefix={cargo_home}=/cargo",
        f"--remap-path-prefix={sysroot}=/rust",
    ])
    env["CC_wasm32_wasip2"] = str(sdk / "bin" / ("clang" + executable))
    env["AR_wasm32_wasip2"] = str(sdk / "bin" / ("llvm-ar" + executable))
    env["CC_SHELL_ESCAPED_FLAGS"] = "1"
    env["CFLAGS_wasm32_wasip2"] = shlex.join([
        f"-ffile-prefix-map={cargo_home}=/cargo",
        f"-ffile-prefix-map={sdk}=/wasi-sdk",
    ])
    subprocess.run([
        "cargo", "build", "--workspace", "--locked", "--release", "--target", "wasm32-wasip2",
        "--manifest-path", "guests/Cargo.toml", "--target-dir", "target/guests",
    ], cwd=root, env=env, check=True)


if __name__ == "__main__":
    build()
