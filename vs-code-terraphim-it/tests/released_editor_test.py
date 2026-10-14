"""Real archive/file tests for the public-release import boundary."""
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
import copy

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from released_editor import checked, materialize


class ReleasedEditorTest(unittest.TestCase):
    def source(self):
        return {"sourceCommit": "a" * 40, "release": "release", "archive": {"sha256": "b" * 64}}

    def archive(self, entries):
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w:gz") as archive:
            for name, data, kind in entries:
                entry = tarfile.TarInfo(name)
                entry.type = kind
                entry.size = len(data)
                archive.addfile(entry, io.BytesIO(data))
        return stream.getvalue()

    def run_import(self, entries, assets, metadata=None, source=None):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "assets"
            manifest = {"schema": 1, "sourceHead": "a" * 40, "assets": assets}
            manifest.update(metadata or {})
            materialize(self.archive(entries), json.dumps(manifest), b"bridge", target,
                        source or self.source())
            return {p.relative_to(target).as_posix(): p.read_bytes()
                    for p in target.rglob("*") if p.is_file()}

    def test_real_archive_preserves_exact_bytes(self):
        data = b"actual fixture bytes"
        files = self.run_import([("vendor/a.js", data, tarfile.REGTYPE)],
                                {"vendor/a.js": hashlib.sha256(data).hexdigest()})
        self.assertEqual(files["vendor/a.js"], data)
        self.assertEqual(files["host-session.js"], b"bridge")
        lock = json.loads(files["host-assets.json"])
        self.assertEqual(lock["sourceHead"], "a" * 40)
        self.assertNotIn("cargoLockSha256", lock)

    def test_public_dependency_provenance_is_bound_and_retained(self):
        metadata = {"sourceRepository": "https://github.com/terraphim/terraphim-editor",
                    "cargoLockSha256": "c" * 64}
        source = {**self.source(), **metadata}
        files = self.run_import([("a.js", b"a", tarfile.REGTYPE)],
                                {"a.js": hashlib.sha256(b"a").hexdigest()}, metadata, source)
        lock = json.loads(files["host-assets.json"])
        self.assertEqual({key: lock[key] for key in metadata}, metadata)

    def test_manifest_identity_and_public_provenance_refused_before_write(self):
        good = {"schema": 1, "sourceHead": "a" * 40,
                "sourceRepository": "https://github.com/terraphim/terraphim-editor",
                "cargoLockSha256": "c" * 64,
                "assets": {"a.js": hashlib.sha256(b"a").hexdigest()}}
        source = {**self.source(), "sourceRepository": good["sourceRepository"],
                  "cargoLockSha256": good["cargoLockSha256"]}
        invalid = [None, [], {}, {**good, "schema": 2}, {**good, "sourceHead": "d" * 40},
                   {**good, "sourceRepository": "https://example.invalid/editor"},
                   {**good, "cargoLockSha256": "e" * 64}, {**good, "cargoLockSha256": None},
                   {**good, "assets": []}, {**good, "assets": {}},
                   {**good, "assets": {"a.js": "not-a-sha256"}}]
        for key in ("sourceRepository", "cargoLockSha256"):
            missing = copy.deepcopy(good); del missing[key]; invalid.append(missing)
        archive = self.archive([("a.js", b"a", tarfile.REGTYPE)])
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "assets"
            for manifest in invalid:
                with self.subTest(manifest=manifest), self.assertRaises(ValueError):
                    materialize(archive, json.dumps(manifest), b"bridge", target, source)
                self.assertFalse(target.exists(), "Refused metadata must not write any asset")

    def test_public_metadata_requires_explicit_matching_pin(self):
        metadata = {"sourceRepository": "https://github.com/terraphim/terraphim-editor",
                    "cargoLockSha256": "c" * 64}
        with self.assertRaises(ValueError):
            self.run_import([("a.js", b"a", tarfile.REGTYPE)],
                            {"a.js": hashlib.sha256(b"a").hexdigest()}, metadata)

    def test_wrong_digest_fails(self):
        with self.assertRaises(ValueError):
            checked(b"changed", hashlib.sha256(b"expected").hexdigest())

    def test_unsafe_duplicate_and_link_entries_fail(self):
        digest = hashlib.sha256(b"a").hexdigest()
        for name, kind in [("../escape", tarfile.REGTYPE), ("/absolute", tarfile.REGTYPE),
                           ("a\\b", tarfile.REGTYPE), ("link", tarfile.SYMTYPE)]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.run_import([(name, b"a", kind)], {name: digest})
        with self.assertRaises(ValueError):
            self.run_import([("a", b"a", tarfile.REGTYPE)] * 2, {"a": digest})

    def test_missing_extra_and_modified_files_fail(self):
        digest = hashlib.sha256(b"a").hexdigest()
        for entries, assets in [([], {"a": digest}), ([("a", b"a", tarfile.REGTYPE)], {}),
                                ([("a", b"b", tarfile.REGTYPE)], {"a": digest})]:
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                self.run_import(entries, assets)


if __name__ == "__main__":
    unittest.main()
