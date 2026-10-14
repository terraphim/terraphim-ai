#!/usr/bin/env python3
"""Explicit private-source provenance; never relax the same-source publisher."""
from __future__ import annotations

import argparse
import hashlib
import hmac
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import zipfile

_spec = importlib.util.spec_from_file_location('lsp_publisher', Path(__file__).with_name('publish_lsp_release.py'))
pub = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pub)
REPO = 'terraphim/terraphim-ai'
PRIVATE_REPO = 'https://git.terraphim.cloud/' + REPO
WORKFLOW = '.github/workflows/lsp-producer.yml'
PUBLISH_WORKFLOW = '.github/workflows/lsp-publish.yml'
RECIPE = '.github/scripts/release/build_lsp_lane.py'
SOURCE_CHECKS = ('native-ci / build (push)', 'native-clippy / lint (push)')
LANES = ('qualify', 'x86_64-unknown-linux-gnu', 'x86_64-unknown-linux-musl', 'aarch64-unknown-linux-musl', 'macos')
LANE_ASSETS = {name: (('terraphim-lsp-' + name,) if name not in ('qualify', 'macos') else ()) for name in LANES}
LANE_ASSETS['macos'] = tuple(n for n in pub.RAW_ASSETS if n.endswith('apple-darwin'))
COMMAND_NAMES = {
    'qualify': ('recipe-tests', 'install-toolchain', 'fmt', 'clippy', 'test'),
    'x86_64-unknown-linux-gnu': ('install-toolchain', 'fetch', 'build', 'architecture', 'version'),
    'x86_64-unknown-linux-musl': ('install-toolchain', 'install-cross', 'fetch', 'build', 'architecture', 'version'),
    'aarch64-unknown-linux-musl': ('install-toolchain', 'install-cross', 'fetch', 'build', 'architecture'),
    'macos': ('install-toolchain', 'fetch', 'build-arm64', 'build-x86_64', 'lipo', 'architecture-arm64',
              'architecture-x86_64', 'architecture-universal', 'version-native', 'version-universal'),
}
HEX64 = re.compile(r'[0-9a-f]{64}')
MAX_BYTES = 6 * 128 * 1024 * 1024 + 1024 * 1024


def require(condition, message='LSP_PROVENANCE_REJECTED'):
    if not condition:
        raise ValueError(message)


def mapping(value):
    require(isinstance(value, dict), 'LSP_METADATA_SHAPE_REJECTED')
    return value


def keys(value, expected):
    require(isinstance(value, dict) and set(value) == set(expected), 'LSP_UNSAFE_OR_MISSING_FIELDS')


def hex_value(value, length=64):
    require(isinstance(value, str) and re.fullmatch(r'[0-9a-f]{' + str(length) + r'}', value))


def integer(value):
    require(type(value) is int and value > 0)


