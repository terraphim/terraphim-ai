"""The actual hash manifest must survive Git's Windows checkout conversion."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ManifestCheckoutTest(unittest.TestCase):
    def test_manifest_bytes_survive_autocrlf_checkout(self):
        expected = (ROOT / 'media/editor-build.json').read_bytes()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.PIPE)
            git('init', '--quiet')
            git('config', 'core.autocrlf', 'true')
            (root / '.gitattributes').write_bytes((ROOT / '.gitattributes').read_bytes())
            target = root / 'media/editor-build.json'
            target.parent.mkdir()
            target.write_bytes(expected)
            git('add', '.gitattributes', 'media/editor-build.json')
            target.unlink()
            git('checkout-index', '--force', '--', 'media/editor-build.json')
            self.assertEqual(target.read_bytes(), expected)
            self.assertNotIn(b'\r\n', target.read_bytes())


if __name__ == '__main__':
    unittest.main()
