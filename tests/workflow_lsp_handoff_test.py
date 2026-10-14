"""Workflow/schema boundaries; real release execution remains a separate gate."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
import yaml
import jsonschema
from tests.lsp_handoff_test import h, request, producer, lane
from tests.workflow_release_recovery_contract_test import GITHUB_RELEASE_WRITER_PATTERN

ROOT = Path(__file__).resolve().parents[1]

class WorkflowTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.producer = yaml.safe_load((ROOT/'.github/workflows/lsp-producer.yml').read_text())
        cls.publisher = yaml.safe_load((ROOT/'.github/workflows/lsp-publish.yml').read_text())

    def test_producer_no_pr_event_no_release_writer_no_broad_credentials(self):
        value = self.producer
        self.assertEqual(set(value.get('on', value.get(True))), {'workflow_dispatch'})
        self.assertEqual(value['permissions'], {'contents': 'read'})
        text = (ROOT/'.github/workflows/lsp-producer.yml').read_text()
        self.assertFalse(GITHUB_RELEASE_WRITER_PATTERN.search(text))
        for forbidden in ('pull_request_target', 'OP_SERVICE_ACCOUNT_TOKEN', 'secrets.GITEA_TOKEN', 'contents: write'):
            self.assertNotIn(forbidden, text)
        self.assertIn("github.ref == 'refs/heads/main'", value['jobs']['resolve']['if'])
        for name in ('qualify', 'linux', 'macos'):
            job = value['jobs'][name]; self.assertEqual(job['environment'], 'lsp-producer')
            self.assertEqual(job['runs-on']['group'], 'lsp-private')
            self.assertIn('self-hosted', job['runs-on']['labels'])
            self.assertIn('resolve', job['needs'] if isinstance(job['needs'], list) else [job['needs']])
            if name != 'qualify': self.assertIn('qualify', job['needs'])
            private_steps = [s for s in job['steps'] if 'GITEA_LSP_SOURCE_TOKEN' in s.get('env', {})]
            self.assertEqual(len(private_steps), 1)
            self.assertIn('lsp_private_producer.py', private_steps[0]['run'])
            for step in job['steps']:
                if 'upload-artifact@' in step.get('uses', ''):
                    self.assertIn(step['with']['path'], ('public-lane/*', 'producer-receipt.json'))

    def test_public_registry_publisher_is_called_only_after_crate_publication(self):
        value = self.publisher
        self.assertEqual(set(value.get('on', value.get(True))), {'workflow_call'})
        self.assertEqual(set(value.get('on', value.get(True))['workflow_call']['inputs']), {'tag', 'source-sha', 'toolchain'})
        job = value['jobs']['publish']; self.assertEqual(job['runs-on'], 'ubuntu-latest')
        self.assertEqual(job['environment'], 'lsp-releases')
        self.assertEqual(job['permissions'], {'contents': 'write'})
        self.assertEqual(job['needs'], ['qualify', 'linux', 'macos'])
        text = (ROOT/'.github/workflows/lsp-publish.yml').read_text()
        for forbidden in ('secrets.', 'GITEA_', 'OP_SERVICE', 'lsp_handoff.py publish'):
            self.assertNotIn(forbidden, text)
        self.assertIn('lsp_public_binary.py publish', text)
        self.assertTrue(GITHUB_RELEASE_WRITER_PATTERN.search(text))
        self.assertIn("github.sha == inputs.source-sha", value['jobs']['qualify']['if'])

    def test_exact_checkouts_and_pinned_actions(self):
        for value in (self.producer, self.publisher):
            for job in value['jobs'].values():
                for step in job['steps']:
                    if 'uses' in step:
                        self.assertRegex(step['uses'], r'^[\w/-]+@[a-f0-9]{40}$')
                    if step.get('uses', '').startswith('actions/checkout@'):
                        self.assertEqual(step['with']['ref'], '${{ github.sha }}')
                        self.assertEqual(step['with']['fetch-depth'], 0)
                        self.assertIs(step['with']['persist-credentials'], False)

    def test_schemas_valid_and_runtime_examples_agree(self):
        definitions = {}
        for kind in ('source-request', 'lane-receipt', 'release-handoff'):
            schema = json.loads((ROOT/f'docs/contracts/schemas/lsp-{kind}.schema.json').read_text())
            jsonschema.Draft202012Validator.check_schema(schema)
            definitions[kind] = jsonschema.Draft202012Validator(schema)
        definitions['source-request'].validate(request())
        definitions['lane-receipt'].validate({'schema': 1, 'producer': producer(), 'receipt': lane('qualify', request())})
        handoff = {'schema': 1, 'tag': request()['tag'], 'requestSha256': 'a'*64, 'manifestSha256': 'b'*64,
                   'producer': producer(), 'artifact': {'id': 2, 'name': h.artifact_name(request()['tag'], 1), 'digest': 'sha256:'+'c'*64}}
        definitions['release-handoff'].validate(handoff); h.validate_handoff(handoff)
        for kind, value in [('source-request', request()), ('release-handoff', handoff)]:
            bad = copy.deepcopy(value); bad['privateLog'] = 'must not be public'
            with self.assertRaises(jsonschema.ValidationError): definitions[kind].validate(bad)

    def test_environment_exact_branch_or_tag_policy_required(self):
        for name, branch, kind in [('lsp-producer', 'main', 'branch'), ('lsp-releases', 'terraphim_lsp-v*', 'tag')]:
            metadata = {'name': name, 'deployment_branch_policy': {'protected_branches': False, 'custom_branch_policies': True}}
            policy = {'branch_policies': [{'name': branch, 'type': kind}]}
            h.validate_environment(metadata, policy, name)
            for bad in ({'branch_policies': []}, {'branch_policies': [{'name': '*', 'type': kind}]},
                        {'branch_policies': [{'name': branch, 'type': kind}, {'name': 'feature', 'type': 'branch'}]}):
                with self.assertRaises(ValueError): h.validate_environment(metadata, bad, name)
            with self.assertRaises(ValueError): h.validate_environment({'name': name, 'deployment_branch_policy': None}, policy, name)

    def test_same_source_transport_hash_preserved(self):
        actual = hashlib.sha256((ROOT/'.github/scripts/release/publish_lsp_release.py').read_bytes()).hexdigest()
        self.assertEqual(actual, 'c87fdaa79a4901b039bce51c6a440a42bd950d1079968b97de17dab4752016c0')

    def test_contract_ci_runs_handoff_and_schema_tests(self):
        text = (ROOT/'.github/workflows/release-manifest-contract.yml').read_text()
        for name in ('tests.lsp_handoff_test', 'tests.lsp_private_producer_test', 'tests.workflow_lsp_handoff_test'):
            self.assertIn(name, text)
        for path in ('.github/workflows/lsp-producer.yml', '.github/workflows/lsp-publish.yml', 'docs/contracts/schemas/lsp-*.schema.json', 'tests/lsp_handoff_test.py', 'tests/lsp_private_producer_test.py', 'tests/workflow_lsp_handoff_test.py'):
            self.assertGreaterEqual(text.count(path), 2)

if __name__ == '__main__': unittest.main()
