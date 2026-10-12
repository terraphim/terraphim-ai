"""Real filesystem importer checks; no mocked IO or substituted engine."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("editor_assets", Path(__file__).resolve().parents[1] / "scripts/editor_assets.py")
assets = importlib.util.module_from_spec(spec)
spec.loader.exec_module(assets)

class AssetImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.mkdir()
        (self.source / "engine.wasm").write_bytes(b"\x00asm\x01\x00\x00\x00")
        self.manifest = {"protocol":1,"assets":{"engine.wasm":hashlib.sha256((self.source/"engine.wasm").read_bytes()).hexdigest()}}
        self.save_manifest()
    def save_manifest(self):
        (self.source / "host-assets.json").write_text(json.dumps(self.manifest))
    def test_import_and_locked_verification(self):
        target, lock = self.root/"target", self.root/"lock.json"
        self.assertEqual(assets.import_assets(self.source,target,lock),1)
        assets.verify(target,lock)
        self.assertEqual(assets.import_assets(self.source,target,lock),1)
    def test_corruption_is_refused(self):
        (self.source/"engine.wasm").write_bytes(b"corrupted")
        with self.assertRaises(ValueError): assets.verify(self.source)
    def test_extra_files_are_refused(self):
        (self.source/"unlisted.js").write_text("unexpected")
        with self.assertRaises(ValueError): assets.verify(self.source)
    def test_path_escape_is_refused(self):
        self.manifest["assets"]={"../outside":"irrelevant"};self.save_manifest()
        with self.assertRaises(ValueError): assets.verify(self.source)
    def test_symlink_is_refused(self):
        (self.root/"outside").write_text("outside")
        (self.source/"engine.wasm").unlink();(self.source/"engine.wasm").symlink_to(self.root/"outside")
        with self.assertRaises(ValueError): assets.verify(self.source)
    def test_different_existing_destination_is_preserved(self):
        target,lock=self.root/"target",self.root/"lock.json"
        assets.import_assets(self.source,target,lock)
        self.manifest["sourceHead"]="different";self.save_manifest()
        with self.assertRaises(ValueError):assets.import_assets(self.source,target,lock)
        self.assertNotEqual((target/"host-assets.json").read_bytes(),(self.source/"host-assets.json").read_bytes())

if __name__ == "__main__": unittest.main()
