"""Credential boundary and metadata validators; no remote-success simulation."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
from tests.lsp_handoff_test import request

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('private_producer', ROOT/'.github/scripts/release/lsp_private_producer.py')
p = importlib.util.module_from_spec(spec); spec.loader.exec_module(p)

class BoundaryTest(unittest.TestCase):
    def test_dedicated_binding_required_no_broad_token_fallback(self):
        with self.assertRaisesRegex(ValueError, 'LSP_SOURCE_CREDENTIAL_MISSING'):
            p.credentials({'GITEA_TOKEN': 'broad-fixture', 'OP_SERVICE_ACCOUNT_TOKEN': 'broad-fixture',
                           'CARGO_REGISTRIES_TERRAPHIM_TOKEN': 'Bearer fixture'})
        with self.assertRaisesRegex(ValueError, 'LSP_REGISTRY_CREDENTIAL_MISSING'):
            p.credentials({'GITEA_LSP_SOURCE_TOKEN': 'scoped-fixture'})
        env = p.credentials({'GITEA_LSP_SOURCE_TOKEN': 'scoped-fixture', 'CARGO_REGISTRIES_TERRAPHIM_TOKEN': 'Bearer fixture',
                             'GH_TOKEN': 'not-allowed', 'GITEA_TOKEN': 'not-allowed', 'OP_SERVICE_ACCOUNT_TOKEN': 'not-allowed',
                             'GIT_TRACE': 'not-allowed', 'HOME': '/fixture'})
        self.assertEqual(set(env), {'GITEA_LSP_SOURCE_TOKEN', 'CARGO_REGISTRIES_TERRAPHIM_TOKEN', 'HOME'})

    def test_askpass_exact_host_repository_and_prompt(self):
        canary = 'fixture-scoped-canary'
        self.assertEqual(p.askpass_response("Username for 'https://git.terraphim.cloud/terraphim/terraphim-ai.git': ", canary), 'token')
        self.assertEqual(p.askpass_response("Password for 'https://token@git.terraphim.cloud/terraphim/terraphim-editor.git': ", canary), canary)
        for url in ('https://git.terraphim.cloud.attacker.invalid/terraphim/terraphim-ai.git',
                    'https://attacker.invalid/https://git.terraphim.cloud',
                    'https://git.terraphim.cloud@attacker.invalid/terraphim/terraphim-ai.git',
                    'https://evil@git.terraphim.cloud/terraphim/terraphim-ai.git',
                    'http://git.terraphim.cloud/terraphim/terraphim-ai.git',
                    'https://git.terraphim.cloud/terraphim/other.git',
                    'https://git.terraphim.cloud:443/terraphim/terraphim-ai.git',
                    'https://git.terraphim.cloud/terraphim/terraphim-ai.git?secret=1'):
            with self.subTest(url=url), self.assertRaises(ValueError):
                p.askpass_response("Password for '" + url + "': ", canary)

    def test_metadata_source_tag_and_ci_required(self):
        req = request(); repo = {'full_name': p.h.REPO, 'default_branch': 'main', 'private': True, 'archived': False}
        editor = {'full_name': 'terraphim/terraphim-editor', 'archived': False}
        tag = {'name': req['tag'], 'commit': {'sha': req['source']['commit']}}
        status = {'sha': req['source']['commit'], 'state': 'success', 'statuses': [
            {'context': name, 'status': 'success', 'target_url': '/terraphim/terraphim-ai/actions/runs/1/jobs/2'} for name in p.h.SOURCE_CHECKS]}
        result = p.verify_source_metadata(req, repo, tag, status, editor)
        self.assertNotIn('statuses', result); self.assertEqual(len(result['metadataSha256']), 64)
        for value in ('0'*40, None):
            bad = copy.deepcopy(tag); bad['commit']['sha'] = value
            with self.assertRaises(ValueError): p.verify_source_metadata(req, repo, bad, status, editor)
        for value in ('pending', 'failure'):
            bad = copy.deepcopy(status); bad['statuses'][0]['status'] = value
            with self.assertRaises(ValueError): p.verify_source_metadata(req, repo, tag, bad, editor)
        bad = copy.deepcopy(status); bad['statuses'][0]['target_url'] = ''
        with self.assertRaises(ValueError): p.verify_source_metadata(req, repo, tag, bad, editor)

    def test_source_metadata_malformed_shapes_fail_validation(self):
        req = request()
        repo = {'full_name': p.h.REPO, 'default_branch': 'main', 'private': True, 'archived': False}
        editor = {'full_name': 'terraphim/terraphim-editor', 'archived': False}
        tag = {'name': req['tag'], 'commit': {'sha': req['source']['commit']}}
        status = {'sha': req['source']['commit'], 'state': 'success', 'statuses': []}
        for malformed in (None, [], 'wrong', 4):
            for index in range(4):
                args = [repo, tag, status, editor]; args[index] = malformed
                with self.subTest(index=index, value=malformed), self.assertRaises(ValueError): p.verify_source_metadata(req, *args)
            bad = copy.deepcopy(tag); bad['commit'] = malformed
            with self.assertRaises(ValueError): p.verify_source_metadata(req, repo, bad, status, editor)
            bad = copy.deepcopy(status); bad['statuses'] = malformed
            with self.assertRaises(ValueError): p.verify_source_metadata(req, repo, tag, bad, editor)
            bad = copy.deepcopy(status); bad['statuses'] = [malformed]
            with self.assertRaises(ValueError): p.verify_source_metadata(req, repo, tag, bad, editor)

    def test_real_private_directory_permissions_and_symlink_refusal(self):
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary); lane = p.protected_directory(home)
            self.assertEqual(lane.stat().st_mode & 0o077, 0)
            self.assertNotIn('runner_temp', str(lane))
            lane.parent.chmod(0o755)
            with self.assertRaises(ValueError): p.protected_directory(home)
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary); (home/'target').mkdir(); (home/'.local').symlink_to(home/'target')
            with self.assertRaises(ValueError): p.protected_directory(home)

if __name__ == '__main__': unittest.main()
