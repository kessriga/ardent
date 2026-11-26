"""Exercise the install and format recipes in a disposable checkout."""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]


class RecipeTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        shutil.copy(REPO / "Justfile", self.root)

    def just(self, *args):
        env = os.environ.copy()
        env.pop("CARGO_INSTALL_ROOT", None)
        subprocess.run(["just", *args], cwd=self.root, env=env, check=True)

    def test_install_refreshes_only_the_bundles_at_cargos_destination(self):
        # A dependency-free binary exercises Cargo's real config lookup and reinstall status.
        crate = self.root / "crates/ardent-cli"
        (crate / "src").mkdir(parents=True)
        (crate / "Cargo.toml").write_text('[package]\nname = "ardent"\nversion = "0.1.0"\nedition = "2024"\n')
        (crate / "src/main.rs").write_text("fn main() {}\n")
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=crate, check=True)
        (self.root / ".cargo").mkdir()
        install_root = self.root / "local tools"
        (self.root / ".cargo/config.toml").write_text(f"[install]\nroot = {json.dumps(str(install_root))}\n")
        (self.root / "scripts").mkdir()
        shutil.copy(REPO / "scripts/install.py", self.root / "scripts")
        for name in ("markdown", "rust"):
            package = self.root / "plugins" / name
            package.mkdir(parents=True)
            (package / "plugin.yaml").write_text(name)

        self.just("install")
        packages = install_root / "bin/plugins"
        (packages / "third-party").mkdir()
        (packages / "rust/retired.txt").touch()
        self.just("install")
        self.assertEqual([p.name for p in (packages / "rust").iterdir()], ["plugin.yaml"])
        self.assertTrue((packages / "third-party").is_dir())
        self.just("install", "override")
        self.assertEqual((self.root / "override/bin/plugins/markdown/plugin.yaml").read_text(), "markdown")

    def test_fmt_md_preserves_snapshot_inputs(self):
        path = Path("crates/ardent-format/tests/markdown/guide.md")
        fixture = self.root / path
        fixture.parent.mkdir(parents=True)
        original = (REPO / path).read_bytes()
        fixture.write_bytes(original)
        ordinary = self.root / "guide.md"
        ordinary.write_bytes(original)

        self.just("fmt-md", ".")
        self.assertEqual(fixture.read_bytes(), original)
        self.just("fmt-md", str(path))
        self.assertEqual(fixture.read_bytes(), original)
        self.assertNotEqual(ordinary.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
