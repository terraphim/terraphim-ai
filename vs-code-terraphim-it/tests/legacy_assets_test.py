"""Asset/source binding on real temporary files; no network or build mocks."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import legacy_assets


class LegacyAssetTest(unittest.TestCase):
    def fixture(self, root):
        for name in legacy_assets.INPUTS:
            destination = root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((legacy_assets.ROOT / name).read_bytes())
        for name in legacy_assets.ASSETS:
            destination = root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(b"\x00asm\x01\x00\x00\x00" if name.endswith(".wasm") else b"parser fixture")
        report = {"schema": 1, "rust": legacy_assets.RUST, "wasmPack": legacy_assets.WASM_PACK,
                  "target": "wasm32-unknown-unknown", "inputs": legacy_assets.inputs(root),
                  "assets": {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in legacy_assets.ASSETS}}
        (root / "media").mkdir()
        (root / legacy_assets.LOCK).write_text(json.dumps(report))
        return report

    def test_real_file_binding_rejects_source_or_asset_change(self):
        for name in legacy_assets.INPUTS + legacy_assets.ASSETS:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                expected = self.fixture(root)
                self.assertEqual(legacy_assets.verify(root), expected)
                (root / name).write_bytes((root / name).read_bytes() + b"\n")
                with self.assertRaises(ValueError):
                    legacy_assets.verify(root)

    def test_private_dependency_and_missing_build_receipt_fail(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.fixture(root)
            lock = root / "rust-lib/Cargo.lock"
            lock.write_text(lock.read_text().replace("registry+https://github.com/rust-lang/crates.io-index", "registry+https://private.invalid/index", 1))
            with self.assertRaisesRegex(ValueError, "public locked"):
                legacy_assets.inputs(root)
            (root / legacy_assets.LOCK).unlink()
            with self.assertRaises(FileNotFoundError):
                legacy_assets.verify(root)


if __name__ == "__main__":
    unittest.main()
