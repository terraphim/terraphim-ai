"""Exercise the actual package builder and its immutable output refusal."""
import hashlib
from pathlib import Path
import tempfile
import unittest
import zipfile
from package import package, FILES


class PackageTests(unittest.TestCase):
    def test_complete_allowlisted_bundle_and_refuse_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package(root)
            archive = root / 'terraphim-continue-integration-0.1.0.zip'
            with zipfile.ZipFile(archive) as bundle:
                self.assertEqual(bundle.namelist(), ['terraphim-continue/' + name for name in FILES] + ['terraphim-continue/source.json'])
            self.assertEqual((root / 'terraphim-continue-SHA256SUMS').read_text(), hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + archive.name + '\n')
            with tempfile.TemporaryDirectory() as second:
                package(Path(second))
                self.assertEqual(archive.read_bytes(), (Path(second) / archive.name).read_bytes())
            with self.assertRaises(FileExistsError):
                package(root)
