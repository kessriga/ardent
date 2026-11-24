import hashlib
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from wasmtime import WasmtimeError

from guests.package import package


PLUGINS = Path(__file__).resolve().parents[1] / "plugins"


def file_hashes(directory):
    return {
        path.relative_to(directory): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in directory.rglob("*")
        if path.is_file()
    }


class PackageTest(unittest.TestCase):
    def test_failed_file_replacement_restores_the_release_and_allows_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            build = Path(temp) / "build"
            build.mkdir()
            release = Path(temp) / "release"
            shutil.copytree(PLUGINS, release)
            for name in ("markdown", "rust"):
                shutil.copyfile(PLUGINS / name / "format.wasm", build / f"ardent_{name}_component.wasm")
            for file in release.glob("*/*"):
                file.write_bytes(b"previous release")
            for manifest in release.glob("*/plugin.yaml"):
                manifest.unlink()
            previous = file_hashes(release)

            replace = Path.replace
            writes = 0

            def fail_fourth_replacement(source, target):
                nonlocal writes
                writes += 1
                if writes == 4:
                    raise PermissionError("cannot replace release file")
                return replace(source, target)

            with patch.object(Path, "replace", fail_fourth_replacement):
                with self.assertRaisesRegex(PermissionError, "cannot replace release file"):
                    package(build, release)
            self.assertEqual(file_hashes(release), previous)
            package(build, release)
            self.assertEqual(file_hashes(release), file_hashes(PLUGINS))

    def test_both_inputs_must_validate_before_replacing_release_files(self):
        with tempfile.TemporaryDirectory() as temp:
            build = Path(temp) / "build"
            build.mkdir()
            release = Path(temp) / "release"
            shutil.copytree(PLUGINS, release)
            (release / "markdown" / "format.wasm").write_bytes(b"old markdown")
            (release / "rust" / "README.md").write_bytes(b"old readme")
            (release / "rust" / "notes.txt").write_bytes(b"local notes")
            previous = file_hashes(release)

            shutil.copyfile(PLUGINS / "markdown" / "format.wasm", build / "ardent_markdown_component.wasm")
            fresh = Path(temp) / "new-release"
            with self.assertRaises(FileNotFoundError):
                package(build, fresh)
            self.assertFalse(fresh.exists())
            with self.assertRaises(FileNotFoundError):
                package(build, release)
            self.assertEqual(file_hashes(release), previous)

            rust = build / "ardent_rust_component.wasm"
            rust.write_bytes(b"incomplete component")
            with self.assertRaises(WasmtimeError):
                package(build, release)
            self.assertEqual(file_hashes(release), previous)

            shutil.copyfile(PLUGINS / "rust" / "format.wasm", rust)
            package(build, release)
            expected = file_hashes(PLUGINS)
            package(build, fresh)
            self.assertEqual(file_hashes(fresh), expected)
            notes = Path("rust/notes.txt")
            expected[notes] = previous[notes]
            self.assertEqual(file_hashes(release), expected)
