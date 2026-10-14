#!/usr/bin/env python3
"""Protected private checkout boundary. Never upload source or raw diagnostics."""
from __future__ import annotations
import argparse
import json
import hmac
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import urllib.error
from urllib.parse import urlsplit

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lsp_handoff as h

API = 'https://git.terraphim.cloud/api/v1'
PRIVATE_ENV = ('PATH', 'HOME', 'RUSTUP_HOME', 'TMPDIR', 'TMP', 'TEMP', 'SDKROOT', 'DEVELOPER_DIR',
               'GITEA_LSP_SOURCE_TOKEN', 'CARGO_REGISTRIES_TERRAPHIM_TOKEN')


def credentials(environment):
    token = environment.get('GITEA_LSP_SOURCE_TOKEN', '')
    registry = environment.get('CARGO_REGISTRIES_TERRAPHIM_TOKEN', '')
    h.require(bool(token) and '\n' not in token and '\r' not in token, 'LSP_SOURCE_CREDENTIAL_MISSING')
    h.require(registry.startswith('Bearer ') and len(registry) > 7 and '\n' not in registry and '\r' not in registry,
              'LSP_REGISTRY_CREDENTIAL_MISSING')
    # No fallback to broad GITEA_TOKEN, OP_SERVICE_ACCOUNT_TOKEN or runner auth.
    return {k: environment[k] for k in PRIVATE_ENV if k in environment}


def askpass_response(prompt, token):
    match = re.fullmatch(r"(Username|Password) for '([^']+)': ?", prompt)
    h.require(match is not None, 'LSP_CREDENTIAL_PROMPT_REJECTED')
    url = urlsplit(match[2])
    h.require(url.scheme == 'https' and url.hostname == 'git.terraphim.cloud'
              and url.netloc in ('git.terraphim.cloud', 'token@git.terraphim.cloud')
              and url.path in ('/terraphim/terraphim-ai.git', '/terraphim/terraphim-editor.git')
              and not url.query and not url.fragment, 'LSP_CREDENTIAL_PROMPT_REJECTED')
    return 'token' if match[1] == 'Username' else token


def private_api(path, token):
    request = urllib.request.Request(API + path, headers={'Authorization': 'token ' + token, 'Accept': 'application/json'})
    # Disallow credential-bearing redirects, including a changed API hostname.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl): return None
    with urllib.request.build_opener(NoRedirect).open(request, timeout=60) as response:
        data = response.read(2 * 1024 * 1024 + 1)
    h.require(len(data) <= 2 * 1024 * 1024)
    return json.loads(data, object_pairs_hook=h.pub.unique_object)


def verify_source_metadata(request, repository, tag, status, editor):
    h.validate_request(request)
    for value in (repository, tag, status, editor): h.mapping(value)
    commit = h.mapping(tag.get('commit'))
    h.require(repository.get('full_name') == h.REPO and repository.get('default_branch') == 'main'
              and repository.get('private') is True and repository.get('archived') is False)
    h.require(editor.get('full_name') == 'terraphim/terraphim-editor' and editor.get('archived') is False)
    h.require(tag.get('name') == request['tag'] and commit.get('sha') == request['source']['commit'])
    h.require(status.get('sha') == request['source']['commit'] and status.get('state') == 'success')
    records = status.get('statuses')
    h.require(isinstance(records, list))
    for record in records: h.mapping(record)
    for context in h.SOURCE_CHECKS:
        found = [v for v in records if v.get('context') == context]
        h.require(found and all(v.get('status') == 'success' for v in found)
                  and any(isinstance(v.get('target_url'), str) and v['target_url'].startswith('/' + h.REPO + '/actions/runs/') for v in found))
    return {'repository': h.PRIVATE_REPO, 'commit': request['source']['commit'], 'tag': request['tag'],
            'requiredChecks': list(h.SOURCE_CHECKS), 'metadataSha256': h.canonical_digest({'tag': tag, 'status': status})}


def preflight(request, token):
    prefix = '/repos/' + h.REPO
    return verify_source_metadata(request, private_api(prefix, token), private_api(prefix + '/tags/' + request['tag'], token),
                                  private_api(prefix + '/commits/' + request['source']['commit'] + '/status', token),
                                  private_api('/repos/terraphim/terraphim-editor', token))


def run_private(argv, cwd, env, log):
    with log.open('xb') as output:
        status = subprocess.run(argv, cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT, timeout=14400, check=False)
    h.require(status.returncode == 0, 'LSP_PRIVATE_STEP_FAILED')


def protected_directory(home):
    # RUNNER_TEMP is erased by Actions after a job. Keep protected diagnostics
    # outside it so a failed build remains inspectable by the runner owner.
    root = home / '.local/state/terraphim-lsp-private'
    for path in (home, home/'.local', home/'.local/state', root):
        h.require(not path.is_symlink(), 'LSP_PRIVATE_PATH_REJECTED')
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    h.require(root.is_dir() and root.stat().st_uid == os.getuid() and root.stat().st_mode & 0o077 == 0,
              'LSP_PRIVATE_PATH_REJECTED')
    return Path(tempfile.mkdtemp(prefix='producer-', dir=root))


