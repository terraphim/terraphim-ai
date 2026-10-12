"""Real archive/file tests for the public-release import boundary."""
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from released_editor import checked, materialize


class ReleasedEditorTest(unittest.TestCase):
    def archive(self, entries):
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w:gz") as archive:
            for name, data, kind in entries:
                entry = tarfile.TarInfo(name)
                entry.type = kind
                entry.size = len(data)
                archive.addfile(entry, io.BytesIO(data))
        return stream.getvalue()

    def run_import(self, entries, assets):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "assets"
            materialize(self.archive(entries), json.dumps({"assets": assets}), b"bridge", target,
                        {"sourceCommit": "commit", "release": "release", "archive": {"sha256": "hash"}})
            return {p.relative_to(target).as_posix(): p.read_bytes()
                    for p in target.rglob("*") if p.is_file()}

    def test_real_archive_preserves_exact_bytes(self):
        data = b"actual fixture bytes"
        files = self.run_import([("vendor/a.js", data, tarfile.REGTYPE)],
                                {"vendor/a.js": hashlib.sha256(data).hexdigest()})
        self.assertEqual(files["vendor/a.js"], data)
        self.assertEqual(files["host-session.js"], b"bridge")

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