def canonical_digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def validate_request(value):
    keys(value, ('schema', 'component', 'version', 'tag', 'toolchain', 'source', 'recipe'))
    require(type(value['schema']) is int and value['schema'] == 1 and value['component'] == 'terraphim-lsp')
    require(isinstance(value['version'], str) and re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', value['version']))
    require(isinstance(value['tag'], str)); pub.validate_tag(value['tag'], value['version'])
    require(isinstance(value['toolchain'], str) and re.fullmatch(r'[0-9]+\.[0-9]+(?:\.[0-9]+)?', value['toolchain']))
    source = value['source']; keys(source, ('repository', 'commit', 'tag', 'requiredChecks'))
    require(source['repository'] == PRIVATE_REPO and source['tag'] == value['tag'])
    hex_value(source['commit'], 40)
    require(source['requiredChecks'] == list(SOURCE_CHECKS))
    keys(value['recipe'], ('path', 'sha256')); require(value['recipe']['path'] == RECIPE)
    hex_value(value['recipe']['sha256'])
    return value


def validate_producer(value):
    keys(value, ('repository', 'workflow', 'controlCommit', 'runId', 'runAttempt', 'event'))
    require(value['repository'] == REPO and value['workflow'] == WORKFLOW and value['event'] == 'workflow_dispatch')
    hex_value(value['controlCommit'], 40); integer(value['runId']); integer(value['runAttempt'])
    return value


def validate_assets(records, names):
    require(isinstance(records, list) and len(records) == len(names))
    for item, name in zip(records, names):
        keys(item, ('name', 'sha256', 'bytes'))
        require(item['name'] == name); hex_value(item['sha256']); integer(item['bytes'])
        require(item['bytes'] <= 128 * 1024 * 1024)


def validate_lane(value, request):
    validate_request(request)
    keys(value, ('schema', 'lane', 'status', 'sourceCommit', 'sourceTag', 'version', 'toolchain',
                 'recipeSha256', 'signing', 'commands', 'assets'))
    name = value['lane']; require(isinstance(name, str) and name in LANES)
    require(type(value['schema']) is int and value['schema'] == 1 and value['status'] == 'PASS')
    require(value['sourceCommit'] == request['source']['commit'] and value['sourceTag'] == request['tag']
            and value['version'] == request['version'] and value['toolchain'] == request['toolchain']
            and value['recipeSha256'] == request['recipe']['sha256'])
    # This producer has no broad signing credential. Never infer signing from an
    # executable's ad-hoc signature or a previous release.
    require(value['signing'] == ('unsigned' if name == 'macos' else 'not-applicable'))
    require(isinstance(value['commands'], list) and len(value['commands']) == len(COMMAND_NAMES[name]))
    for command, expected in zip(value['commands'], COMMAND_NAMES[name]):
        keys(command, ('name', 'exitCode', 'logSha256'))
        require(command['name'] == expected and type(command['exitCode']) is int and command['exitCode'] == 0)
        hex_value(command['logSha256'])
    validate_assets(value['assets'], LANE_ASSETS[name])
    return value


def artifact_name(tag, attempt=None):
    require(isinstance(tag, str) and pub.TAG.fullmatch(tag))
    if attempt is not None: integer(attempt)
    return 'lsp-frozen-' + tag + (f'-attempt{attempt}' if attempt is not None else '')


def assemble_public_bundle(folders, request, producer, output):
    validate_request(request); validate_producer(producer)
    require(not output.exists() and not output.is_symlink())
    require(len(folders) == len(LANES))
    receipts = []
    for folder, name in zip(folders, LANES):
        pub.asset_paths(folder, (*LANE_ASSETS[name], 'lane.json'))
        envelope = pub.read_json(folder / 'lane.json')
        keys(envelope, ('schema', 'producer', 'receipt'))
        require(type(envelope['schema']) is int and envelope['schema'] == 1 and envelope['producer'] == producer)
        receipt = validate_lane(envelope['receipt'], request)
        require(receipt['lane'] == name)
        require(receipt['assets'] == pub.asset_records([folder / n for n in LANE_ASSETS[name]]))
        receipts.append(receipt)
    output.mkdir(parents=True)
    for folder, name in zip(folders, LANES):
        for asset in LANE_ASSETS[name]: shutil.copyfile(folder / asset, output / asset)
    (output / 'checksums.txt').write_text(''.join(f'{pub.digest(output / n)}  {n}\n' for n in pub.RAW_ASSETS), encoding='ascii')
    manifest = {'schema': 1, 'component': 'terraphim-lsp', 'provenanceMode': 'private-source-handoff-v1',
                'requestSha256': canonical_digest(request), 'privateSource': request['source'],
                'version': request['version'], 'toolchain': request['toolchain'], 'recipe': request['recipe'],
                'producer': producer, 'lanes': receipts, 'universalMacosSigning': 'unsigned',
                'assets': pub.asset_records([output / n for n in (*pub.RAW_ASSETS, 'checksums.txt')])}
    pub.write_new(output / pub.MANIFEST, manifest)
    verify_bundle(output, request, producer)
    return manifest


def verify_bundle(folder, request, producer):
    validate_request(request); validate_producer(producer)
    paths = pub.asset_paths(folder, pub.RELEASE_FILES); pub.verify_sums(folder)
    manifest = pub.read_json(folder / pub.MANIFEST)
    keys(manifest, ('schema', 'component', 'provenanceMode', 'requestSha256', 'privateSource', 'version',
                    'toolchain', 'recipe', 'producer', 'lanes', 'universalMacosSigning', 'assets'))
    require(type(manifest['schema']) is int and manifest['schema'] == 1 and manifest['component'] == 'terraphim-lsp'
            and manifest['provenanceMode'] == 'private-source-handoff-v1')
    require(manifest['requestSha256'] == canonical_digest(request) and manifest['privateSource'] == request['source']
            and manifest['version'] == request['version'] and manifest['toolchain'] == request['toolchain']
            and manifest['recipe'] == request['recipe'] and manifest['producer'] == producer
            and manifest['universalMacosSigning'] == 'unsigned')
    require(isinstance(manifest['lanes'], list) and len(manifest['lanes']) == len(LANES))
    for lane, name in zip(manifest['lanes'], LANES):
        validate_lane(lane, request); require(lane['lane'] == name)
        require(lane['assets'] == pub.asset_records([folder / n for n in LANE_ASSETS[name]]))
    require(manifest['assets'] == pub.asset_records(paths[:-1]))
    return manifest, paths


def validate_handoff(value):
    keys(value, ('schema', 'tag', 'requestSha256', 'manifestSha256', 'producer', 'artifact'))
    require(type(value['schema']) is int and value['schema'] == 1)
    artifact_name(value['tag']); hex_value(value['requestSha256']); hex_value(value['manifestSha256'])
    validate_producer(value['producer'])
    artifact = value['artifact']; keys(artifact, ('id', 'name', 'digest')); integer(artifact['id'])
    require(artifact['name'] == artifact_name(value['tag'], value['producer']['runAttempt']) and isinstance(artifact['digest'], str)
            and artifact['digest'].startswith('sha256:'))
    hex_value(artifact['digest'].removeprefix('sha256:'))
    return value


def verify_producer(handoff, run, artifact):
    validate_handoff(handoff); producer = handoff['producer']; wanted = handoff['artifact']
    mapping(run); mapping(artifact)
    repository = mapping(run.get('repository')); head_repository = mapping(run.get('head_repository'))
    artifact_run = mapping(artifact.get('workflow_run'))
    require(run.get('id') == producer['runId'] and run.get('run_attempt') == producer['runAttempt']
            and repository.get('full_name') == REPO and head_repository.get('full_name') == REPO
            and run.get('path') == WORKFLOW and run.get('head_sha') == producer['controlCommit']
            and run.get('head_branch') == 'main' and run.get('event') == 'workflow_dispatch'
            and run.get('status') == 'completed' and run.get('conclusion') == 'success', 'LSP_PRODUCER_PROOF_REJECTED')
    require(all(artifact.get(k) == wanted[k] for k in ('id', 'name', 'digest'))
            and artifact.get('expired') is False and type(artifact.get('size_in_bytes')) is int
            and 0 < artifact['size_in_bytes'] <= MAX_BYTES and artifact_run.get('id') == producer['runId']
            and artifact_run.get('head_sha') == producer['controlCommit']
            and artifact_run.get('head_branch') == 'main', 'LSP_ARTIFACT_PROOF_REJECTED')


def extract_bundle(archive, destination, max_bytes=MAX_BYTES):
    require(archive.is_file() and not archive.is_symlink() and archive.stat().st_size <= MAX_BYTES)
    require(not destination.exists() and not destination.is_symlink())
    with zipfile.ZipFile(archive) as source:
        entries = source.infolist()
        require(len(entries) == len(pub.RELEASE_FILES) and {e.filename for e in entries} == set(pub.RELEASE_FILES))
        require(sum(e.file_size for e in entries) <= max_bytes)
        for entry in entries:
            kind = stat.S_IFMT(entry.external_attr >> 16)
            require(not entry.is_dir() and kind in (0, stat.S_IFREG) and not (entry.flag_bits & 1)
                    and 0 < entry.file_size <= 128 * 1024 * 1024)
        destination.mkdir(parents=True)
        for entry in entries:
            with source.open(entry) as src, (destination / entry.filename).open('xb') as dest:
                shutil.copyfileobj(src, dest)
    pub.asset_paths(destination, pub.RELEASE_FILES)


def public_control(root, commit, tag=None):
    hex_value(commit, 40)
    require(pub.git(root, 'remote', 'get-url', 'origin').removesuffix('.git') == 'https://github.com/' + REPO)
    require(pub.git(root, 'rev-parse', 'HEAD') == commit and not pub.git(root, 'status', '--porcelain', '--untracked-files=no'))
    subprocess.run(['git', 'merge-base', '--is-ancestor', commit, 'origin/main'], cwd=root, check=True, capture_output=True, timeout=30)
    if tag:
        artifact_name(tag)
        try: actual = subprocess.check_output(['git', 'rev-parse', '--verify', f'refs/tags/{tag}^{{commit}}'], cwd=root, text=True, stderr=subprocess.PIPE, timeout=30).strip()
        except subprocess.SubprocessError as error: raise ValueError('LSP_CONTROL_TAG_MISSING') from error
        require(actual == commit)


def verify_handoff_bundle(handoff, request, folder):
    validate_handoff(handoff); validate_request(request)
    require(handoff['tag'] == request['tag'] and handoff['requestSha256'] == canonical_digest(request)
            and hmac.compare_digest(handoff['manifestSha256'], pub.digest(folder / pub.MANIFEST)))
    return verify_bundle(folder, request, handoff['producer'])


def validate_environment(metadata, policies, name):
    mapping(metadata); mapping(policies)
    expected = ('main', 'branch') if name == 'lsp-producer' else ('terraphim_lsp-v*', 'tag')
    require(name in ('lsp-producer', 'lsp-releases') and metadata.get('name') == name
            and metadata.get('deployment_branch_policy') == {'protected_branches': False, 'custom_branch_policies': True})
    entries = policies.get('branch_policies')
    require(isinstance(entries, list) and len(entries) == 1
            and (mapping(entries[0]).get('name'), mapping(entries[0]).get('type')) == expected,
            'LSP_ENVIRONMENT_PROTECTION_REQUIRED')


def verify_environment(name):
    prefix = f'repos/{REPO}/environments/{name}'
    validate_environment(pub.api(prefix), pub.api(prefix + '/deployment-branch-policies'), name)


def producer_context():
    require(os.environ.get('GITHUB_REPOSITORY') == REPO and os.environ.get('GITHUB_EVENT_NAME') == 'workflow_dispatch'
            and os.environ.get('GITHUB_REF') == 'refs/heads/main'
            and os.environ.get('GITHUB_WORKFLOW_REF') == f'{REPO}/{WORKFLOW}@refs/heads/main'
            and os.environ.get('GITHUB_WORKFLOW_SHA') == os.environ.get('GITHUB_SHA'))
    return validate_producer({'repository': REPO, 'workflow': WORKFLOW, 'controlCommit': os.environ['GITHUB_SHA'],
                              'runId': int(os.environ['GITHUB_RUN_ID']), 'runAttempt': int(os.environ['GITHUB_RUN_ATTEMPT']),
                              'event': 'workflow_dispatch'})


def request_path(root, tag):
    artifact_name(tag)
    path = root / 'release/lsp/requests' / (tag + '.json')
    require(path.is_file() and not path.is_symlink())
    return path


def publish_handoff(root, tag, commit, receipt):
    public_control(root, commit, tag)
    require(os.environ.get('GITHUB_REPOSITORY') == REPO and os.environ.get('GITHUB_EVENT_NAME') == 'push'
            and os.environ.get('GITHUB_REF') == 'refs/tags/' + tag
            and os.environ.get('GITHUB_SHA') == commit and os.environ.get('GITHUB_WORKFLOW_SHA') == commit
            and os.environ.get('GITHUB_WORKFLOW_REF') == f'{REPO}/{PUBLISH_WORKFLOW}@refs/tags/{tag}')
    verify_environment('lsp-releases')
    handoff_path = root / 'release/lsp/handoffs' / (tag + '.json')
    require(handoff_path.is_file() and not handoff_path.is_symlink())
    handoff = validate_handoff(pub.read_json(handoff_path)); request = validate_request(pub.read_json(request_path(root, tag)))
    producer = handoff['producer']; artifact = handoff['artifact']
    # Read the original run AND current attempt: a later rerun cannot silently
    # replace an already reviewed producer attempt.
    run = pub.api(f'repos/{REPO}/actions/runs/{producer["runId"]}')
    attempt = pub.api(f'repos/{REPO}/actions/runs/{producer["runId"]}/attempts/{producer["runAttempt"]}')
    metadata = pub.api(f'repos/{REPO}/actions/artifacts/{artifact["id"]}')
    verify_producer(handoff, run, metadata); verify_producer(handoff, attempt, metadata)
    with tempfile.TemporaryDirectory(prefix='lsp-handoff-') as temp:
        folder = Path(temp); archive = folder / 'artifact.zip'
        with archive.open('xb') as output:
            subprocess.run(['gh', 'api', '--hostname', 'github.com', f'repos/{REPO}/actions/artifacts/{artifact["id"]}/zip'],
                           stdout=output, stderr=subprocess.PIPE, check=True, timeout=600)
        require(hmac.compare_digest('sha256:' + pub.digest(archive), artifact['digest']), 'LSP_ARCHIVE_DIGEST_REJECTED')
        assets = folder / 'assets'; extract_bundle(archive, assets)
        _, files = verify_handoff_bundle(handoff, request, assets)
        notes = (f'Private-source LSP prerelease {tag}.\n\n'
                 f'Build source: `{request["source"]["commit"]}` on `{PRIVATE_REPO}`.\n'
                 f'Public release control: `{commit}`.\n'
                 f'Producer: https://github.com/{REPO}/actions/runs/{producer["runId"]} '
                 f'(attempt {producer["runAttempt"]}).\n'
                 'Six frozen binaries, checksums and provenance manifest. macOS binaries are unsigned.\n')
        result = pub.publish(REPO, tag, files, commit, notes)
        # The shared transport's source means tag target. Do not mislabel it as
        # the private build source in this explicit mode.
        result['publicControlCommit'] = result.pop('sourceCommit')
        result['privateSource'] = request['source']; result['producer'] = producer
        result['provenanceMode'] = 'private-source-handoff-v1'
        pub.write_new(receipt, result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('check-request', 'assemble', 'publish'))
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--tag', required=True); parser.add_argument('--lanes', type=Path)
    parser.add_argument('--output', type=Path); parser.add_argument('--receipt', type=Path)
    args = parser.parse_args()
    try:
        require(args.receipt and not args.receipt.exists() and not args.receipt.is_symlink())
        request = validate_request(pub.read_json(request_path(args.root, args.tag)))
        if args.command == 'publish':
            publish_handoff(args.root, args.tag, os.environ['GITHUB_SHA'], args.receipt)
        else:
            producer = producer_context(); public_control(args.root, producer['controlCommit'])
            if args.command == 'check-request':
                verify_environment('lsp-producer')
                require(pub.api(f'repos/{REPO}/git/ref/heads/main')['object']['sha'] == producer['controlCommit'])
            if args.command == 'assemble':
                assemble_public_bundle([args.lanes / name for name in LANES], request, producer, args.output)
            pub.write_new(args.receipt, {'schema': 1, 'status': 'PASS', 'requestSha256': canonical_digest(request), 'producer': producer})
        return 0
    except (ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError, zipfile.BadZipFile):
        # Never copy raw exception text or private stderr into public Actions logs.
        if args.receipt and not args.receipt.exists() and not args.receipt.is_symlink():
            pub.write_new(args.receipt, {'schema': 1, 'status': 'FAIL', 'errorCode': 'LSP_HANDOFF_REJECTED'})
        print('LSP_HANDOFF_REJECTED', file=sys.stderr)
        return 1


if __name__ == '__main__': sys.exit(main())