def produce(root, request, lane, output):
    producer = h.producer_context(); h.public_control(root, producer['controlCommit'])
    h.require(h.pub.api(f'repos/{h.REPO}/git/ref/heads/main')['object']['sha'] == producer['controlCommit'])
    h.validate_request(request); h.require(lane in h.LANES and not output.exists() and not output.is_symlink())
    env = credentials(os.environ)
    proof = preflight(request, env['GITEA_LSP_SOURCE_TOKEN'])
    private = protected_directory(Path(env['HOME']))
    h.pub.write_new(private/'source-proof.json', proof)
    h.pub.write_new(private/'producer.json', producer)
    source = private/'source'; askpass = private/'askpass.py'
    askpass.write_text('#!' + sys.executable + '\nimport os,sys\n'
                       + 'sys.path.insert(0, ' + repr(str(Path(__file__).resolve().parent)) + ')\n'
                       + 'from lsp_private_producer import askpass_response\n'
                       + 'try: print(askpass_response(sys.argv[1], os.environ["GITEA_LSP_SOURCE_TOKEN"]))\n'
                       + 'except (ValueError,KeyError,IndexError): raise SystemExit(1)\n')
    askpass.chmod(0o700)
    env.update(GIT_ASKPASS=str(askpass), GIT_TERMINAL_PROMPT='0', GIT_CONFIG_NOSYSTEM='1',
               GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_COUNT='2', GIT_CONFIG_KEY_0='credential.helper', GIT_CONFIG_VALUE_0='',
               GIT_CONFIG_KEY_1='credential.useHttpPath', GIT_CONFIG_VALUE_1='true')
    try:
        run_private(['git', 'clone', '--no-checkout', '--single-branch', '--branch', 'main', h.PRIVATE_REPO + '.git', str(source)],
                    private, env, private/'clone.log')
        run_private(['git', 'fetch', 'origin', f'refs/tags/{request["tag"]}:refs/tags/{request["tag"]}'], source, env, private/'tag.log')
        run_private(['git', 'checkout', '--detach', request['source']['commit']], source, env, private/'checkout.log')
        h.require(h.pub.toolchain(source) == request['toolchain'] and h.pub.version(source) == request['version'])
        recipe = source / h.RECIPE
        h.require(recipe.is_file() and not recipe.is_symlink() and hmac.compare_digest(h.pub.digest(recipe), request['recipe']['sha256']))
        # Existing exact-source guard is re-used on the private checkout.
        run_private([sys.executable, str(source/'.github/scripts/release/publish_lsp_release.py'), 'source', '--root', str(source),
                     '--tag', request['tag'], '--source', request['source']['commit'], '--require-tag'], source, env, private/'source-guard.log')
        staged = private/'safe-output'
        run_private([sys.executable, str(recipe), '--root', str(source), '--lane', lane, '--source', request['source']['commit'],
                     '--tag', request['tag'], '--output', str(staged), '--private-work', str(private/'work')], source, env, private/'recipe.log')
        h.pub.asset_paths(staged, (*h.LANE_ASSETS[lane], 'lane.json'))
        receipt = h.validate_lane(h.pub.read_json(staged/'lane.json'), request)
        h.require(receipt['lane'] == lane and receipt['assets'] == h.pub.asset_records([staged/n for n in h.LANE_ASSETS[lane]]))
        # Recheck movable private tags and CI metadata after the long build.
        preflight(request, env['GITEA_LSP_SOURCE_TOKEN'])
        output.mkdir(parents=True)
        for name in h.LANE_ASSETS[lane]: shutil.copyfile(staged/name, output/name)
        h.pub.write_new(output/'lane.json', {'schema': 1, 'producer': producer, 'receipt': receipt})
    finally:
        # No credential value is ever written to disk. The helper also goes away;
        # source, exact receipts and raw diagnostics stay owner-only for review.
        askpass.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--tag', required=True); parser.add_argument('--lane', choices=h.LANES, required=True)
    parser.add_argument('--output', type=Path, required=True); parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    try:
        h.require(not args.receipt.exists() and not args.receipt.is_symlink())
        request = h.validate_request(h.pub.read_json(h.request_path(args.root, args.tag)))
        produce(args.root, request, args.lane, args.output)
        h.pub.write_new(args.receipt, {'schema': 1, 'status': 'PASS', 'lane': args.lane})
        return 0
    except (ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError) as error:
        allowed = {'LSP_SOURCE_CREDENTIAL_MISSING', 'LSP_REGISTRY_CREDENTIAL_MISSING', 'LSP_PRIVATE_PATH_REJECTED'}
        code = str(error) if str(error) in allowed else 'LSP_PRIVATE_PRODUCER_REJECTED'
        if not args.receipt.exists() and not args.receipt.is_symlink():
            h.pub.write_new(args.receipt, {'schema': 1, 'status': 'FAIL', 'errorCode': code, 'lane': args.lane})
        print(code, file=sys.stderr)
        return 1


if __name__ == '__main__': sys.exit(main())
