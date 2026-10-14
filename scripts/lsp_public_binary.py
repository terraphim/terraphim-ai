#!/usr/bin/env python3
"""Build immutable LSP assets from the exact published crates.io archive.

No private source, registry credential or source-handoff request is accepted.
The existing publisher owns the final draft/resume/anonymous-readback policy.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import queue
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


crates = module('lsp_crates_release', ROOT / 'scripts/lsp_crates_release.py')
publisher = module('immutable_lsp_publisher', ROOT / '.github/scripts/release/publish_lsp_release.py')
TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu',
           'x86_64-unknown-linux-musl', 'aarch64-unknown-linux-musl')
RAW = publisher.RAW_ASSETS
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
SHA256 = re.compile(r'[0-9a-f]{64}')
OWNED_DEPENDENCIES = {'terraphim_lsp_core': '1.23.1', 'terraphim_types': '1.22.3',
                      'terraphim_alternatives': '0.1.1', 'terraphim_lab': '0.1.1',
                      'terraphim_automata': '2.1.0', 'terraphim-markdown-parser': '1.20.5'}
CROSS_REVISION = '29d00c7803f221f1b3f35e561b03792368fb8339'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def identity(root, tag, source, toolchain):
    plan = crates.load_plan(root)
    require(toolchain == plan['toolchain'], 'Toolchain differs from reviewed plan')
    item, head = crates.validate_identity(root, plan, tag, os.environ)
    require(item['name'] == 'terraphim_lsp' and source == head, 'Only the exact public LSP tag may build binaries')
    crates.validate_manifests(root, plan)
    return {'repository': 'https://github.com/terraphim/terraphim-ai', 'commit': head,
            'tag': tag, 'version': item['version'], 'toolchain': toolchain}, item


def public_lock(text, item):
    lock = tomllib.loads(text)
    require(not lock.get('patch') and isinstance(lock.get('package'), list), 'Invalid registry lock')
    roots = []
    for package in lock['package']:
        require(isinstance(package, dict), 'Malformed locked package')
        source = package.get('source')
        if source is None:
            roots.append((package.get('name'), package.get('version')))
        else:
            require(source == REGISTRY and isinstance(package.get('checksum'), str)
                    and SHA256.fullmatch(package['checksum']), 'Lock contains a non-registry or unchecked dependency')
    require(roots == [(item['name'], item['version'])], 'Only the published crate may be a path package')
    for name, version in OWNED_DEPENDENCIES.items():
        matches = [p for p in lock['package'] if p.get('name') == name]
        require(len(matches) == 1 and matches[0].get('version') == version and matches[0].get('source') == REGISTRY,
                'Owned dependency differs from reviewed public closure: ' + name)
    return {'sha256': hashlib.sha256(text.encode()).hexdigest(), 'packageCount': len(lock['package']),
            'registry': REGISTRY}


def source_receipt(lane, expected):
    receipt = publisher.read_json(lane / 'source.json')
    require(isinstance(receipt, dict) and receipt.get('schema') == 1 and receipt.get('source') == expected, 'Lane source identity differs')
    require(isinstance(receipt.get('package'), dict) and isinstance(receipt.get('archive'), dict), 'Malformed source receipt')
    item = receipt['package']
    archive = lane / 'source.crate'
    require(crates.inspect_package(archive, item, expected['commit']) == receipt['archive'], 'Published archive changed')
    require(receipt['archive']['sha256'] == receipt['registryChecksum'], 'Registry checksum differs')
    source = lane / 'unpacked' / (item['name'] + '-' + item['version'])
    require(not source.is_symlink() and source.is_dir(), 'Missing extracted source')
    with tarfile.open(archive, 'r:gz') as package:
        prefix = item['name'] + '-' + item['version'] + '/'
        wanted = {member.name.removeprefix(prefix): package.extractfile(member).read() for member in package}
    actual = {str(p.relative_to(source)): p for p in source.rglob('*') if p.is_file() or p.is_symlink()}
    require(set(actual) == set(wanted), 'Extracted source file set changed')
    require(all(not p.is_symlink() and p.read_bytes() == wanted[name] for name, p in actual.items()), 'Extracted source bytes changed')
    require(public_lock((source / 'Cargo.lock').read_text(), item) == receipt['lock'], 'Registry lock changed')
    return receipt, source


def prepare(root, lane, expected, item):
    require(not lane.exists() and not lane.resolve().is_relative_to(root.resolve()), 'Use a new external lane directory')
    status, data = crates.registry_get('/api/v1/crates/terraphim_lsp/' + item['version'])
    version = data.get('version', {}) if isinstance(data, dict) else {}
    require(isinstance(version, dict), 'Malformed registry version')
    checksum = version.get('checksum')
    require(status == 200 and version.get('num') == item['version'] and version.get('yanked') is False
            and isinstance(checksum, str) and SHA256.fullmatch(checksum), 'Exact published LSP version is unavailable')
    url = 'https://static.crates.io/crates/terraphim_lsp/terraphim_lsp-' + item['version'] + '.crate'
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), crates.NoRedirect())
    with opener.open(urllib.request.Request(url, headers={'User-Agent': 'Terraphim public LSP release'}), timeout=60) as response:
        raw = response.read(crates.MAX_CRATE_BYTES + 1)
        require(response.status == 200 and len(raw) <= crates.MAX_CRATE_BYTES, 'Published crate download failed')
    require(hashlib.sha256(raw).hexdigest() == checksum, 'Published crate checksum differs')
    lane.mkdir(parents=True)
    archive = lane / 'source.crate'; archive.write_bytes(raw)
    record = crates.inspect_package(archive, item, expected['commit'])
    extracted = crates.unpack_package(archive, item, expected['commit'], lane / 'unpacked', root)
    lock = public_lock((extracted / 'Cargo.lock').read_text(), item)
    publisher.write_new(lane / 'source.json', {'schema': 1, 'source': expected, 'package': item,
        'archive': record, 'registryChecksum': checksum, 'downloadUrl': url, 'lock': lock})
    return source_receipt(lane, expected)


def environment(lane):
    env = {k: v for k, v in os.environ.items() if not k.startswith(('CARGO_', 'RUSTC_', 'RUSTFLAGS', 'GH_', 'GITHUB_TOKEN', 'GITEA_', 'OP_'))}
    for key in ('RUSTC', 'RUSTDOC', 'RUSTUP_TOOLCHAIN'):
        env.pop(key, None)
    env.update({'CARGO_HOME': str(lane / 'cargo'), 'CARGO_TARGET_DIR': str(lane / 'target'),
                'CARGO_BUILD_BUILD_DIR': str(lane / 'target'), 'CARGO_TERM_COLOR': 'never',
                'RUSTC_WRAPPER': '', 'RUSTC_WORKSPACE_WRAPPER': ''})
    return env


def command(argv, cwd, lane, name):
    logs = lane / 'logs'; logs.mkdir(exist_ok=True)
    path = logs / (name + '.log')
    with path.open('xb') as output:
        result = subprocess.run(argv, cwd=cwd, env=environment(lane), stdout=output,
                                stderr=subprocess.STDOUT, timeout=3600, check=False)
    record = {'argv': argv, 'exitCode': result.returncode, 'log': path.name, 'sha256': publisher.digest(path)}
    require(result.returncode == 0, 'Build command failed: ' + name)
    return record


def qualify(lane, expected):
    receipt, source = source_receipt(lane, expected)
    cargo = ['rustup', 'run', expected['toolchain'], 'cargo']
    commands = []
    for name, args in [('clippy', ['clippy', '--locked', '--all-targets', '--', '-D', 'warnings']),
                       ('test', ['test', '--locked', '--all-targets']),
                       ('doctest', ['test', '--locked', '--doc'])]:
        commands.append(command(cargo + args, source, lane, name))
    source_receipt(lane, expected)
    publisher.write_new(lane / 'qualification.json', {'schema': 1, 'status': 'PASS', 'source': receipt,
                                                     'commands': commands})


def architecture(path):
    data = path.read_bytes()
    if data[:4] == b'\x7fELF':
        require(len(data) >= 64 and data[4:6] == b'\x02\x01', 'Unsupported ELF encoding')
        machine = struct.unpack_from('<H', data, 18)[0]
        require(machine in (62, 183), 'Unexpected ELF architecture')
        return ['x86_64' if machine == 62 else 'aarch64']
    if data[:4] == b'\xcf\xfa\xed\xfe':
        require(len(data) >= 32, 'Truncated Mach-O')
        cpu = struct.unpack_from('<I', data, 4)[0]
        require(cpu in (0x01000007, 0x0100000c), 'Unexpected Mach-O architecture')
        return ['x86_64' if cpu == 0x01000007 else 'aarch64']
    if data[:4] in (b'\xca\xfe\xba\xbe', b'\xca\xfe\xba\xbf'):
        require(len(data) >= 8, 'Truncated fat Mach-O')
        count = struct.unpack_from('>I', data, 4)[0]; width = 32 if data[:4] == b'\xca\xfe\xba\xbf' else 20
        require(count == 2 and len(data) >= 8 + width * count, 'Expected exactly two universal slices')
        cpus = [struct.unpack_from('>I', data, 8 + width * i)[0] for i in range(count)]
        require(set(cpus) == {0x01000007, 0x0100000c}, 'Incorrect universal architectures')
        return ['aarch64', 'x86_64']
    raise ValueError('Unknown executable format')


def native_version(binary, version, lane):
    result = subprocess.run([str(binary), '--version'], env=environment(lane), capture_output=True,
                            text=True, timeout=20, check=False)
    require(result.returncode == 0 and result.stdout.strip() == 'terraphim-lsp ' + version,
            'Native binary version mismatch')
    return {'status': 'PASS', 'stdout': result.stdout, 'stderr': result.stderr, 'exitCode': result.returncode}


SMOKE_FILES = ('input.md', 'brief.md', 'brief.knowledge-graph.json', 'role.json', 'transcript.json', 'stderr.log')
SMOKE_SCOPE = 'Headless protocol client; explicit alternative input, actual server edit, save/reopen and graph export; no editor UI claim.'


def apply_single_edit(text, change, uri, version):
    require(isinstance(change, dict) and change.get('textDocument') == {'uri': uri, 'version': version}, 'Unexpected source edit document')
    edits = change.get('edits')
    require(isinstance(edits, list) and len(edits) == 1 and isinstance(edits[0], dict), 'Expected one source edit')
    edit = edits[0]
    require(isinstance(edit.get('newText'), str), 'Missing source edit text')
    def offset(position):
        require(isinstance(position, dict) and type(position.get('line')) is int and type(position.get('character')) is int, 'Invalid edit position')
        line, character = position['line'], position['character']
        lines = text.splitlines(keepends=True)
        if text.endswith('\n') or not lines: lines.append('')
        require(0 <= line < len(lines) and 0 <= character <= len(lines[line].encode('utf-16-le')) // 2, 'Edit position out of bounds')
        prefix = lines[line].encode('utf-16-le')[:character * 2].decode('utf-16-le')
        return sum(map(len, lines[:line])) + len(prefix)
    start, end = offset(edit['range']['start']), offset(edit['range']['end'])
    require(start <= end, 'Reversed source edit')
    return text[:start] + edit['newText'] + text[end:]


def validate_saved_smoke(folder, record, binary_hash):
    require(isinstance(record, dict) and record.get('status') == 'PASS' and record.get('binarySha256') == binary_hash
            and record.get('scope') == SMOKE_SCOPE and record.get('exitCode') == 0, 'Missing actual binary protocol evidence')
    require(folder.is_dir() and not folder.is_symlink() and {p.name for p in folder.iterdir()} == set(SMOKE_FILES), 'Unexpected smoke files')
    files = [{'name': n, 'sha256': publisher.digest(folder / n), 'bytes': (folder / n).stat().st_size} for n in SMOKE_FILES]
    require(all(not (folder / n).is_symlink() for n in SMOKE_FILES) and record.get('files') == files, 'Saved smoke bytes differ')
    require((folder / 'input.md').read_text() == 'Risk: delivery may slip.\n', 'Smoke input differs')
    source = (folder / 'brief.md').read_text()
    require(source.startswith((folder / 'input.md').read_text()) and 'Concern' in source, 'Authored saved source differs')
    require(record.get('sourceBeforeExport') == record.get('sourceAfterExport') == publisher.digest(folder / 'brief.md'), 'Export changed source')
    role = publisher.read_json(folder / 'role.json')
    require(role == {'name': 'release-smoke-empty-role', 'data': {}}, 'Smoke role was prepopulated')
    graph = publisher.read_json(folder / 'brief.knowledge-graph.json')
    data = graph.get('data') if isinstance(graph, dict) else None
    require(isinstance(data, dict) and isinstance(data.get('risk'), dict) and isinstance(data.get('concern'), dict), 'Missing saved synonyms')
    require(type(data['risk'].get('id')) is int and data['risk']['id'] == data['concern'].get('id')
            and data['concern'].get('nterm') == data['risk'].get('nterm') == 'risk', 'Saved synonym registration differs')
    require(record.get('registeredConceptId') == data['risk']['id'], 'Synonym receipt differs')
    transcript = publisher.read_json(folder / 'transcript.json')
    require(isinstance(transcript, list) and any(x.get('direction') == 'received' and x.get('message', {}).get('result', {}).get('written') is True
            for x in transcript if isinstance(x, dict) and isinstance(x.get('message'), dict) and isinstance(x['message'].get('result'), dict)), 'Missing actual export response')
    return graph


def native_protocol(binary, lane, folder):
    # The alternative is authored through the public LSP protocol, not inserted
    # into the expected graph. All exported bytes come from workspace/applyEdit.
    folder.mkdir(parents=True, exist_ok=False)
    source = folder / 'brief.md'; target = folder / 'brief.knowledge-graph.json'
    body = 'Risk: delivery may slip.\n'; source.write_text(body); (folder / 'input.md').write_text(body)
    publisher.write_new(folder / 'role.json', {'name': 'release-smoke-empty-role', 'data': {}})
    transcript = []; inbox = queue.Queue(); process = None
    def read_messages():
        try:
            for _ in range(128):
                headers = {}; total = 0
                while True:
                    line = process.stdout.readline(4097); total += len(line)
                    require(line and total <= 8192, 'Invalid LSP header')
                    if line == b'\r\n': break
                    key, value = line.decode('ascii').split(':', 1); headers[key.lower()] = value.strip()
                length = int(headers.get('content-length', '0'))
                require(0 < length <= 1048576, 'Invalid LSP body length')
                raw = process.stdout.read(length); require(len(raw) == length, 'Truncated LSP body')
                message = json.loads(raw); require(isinstance(message, dict), 'Malformed LSP message')
                inbox.put(message)
            raise ValueError('LSP message bound exceeded')
        except (ValueError, OSError) as error: inbox.put(error)
    def send(message):
        message = {'jsonrpc': '2.0', **message}; transcript.append({'direction': 'sent', 'message': message})
        raw = json.dumps(message).encode(); process.stdin.write(('Content-Length: %d\r\n\r\n' % len(raw)).encode() + raw); process.stdin.flush()
    def result(request_id):
        deadline = time.monotonic() + 20
        while True:
            try: message = inbox.get(timeout=max(0.01, deadline-time.monotonic()))
            except queue.Empty as error: raise ValueError('LSP response timed out') from error
            if isinstance(message, Exception): raise message
            transcript.append({'direction': 'received', 'message': message})
            if message.get('id') == request_id and 'method' not in message:
                require('error' not in message, 'LSP command returned an error'); return message.get('result')
            if message.get('method') == 'workspace/applyEdit':
                changes = message['params']['edit']['documentChanges']
                require(isinstance(changes, list) and len(changes) == 2, 'Unexpected export operations')
                create, edit = changes
                require(create.get('kind') == 'create' and create.get('uri') == target.as_uri()
                        and create.get('options', {}).get('overwrite') is False and not target.exists(), 'Unsafe export destination')
                require(edit.get('textDocument', {}).get('uri') == target.as_uri() and len(edit.get('edits', [])) == 1, 'Unexpected graph edit')
                addition = edit['edits'][0]
                require(addition.get('range') == {'start': {'line': 0, 'character': 0}, 'end': {'line': 0, 'character': 0}}
                        and isinstance(addition.get('newText'), str), 'Unexpected graph edit range')
                with target.open('x') as output: output.write(addition['newText'])
                send({'id': message['id'], 'result': {'applied': True}})
            elif message.get('method') == 'window/showDocument':
                require(message.get('params', {}).get('uri') == target.as_uri(), 'Unexpected shown document')
                send({'id': message['id'], 'result': {'success': False}})  # Headless client has no UI.
            elif 'id' in message and 'method' in message:
                raise ValueError('Unexpected LSP client request')
            require(time.monotonic() < deadline, 'LSP response timed out')
    try:
        with (folder / 'stderr.log').open('xb') as error_log:
            process = subprocess.Popen([str(binary), '--thesaurus', str(folder / 'role.json')], stdin=subprocess.PIPE,
                                       stdout=subprocess.PIPE, stderr=error_log, env=environment(lane))
            reader = threading.Thread(target=read_messages, daemon=True); reader.start()
            send({'id': 1, 'method': 'initialize', 'params': {'processId': None, 'rootUri': None, 'capabilities': {
                'workspace': {'applyEdit': True, 'workspaceEdit': {'documentChanges': True, 'resourceOperations': ['create']}},
                'window': {'showDocument': {'support': True}}}}})
            initialized = result(1)
            require('terraphim.alternative.add' in initialized['capabilities']['executeCommandProvider']['commands'], 'Missing alternative command')
            send({'method': 'initialized', 'params': {}})
            send({'method': 'textDocument/didOpen', 'params': {'textDocument': {'uri': source.as_uri(), 'languageId': 'markdown', 'version': 1, 'text': body}}})
            send({'id': 2, 'method': 'workspace/executeCommand', 'params': {'command': 'terraphim.alternative.add', 'arguments': [{
                'uri': source.as_uri(), 'version': 1, 'range': {'start': {'line': 0, 'character': 0}, 'end': {'line': 0, 'character': 4}}, 'text': 'Concern'}]}})
            edits = result(2)['documentChanges']; require(len(edits) == 1, 'Unexpected alternative edit count')
            authored = apply_single_edit(body, edits[0], source.as_uri(), 1)
            require(authored.startswith(body), 'Alternative edit changed body'); source.write_text(authored)
            send({'method': 'textDocument/didChange', 'params': {'textDocument': {'uri': source.as_uri(), 'version': 2}, 'contentChanges': [{'text': authored}]}})
            send({'method': 'textDocument/didClose', 'params': {'textDocument': {'uri': source.as_uri()}}})
            send({'method': 'textDocument/didOpen', 'params': {'textDocument': {'uri': source.as_uri(), 'languageId': 'markdown', 'version': 3, 'text': source.read_text()}}})
            before = publisher.digest(source)
            send({'id': 3, 'method': 'workspace/executeCommand', 'params': {'command': 'terraphim.export_knowledge_graph', 'arguments': [{'uri': source.as_uri(), 'version': 3}]}})
            exported = result(3); require(exported.get('written') is True and exported.get('uri') == target.as_uri(), 'Graph export was not saved')
            graph = publisher.read_json(target); require(exported.get('graph') == graph, 'Saved graph differs from server result')
            send({'id': 4, 'method': 'shutdown'}); result(4)
            send({'method': 'exit'}); process.stdin.close(); process.wait(timeout=10)
            require(process.returncode == 0, 'LSP did not exit cleanly')
        publisher.write_new(folder / 'transcript.json', transcript)
        record = {'status': 'PASS', 'binarySha256': publisher.digest(binary), 'scope': SMOKE_SCOPE, 'exitCode': process.returncode,
                  'sourceBeforeExport': before, 'sourceAfterExport': publisher.digest(source), 'registeredConceptId': graph['data']['risk']['id'],
                  'files': [{'name': n, 'sha256': publisher.digest(folder / n), 'bytes': (folder / n).stat().st_size} for n in SMOKE_FILES]}
        validate_saved_smoke(folder, record, publisher.digest(binary)); return record
    finally:
        if process is not None:
            if process.poll() is None: process.kill(); process.wait(timeout=5)
            if process.stdout: process.stdout.close()
            if process.stdin and not process.stdin.closed: process.stdin.close()
        if not (folder / 'transcript.json').exists(): publisher.write_new(folder / 'transcript.json', transcript)


def stage_binary(binary, target, lane, expected, commands, extra=None):
    require(target in (*TARGETS, 'universal-apple-darwin'), 'Unsupported target')
    require(binary.is_file() and not binary.is_symlink(), 'Missing regular binary')
    arches = architecture(binary)
    wanted = ['aarch64', 'x86_64'] if target == 'universal-apple-darwin' else [target.split('-')[0]]
    require(arches == wanted, 'Binary architecture differs from target')
    output = lane / 'dist'; output.mkdir(exist_ok=True)
    name = 'terraphim-lsp-' + target; staged = output / name
    require(not staged.exists(), 'Existing binary is immutable')
    shutil.copyfile(binary, staged); staged.chmod(0o755)
    runnable = (platform.system() == 'Darwin' and target.endswith('apple-darwin')
                and (target.startswith({'arm64': 'aarch64', 'x86_64': 'x86_64'}.get(platform.machine(), 'unsupported')) or target.startswith('universal'))
                or platform.system() == 'Linux' and target.startswith('x86_64-unknown-linux') and platform.machine() == 'x86_64')
    smoke = native_version(staged, expected['version'], lane) if runnable else {'status': 'NOT_RUN_CROSS_TARGET'}
    protocol = native_protocol(staged, lane, output / 'smoke' / target) if runnable else {'status': 'NOT_RUN_CROSS_TARGET'}
    log_dir = output / 'logs' / target; log_dir.mkdir(parents=True)
    for record in commands:
        shutil.copyfile(lane / 'logs' / record['log'], log_dir / record['log'])
    receipt, _ = source_receipt(lane, expected)
    record = {'schema': 1, 'status': 'PASS', 'source': receipt, 'target': target,
              'asset': {'name': name, 'sha256': publisher.digest(staged), 'bytes': staged.stat().st_size},
              'architectures': arches, 'host': {'system': platform.system(), 'machine': platform.machine()}, 'nativeVersion': smoke, 'nativeProtocol': protocol, 'commands': commands, 'extra': extra or {}}
    publisher.write_new(output / (target + '.json'), record)
    return record


def build(lane, expected, target, cross_image=None):
    require(target in TARGETS, 'Unsupported target')
    _, source = source_receipt(lane, expected)
    cargo = ['rustup', 'run', expected['toolchain'], 'cargo']; commands = []
    commands.append(command(cargo + ['fetch', '--locked', '--target', target], source, lane, 'fetch-' + target))
    if cross_image:
        require(target.endswith('linux-musl') and re.fullmatch(r'ghcr\.io/cross-rs/' + re.escape(target) + r'@sha256:[0-9a-f]{64}', cross_image), 'Cross image must be the exact public target digest')
        config = lane / 'Cross.toml'; require(not config.exists(), 'Cross config already exists')
        config.write_text('[target.' + target + ']\nimage = "' + cross_image + '"\n')
        # Explicit config and digest avoid an inherited or moving build image.
        os.environ['CROSS_CONFIG'] = str(config)
        invocation = ['rustup', 'run', expected['toolchain'], 'cross']
    else:
        invocation = cargo
    commands.append(command(invocation + ['build', '--locked', '--offline', '--release', '--target', target, '--bin', 'terraphim-lsp'], source, lane, 'build-' + target))
    return stage_binary(lane / 'target' / target / 'release/terraphim-lsp', target, lane, expected, commands,
                        {'crossImage': cross_image, 'crossImageSelection': 'Runtime digest resolved from upstream 0.2.5 tag; the recorded digest is used for this build.'} if cross_image else {})


def universal(lane, expected):
    inputs = [lane / 'dist' / ('terraphim-lsp-' + t) for t in ('aarch64-apple-darwin', 'x86_64-apple-darwin')]
    require(all(p.is_file() for p in inputs), 'Universal inputs are missing')
    output = lane / 'universal'; cmd = ['lipo', '-create', *map(str, inputs), '-output', str(output)]
    record = command(cmd, lane, lane, 'universal')
    return stage_binary(output, 'universal-apple-darwin', lane, expected, [record],
                        {'inputHashes': {p.name: publisher.digest(p) for p in inputs}, 'signing': 'unsigned'})


def bundle(folder, qualification, expected, write=False):
    q = publisher.read_json(qualification / 'qualification.json')
    require(isinstance(q, dict) and q.get('status') == 'PASS' and isinstance(q.get('source'), dict) and q['source'].get('source') == expected, 'Qualification identity differs')
    require(isinstance(q.get('commands'), list) and all(isinstance(c, dict) for c in q['commands']), 'Malformed qualification commands')
    require([c.get('argv') for c in q['commands']] == [
        ['rustup', 'run', expected['toolchain'], 'cargo', 'clippy', '--locked', '--all-targets', '--', '-D', 'warnings'],
        ['rustup', 'run', expected['toolchain'], 'cargo', 'test', '--locked', '--all-targets'],
        ['rustup', 'run', expected['toolchain'], 'cargo', 'test', '--locked', '--doc']], 'Qualification commands differ')
    for c, name in zip(q['commands'], ('clippy.log', 'test.log', 'doctest.log')):
        require(c.get('log') == name, 'Qualification log path differs')
        log = qualification / 'logs' / c['log']
        require(c['exitCode'] == 0 and log.is_file() and not log.is_symlink() and publisher.digest(log) == c['sha256'], 'Qualification log differs')
    records = []
    for name in RAW:
        target = name.removeprefix('terraphim-lsp-'); path = folder / name
        record = publisher.read_json(folder / (target + '.json'))
        require(isinstance(record, dict) and record.get('schema') == 1 and record.get('status') == 'PASS' and record.get('source') == q['source']
                and record.get('target') == target and record.get('asset') == {'name': name, 'sha256': publisher.digest(path), 'bytes': path.stat().st_size}
                and not path.is_symlink() and architecture(path) == record.get('architectures'), 'Lane bytes/provenance differs')
        wanted = ['aarch64', 'x86_64'] if target.startswith('universal') else [target.split('-')[0]]
        require(record['architectures'] == wanted, 'Lane target architecture differs')
        smoke = record.get('nativeVersion'); host = record.get('host')
        require(isinstance(smoke, dict) and isinstance(host, dict), 'Missing native-version evidence')
        if target in ('x86_64-unknown-linux-gnu', 'x86_64-unknown-linux-musl', 'aarch64-apple-darwin', 'universal-apple-darwin'):
            require(smoke.get('status') == 'PASS' and smoke.get('exitCode') == 0
                    and smoke.get('stdout', '').strip() == 'terraphim-lsp ' + expected['version'], 'Required native version did not run')
        else:
            require(smoke.get('status') in {'PASS', 'NOT_RUN_CROSS_TARGET'}, 'Missing cross-target qualification scope')
        protocol = record.get('nativeProtocol')
        if smoke.get('status') == 'PASS':
            validate_saved_smoke(folder / 'smoke' / target, protocol, publisher.digest(path))
        else:
            require(protocol == {'status': 'NOT_RUN_CROSS_TARGET'}, 'Unexecuted target has misleading protocol evidence')
        commands = record.get('commands')
        require(isinstance(commands, list) and commands, 'Missing executed build commands')
        for command_record in commands:
            require(isinstance(command_record, dict) and isinstance(command_record.get('log'), str)
                    and re.fullmatch(r'[a-z0-9-]+\.log', command_record['log']), 'Invalid build log name')
            log = folder / 'logs' / target / command_record['log']
            require(command_record.get('exitCode') == 0 and log.is_file() and not log.is_symlink()
                    and publisher.digest(log) == command_record.get('sha256'), 'Build log differs')
        extra = record.get('extra')
        require(isinstance(extra, dict), 'Malformed lane extra metadata')
        if target.startswith('universal'):
            require(extra.get('inputHashes') == {
                'terraphim-lsp-' + t: publisher.digest(folder / ('terraphim-lsp-' + t))
                for t in ('aarch64-apple-darwin', 'x86_64-apple-darwin')}, 'Universal input bytes differ')
        else:
            build_command = commands[-1].get('argv', [])
            require(build_command[:3] == ['rustup', 'run', expected['toolchain']]
                    and build_command[3:4] in (['cargo'], ['cross'])
                    and build_command[4:] == ['build', '--locked', '--offline', '--release', '--target', target, '--bin', 'terraphim-lsp'], 'Build target/command differs')
        records.append(record)
    expected_names = {'logs', 'smoke'} | set(RAW) | {r['target'] + '.json' for r in records}
    manifest = {'schema': 2, 'component': 'terraphim-lsp', 'source': expected,
                'registrySource': q['source'], 'qualification': q, 'lanes': records,
                'universalMacosSigning': 'unsigned', 'assets': publisher.asset_records([folder / n for n in RAW])}
    sums = ''.join(publisher.digest(folder / n) + '  ' + n + '\n' for n in RAW)
    if write:
        require({p.name for p in folder.iterdir()} == expected_names, 'Unexpected staged files')
        (folder / 'checksums.txt').write_text(sums)
        publisher.write_new(folder / publisher.MANIFEST, manifest)
    else:
        require({p.name for p in folder.iterdir()} == expected_names | {'checksums.txt', publisher.MANIFEST}, 'Unexpected release files')
        require((folder / 'checksums.txt').read_text() == sums and publisher.read_json(folder / publisher.MANIFEST) == manifest, 'Frozen manifest/checksums differ')
    return [folder / name for name in publisher.RELEASE_FILES]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('prepare', 'qualify', 'build', 'universal', 'assemble', 'publish'))
    parser.add_argument('--root', type=Path, default=ROOT); parser.add_argument('--lane', type=Path)
    parser.add_argument('--tag', required=True); parser.add_argument('--source', required=True); parser.add_argument('--toolchain', required=True)
    parser.add_argument('--target', choices=TARGETS); parser.add_argument('--cross-image')
    parser.add_argument('--assets', type=Path); parser.add_argument('--qualification', type=Path); parser.add_argument('--receipt', type=Path)
    args = parser.parse_args(); result = {'schema': 1, 'status': 'FAIL'}
    try:
        expected, item = identity(args.root, args.tag, args.source, args.toolchain)
        if args.command == 'prepare': prepare(args.root, args.lane, expected, item)
        elif args.command == 'qualify': qualify(args.lane, expected)
        elif args.command == 'build': build(args.lane, expected, args.target, args.cross_image)
        elif args.command == 'universal': universal(args.lane, expected)
        elif args.command == 'assemble': bundle(args.assets, args.qualification, expected, write=True)
        else:
            require(args.receipt is not None and not args.receipt.exists(), 'Choose a new publication receipt')
            bundle(args.assets, args.qualification, expected)
            with tempfile.TemporaryDirectory(prefix='lsp-public-frozen-') as tmp:
                registry, _ = prepare(args.root, Path(tmp) / 'registry-readback', expected, item)
                require(registry == publisher.read_json(args.qualification / 'qualification.json')['source'], 'Fresh registry archive differs from qualification')
                frozen = Path(tmp) / 'assets'; shutil.copytree(args.assets, frozen, symlinks=True)
                files = bundle(frozen, args.qualification, expected)
                result = publisher.publish(publisher.REPOSITORY, args.tag, files, args.source,
                    'Built and qualified from the published crates.io archive. The manifest records source, registry checksum, lock, target checks and test logs. macOS binaries are unsigned. Cross-target runtime checks are marked explicitly.')
            publisher.write_new(args.receipt, result)
        return 0
    except (ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError) as error:
        if args.lane and args.lane.is_dir() and not (args.lane / 'failure.json').exists():
            publisher.write_new(args.lane / 'failure.json', {**result, 'command': args.command, 'error': str(error)})
        if args.receipt and not args.receipt.exists():
            publisher.write_new(args.receipt, {**result, 'error': str(error)})
        print('lsp_public_binary: ' + str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
