"""Real files/Git/ZIP tests; metadata samples do not prove remote publication."""
import copy
import importlib.util
import json
from pathlib import Path
import stat
import sys
import subprocess
import tempfile
import unittest
import zipfile
import warnings

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('handoff', ROOT / '.github/scripts/release/lsp_handoff.py')
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)


def request():
    return {'schema': 1, 'component': 'terraphim-lsp', 'version': '1.22.0',
            'tag': 'terraphim_lsp-v1.22.0-rc.3', 'toolchain': '1.96',
            'source': {'repository': h.PRIVATE_REPO, 'commit': '1' * 40,
                       'tag': 'terraphim_lsp-v1.22.0-rc.3', 'requiredChecks': list(h.SOURCE_CHECKS)},
            'recipe': {'path': h.RECIPE, 'sha256': '2' * 64}}


def producer():
    return {'repository': h.REPO, 'workflow': h.WORKFLOW, 'controlCommit': '3' * 40,
            'runId': 100, 'runAttempt': 1, 'event': 'workflow_dispatch'}


def lane(name, req, assets=()):
    return {'schema': 1, 'lane': name, 'status': 'PASS', 'sourceCommit': req['source']['commit'],
            'sourceTag': req['tag'], 'version': req['version'], 'toolchain': req['toolchain'],
            'recipeSha256': req['recipe']['sha256'], 'signing': 'unsigned' if name == 'macos' else 'not-applicable',
            'commands': [{'name': n, 'exitCode': 0, 'logSha256': '4' * 64} for n in h.COMMAND_NAMES[name]],
            'assets': list(assets)}


class ContractTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.addCleanup(self.temp.cleanup)

    def test_request_exact_and_extra_private_data_refused(self):
        self.assertEqual(h.validate_request(request()), request())
        for path, value in [('toolchain', 'stable'), ('tag', '../escape'), ('extra', 'private log')]:
            bad = request(); bad[path] = value
            with self.subTest(path=path), self.assertRaises(ValueError): h.validate_request(bad)
        for key, value in [('repository', 'https://example.com/source'), ('commit', 'main'), ('tag', 'other'), ('requiredChecks', [])]:
            bad = request(); bad['source'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): h.validate_request(bad)
        for value in ['../secret.py', '.github/scripts/release/other.py']:
            bad = request(); bad['recipe']['path'] = value
            with self.assertRaises(ValueError): h.validate_request(bad)

    def test_lane_refuses_missing_failed_extra_or_source_changes(self):
        good = lane('qualify', request())
        h.validate_lane(good, request())
        variants = []
        for key, value in [('sourceCommit', '5' * 40), ('toolchain', 'nightly'), ('privateLog', 'secret'), ('status', 'FAIL')]:
            bad = copy.deepcopy(good); bad[key] = value; variants.append(bad)
        bad = copy.deepcopy(good); bad['commands'][0]['exitCode'] = 1; variants.append(bad)
        bad = copy.deepcopy(good); bad['commands'][0]['stderr'] = 'private'; variants.append(bad)
        bad = copy.deepcopy(good); bad['commands'].pop(); variants.append(bad)
        for bad in variants:
            with self.assertRaises(ValueError): h.validate_lane(bad, request())

    def bundle(self):
        req = request(); folders = []
        for name in h.LANES:
            folder = self.root / name; folder.mkdir(); folders.append(folder)
            for asset in h.LANE_ASSETS[name]: (folder / asset).write_bytes(b'fixture-not-a-released-binary:' + asset.encode())
            receipt = lane(name, req, h.pub.asset_records([folder / n for n in h.LANE_ASSETS[name]]))
            h.pub.write_new(folder / 'lane.json', {'schema': 1, 'producer': producer(), 'receipt': receipt})
        out = self.root / 'bundle'
        manifest = h.assemble_public_bundle(folders, req, producer(), out)
        return req, out, manifest

    def test_bundle_checks_exact_bytes_and_no_private_logs(self):
        req, out, manifest = self.bundle()
        h.verify_bundle(out, req, producer())
        self.assertEqual(set(p.name for p in out.iterdir()), set(h.pub.RELEASE_FILES))
        self.assertEqual(manifest['provenanceMode'], 'private-source-handoff-v1')
        (out / h.pub.RAW_ASSETS[0]).write_bytes(b'changed')
        with self.assertRaises(ValueError): h.verify_bundle(out, req, producer())

    def test_lane_directory_extra_and_symlink_refused(self):
        req, out, manifest = self.bundle()
        folder = self.root / 'qualify'; (folder / 'private.log').write_text('private')
        with self.assertRaises(ValueError): h.assemble_public_bundle([self.root / n for n in h.LANES], req, producer(), self.root / 'out2')
        (folder / 'private.log').unlink()
        asset = self.root / 'macos' / h.LANE_ASSETS['macos'][0]
        data = asset.read_bytes(); asset.unlink(); (self.root / 'outside').write_bytes(data); asset.symlink_to(self.root / 'outside')
        with self.assertRaises(ValueError): h.assemble_public_bundle([self.root / n for n in h.LANES], req, producer(), self.root / 'out3')

    def test_mixed_attempt_lane_receipt_refused(self):
        req, out, manifest = self.bundle()
        path = self.root / 'qualify/lane.json'; envelope = h.pub.read_json(path)
        envelope['producer']['runAttempt'] = 2; path.write_text(json.dumps(envelope))
        with self.assertRaises(ValueError): h.assemble_public_bundle([self.root/n for n in h.LANES], req, producer(), self.root/'mixed')

    def test_cli_missing_request_saves_only_fixed_code(self):
        receipt = self.root/'failure.json'
        result = subprocess.run([sys.executable, str(ROOT/'.github/scripts/release/lsp_handoff.py'), 'check-request',
                                 '--root', str(self.root), '--tag', request()['tag'], '--receipt', str(receipt)],
                                 text=True, capture_output=True, check=False, timeout=30)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stderr.strip(), 'LSP_HANDOFF_REJECTED')
        self.assertEqual(h.pub.read_json(receipt), {'schema': 1, 'status': 'FAIL', 'errorCode': 'LSP_HANDOFF_REJECTED'})
        self.assertNotIn(str(self.root), result.stderr)

    def test_zip_limits_paths_duplicates_and_symlinks(self):
        for index, names in enumerate([['../secret'], ['/secret'], ['folder/file'], ['checksums.txt', 'checksums.txt'], ['unexpected']]):
            z = self.root / f'bad{index}.zip'
            with warnings.catch_warnings(), zipfile.ZipFile(z, 'w') as archive:
                warnings.simplefilter('ignore', UserWarning)
                for name in names: archive.writestr(name, b'x')
            with self.subTest(names=names), self.assertRaises(ValueError): h.extract_bundle(z, self.root / f'extracted{index}')
        z = self.root / 'symlink.zip'
        with zipfile.ZipFile(z, 'w') as archive:
            for name in h.pub.RELEASE_FILES:
                info = zipfile.ZipInfo(name)
                info.create_system = 3; info.external_attr = (stat.S_IFLNK | 0o777) << 16
                archive.writestr(info, b'elsewhere')
        with self.assertRaises(ValueError): h.extract_bundle(z, self.root / 'symlink-out')

    def test_zip_real_bundle_roundtrip(self):
        req, out, manifest = self.bundle(); z = self.root / 'bundle.zip'
        with zipfile.ZipFile(z, 'w') as archive:
            for path in out.iterdir(): archive.write(path, path.name)
        dest = self.root / 'unpacked'; h.extract_bundle(z, dest)
        h.verify_bundle(dest, req, producer())
        with self.assertRaises(ValueError): h.extract_bundle(z, dest)
        with self.assertRaises(ValueError): h.extract_bundle(z, self.root / 'small', max_bytes=1)

    def metadata(self):
        req, out, manifest = self.bundle()
        proof = {'schema': 1, 'tag': req['tag'], 'requestSha256': h.canonical_digest(req),
                 'manifestSha256': h.pub.digest(out / h.pub.MANIFEST), 'producer': producer(),
                 'artifact': {'id': 200, 'name': h.artifact_name(req['tag'], 1), 'digest': 'sha256:' + '6' * 64}}
        run = {'id': 100, 'run_attempt': 1, 'repository': {'full_name': h.REPO}, 'head_repository': {'full_name': h.REPO},
               'head_sha': '3' * 40, 'head_branch': 'main', 'path': h.WORKFLOW, 'event': 'workflow_dispatch',
               'status': 'completed', 'conclusion': 'success'}
        art = {'id': 200, 'name': proof['artifact']['name'], 'digest': proof['artifact']['digest'], 'expired': False, 'size_in_bytes': 1000,
               'workflow_run': {'id': 100, 'head_sha': '3' * 40, 'head_branch': 'main'}}
        return req, out, proof, run, art

    def test_producer_proof_refuses_every_identity_failure(self):
        req, out, proof, run, art = self.metadata(); h.verify_producer(proof, run, art)
        for key, val in [('id', 101), ('run_attempt', 2), ('head_sha', '7' * 40), ('head_branch', 'feature'), ('path', 'other.yml'), ('event', 'pull_request'), ('status', 'in_progress'), ('conclusion', 'failure'), ('repository', {'full_name': 'fork/repo'}), ('head_repository', {'full_name': 'fork/repo'})]:
            bad = copy.deepcopy(run); bad[key] = val
            with self.subTest(key=key), self.assertRaises(ValueError): h.verify_producer(proof, bad, art)
        for key, val in [('id', 201), ('digest', 'sha256:' + '7' * 64), ('expired', True), ('name', 'other'), ('workflow_run', {'id': 101})]:
            bad = copy.deepcopy(art); bad[key] = val
            with self.subTest(key=key), self.assertRaises(ValueError): h.verify_producer(proof, run, bad)

    def test_malformed_remote_shapes_raise_safe_validation_error(self):
        req, out, proof, run, art = self.metadata()
        for malformed in (None, [], 'wrong', 4):
            with self.subTest(top=malformed), self.assertRaises(ValueError): h.verify_producer(proof, malformed, art)
            with self.assertRaises(ValueError): h.verify_producer(proof, run, malformed)
            for field in ('repository', 'head_repository'):
                bad = copy.deepcopy(run); bad[field] = malformed
                with self.subTest(field=field, value=malformed), self.assertRaises(ValueError): h.verify_producer(proof, bad, art)
            bad = copy.deepcopy(art); bad['workflow_run'] = malformed
            with self.assertRaises(ValueError): h.verify_producer(proof, run, bad)
            with self.assertRaises(ValueError): h.validate_environment(malformed, {}, 'lsp-producer')

    def test_real_git_public_control_tag_main_and_dirty_guards(self):
        repo = self.root / 'git'; repo.mkdir()
        def git(*args): return subprocess.check_output(['git', *args], cwd=repo, text=True, stderr=subprocess.DEVNULL, timeout=30).strip()
        git('init', '-b', 'main'); git('config', 'user.name', 'Fixture'); git('config', 'user.email', 'fixture@example.invalid')
        git('remote', 'add', 'origin', 'https://github.com/' + h.REPO + '.git')
        (repo / 'control').write_text('reviewed'); git('add', '.'); git('commit', '-m', 'fixture')
        sha = git('rev-parse', 'HEAD'); git('update-ref', 'refs/remotes/origin/main', sha); git('tag', request()['tag'])
        h.public_control(repo, sha, request()['tag'])
        with self.assertRaises(ValueError): h.public_control(repo, '0' * 40, request()['tag'])
        (repo / 'control').write_text('changed')
        with self.assertRaises(ValueError): h.public_control(repo, sha, request()['tag'])
        git('checkout', '--', 'control'); git('tag', '-d', request()['tag'])
        with self.assertRaises(ValueError): h.public_control(repo, sha, request()['tag'])

    def test_handoff_changed_manifest_or_request_refused(self):
        req, out, proof, run, art = self.metadata(); h.validate_handoff(proof)
        h.verify_handoff_bundle(proof, req, out)
        bad = copy.deepcopy(proof); bad['requestSha256'] = '0' * 64
        with self.assertRaises(ValueError): h.verify_handoff_bundle(bad, req, out)
        bad = copy.deepcopy(proof); bad['manifestSha256'] = '0' * 64
        with self.assertRaises(ValueError): h.verify_handoff_bundle(bad, req, out)
        bad = copy.deepcopy(proof); bad['privateLog'] = 'oops'
        with self.assertRaises(ValueError): h.validate_handoff(bad)

if __name__ == '__main__': unittest.main()
