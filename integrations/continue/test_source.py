"""Exact remote-tag identity parsing (no network or protocol mocks)."""
import unittest
from verify_source import peeled_tag


class SourceTests(unittest.TestCase):
    def test_lightweight_and_annotated_tag_identity(self):
        self.assertEqual(peeled_tag('aaa refs/tags/terraphim_continue-v0.1.0\n', 'terraphim_continue-v0.1.0'), 'aaa')
        self.assertEqual(peeled_tag('aaa refs/tags/x\nbbb refs/tags/x^{}\n', 'x'), 'bbb')
        with self.assertRaises(ValueError):
            peeled_tag('aaa refs/tags/other\n', 'x')

    def test_actual_git_tag_gate_and_moved_tag_refusal(self):
        from pathlib import Path
        import json
        import shutil
        import subprocess
        import sys
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo, origin = root / 'repo', root / 'origin.git'
            here = Path(__file__).resolve().parent
            shutil.copytree(here, repo / 'integrations/continue', ignore=shutil.ignore_patterns('__pycache__', 'proof'))
            def git(*args, cwd=repo):
                return subprocess.check_output(['git', '-C', str(cwd), *args], stderr=subprocess.DEVNULL, text=True).strip()
            git('init', '--initial-branch=main')
            git('config', 'user.name', 'Public test')
            git('config', 'user.email', 'test@example.invalid')
            git('add', '.')
            git('commit', '-m', 'Public integration fixture')
            head = git('rev-parse', 'HEAD')
            manifest = json.loads((here / 'integration.json').read_text())
            tag = manifest['tagPrefix'] + manifest['version']
            archive_name = f"terraphim-continue-integration-{manifest['version']}.zip"
            git('tag', tag)
            git('clone', '--bare', str(repo), str(origin))
            git('remote', 'add', 'origin', str(origin))
            build = root / 'build'
            subprocess.check_call([sys.executable, str(repo / 'integrations/continue/package.py'), str(build)], stdout=subprocess.DEVNULL)
            command = [sys.executable, str(repo / 'integrations/continue/verify_source.py'), '--tag', tag, '--qualified', head, '--archive', str(build / archive_name)]
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 0)
            (repo / 'new.md').write_text('New public source revision\n')
            git('add', 'new.md')
            git('commit', '-m', 'Advance public source')
            changed = git('rev-parse', 'HEAD')
            # Modify only this test's bare origin; do not push or rewrite a real repository.
            git('fetch', str(repo), 'HEAD:refs/heads/unreviewed', cwd=origin)
            git('update-ref', 'refs/tags/' + tag, changed, cwd=origin)
            unreviewed_build = root / 'unreviewed-build'
            subprocess.check_call([sys.executable, str(repo / 'integrations/continue/package.py'), str(unreviewed_build)], stdout=subprocess.DEVNULL)
            unreviewed = [sys.executable, str(repo / 'integrations/continue/verify_source.py'), '--tag', tag, '--qualified', changed, '--archive', str(unreviewed_build / archive_name)]
            self.assertNotEqual(subprocess.run(unreviewed, capture_output=True).returncode, 0)
            git('checkout', '--detach', head)
            refused = subprocess.run(command, capture_output=True)
            self.assertNotEqual(refused.returncode, 0)
            self.assertIn(b'Live remote tag moved', refused.stderr)
