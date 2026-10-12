"""The actual hash manifest must survive Git's Windows checkout conversion."""
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import legacy_assets


class ManifestCheckoutTest(unittest.TestCase):
    def test_manifest_bytes_survive_autocrlf_checkout(self):
        names = ['media/editor-build.json'] + legacy_assets.INPUTS
        expected = {name: (ROOT / name).read_bytes() for name in names}
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.PIPE)
            git('init', '--quiet')
            git('config', 'core.autocrlf', 'true')
            (root / '.gitattributes').write_bytes((ROOT / '.gitattributes').read_bytes())
            for name, data in expected.items():
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
            git('add', '.gitattributes', *names)
            for name in names:
                (root / name).unlink()
            git('checkout-index', '--force', '--', *names)
            for name, data in expected.items():
                with self.subTest(name=name):
                    self.assertEqual((root / name).read_bytes(), data)
                    self.assertNotIn(b'\r\n', (root / name).read_bytes())
            self.assertEqual(legacy_assets.inputs(root), legacy_assets.inputs(ROOT))


if __name__ == '__main__':
    unittest.main()
