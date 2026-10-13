"""Exercise the actual installer against owned project directories."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from server import digest


class InstallTests(unittest.TestCase):
    def run_install(self, project):
        here = Path(__file__).resolve().parent
        return subprocess.run([sys.executable, str(here / 'install.py'), '--project', str(project), '--binary', os.environ['TERRAPHIM_GREP_BINARY'], '--binary-sha256', digest(Path(os.environ['TERRAPHIM_GREP_BINARY'])), '--graph', str(here / 'risk-concern.exported.json'), '--corpus', str(project)], capture_output=True)

    def test_project_install_preserves_existing_configuration(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.continue').mkdir()
            model = root / '.continue/config.yaml'
            model.write_text('name: Existing user model\n')
            self.assertEqual(self.run_install(root).returncode, 0)
            config = root / '.continue/mcpServers/terraphim.json'
            original = config.read_bytes()
            value = json.loads(original)['mcpServers']['Terraphim']
            self.assertEqual(value['command'], sys.executable)
            self.assertNotIn('--allow-synthesis', value['args'])
            self.assertNotEqual(self.run_install(root).returncode, 0)
            self.assertEqual(config.read_bytes(), original)
            self.assertEqual(model.read_text(), 'name: Existing user model\n')

    def test_symlink_directory_refused(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            root = Path(directory)
            (root / '.continue').symlink_to(other)
            self.assertNotEqual(self.run_install(root).returncode, 0)
            self.assertEqual(list(Path(other).iterdir()), [])


if __name__ == '__main__':
    unittest.main()
