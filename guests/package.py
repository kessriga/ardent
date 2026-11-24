"""Package the two release components from a wasm32-wasip2 build directory."""

import argparse
import json
import shutil
import tempfile
from contextlib import ExitStack
from functools import partial
from pathlib import Path

from wasmtime import Engine, Store, WasiConfig
from wasmtime.component import Component, Linker


def describe(engine, linker, component_bytes):
    store = Store(engine)
    store.set_wasi(WasiConfig())
    component = Component(engine, component_bytes)
    instance = linker.instantiate(store, component)
    interface = instance.get_export_index(store, "ardent:format/scanner@0.1.0")
    if interface is None:
        raise ValueError("component has no ardent:format/scanner@0.1.0 export")
    index = instance.get_export_index(store, "describe", interface)
    if index is None or (function := instance.get_func(store, index)) is None:
        raise ValueError("scanner has no describe function")
    metadata = function(store)
    function.post_return(store)
    return metadata


def manifest(metadata):
    lines = [
        "schema: 1",
        f"name: {json.dumps(metadata.name)}",
        f"version: {json.dumps(metadata.version)}",
        "kind: format",
        "component: format.wasm",
    ]
    if metadata.files:
        lines.append("files:")
        lines.extend(f"  - {json.dumps(pattern)}" for pattern in metadata.files)
    if metadata.prefixes:
        lines.append("prefixes:")
        for prefix in metadata.prefixes:
            key = json.dumps(prefix.name)
            lines.append(f"  {key}:" if prefix.files else f"  {key}: []")
            lines.extend(f"    - {json.dumps(pattern)}" for pattern in prefix.files)
    return "\n".join(lines) + "\n"


def stage_packages(build_directory, staging):
    engine = Engine()
    linker = Linker(engine)
    linker.add_wasip2()
    for name in ("markdown", "rust"):
        component_bytes = (build_directory / f"ardent_{name}_component.wasm").read_bytes()
        metadata = describe(engine, linker, component_bytes)
        if metadata.name != name:
            raise ValueError(f"{name} component declared its name as {metadata.name!r}")
        directory = staging / name
        directory.mkdir(parents=True, exist_ok=True)
        if name == "rust":
            readme = Path(__file__).resolve().parent / "rust" / "README.md"
            shutil.copyfile(readme, directory / "README.md")
        (directory / "format.wasm").write_bytes(component_bytes)
        (directory / "plugin.yaml").write_text(manifest(metadata), encoding="utf-8")


def install(staging, destination, backups):
    with ExitStack() as rollback:
        for directory in staging.iterdir():
            installed = destination / directory.name
            if not installed.exists():
                installed.mkdir()
                rollback.callback(installed.rmdir)
            for source in directory.iterdir():
                target = installed / source.name
                if target.exists() or target.is_symlink():
                    backup = backups / f"{directory.name}-{source.name}"
                    shutil.copy2(target, backup, follow_symlinks=False)
                    undo = partial(backup.replace, target)
                else:
                    undo = target.unlink
                source.replace(target)
                rollback.callback(undo)
        rollback.pop_all()


def package(build_directory, destination):
    new_release = not destination.exists()
    parent = destination.parent if new_release else destination
    parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=parent) as temporary:
        staging = Path(temporary) / "packages"
        stage_packages(build_directory, staging)
        if new_release:
            staging.rename(destination)
        else:
            install(staging, destination, Path(temporary))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("build_directory", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    package(args.build_directory, args.destination)
