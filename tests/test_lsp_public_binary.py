"""Public-binary guard tests: real archives, filesystem bytes and subprocesses."""
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('binary', ROOT / 'scripts/lsp_public_binary.py')
b = importlib.util.module_from_spec(spec); spec.loader.exec_module(b)


class PublicBinaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.expected = {'repository': 'https://github.com/terraphim/terraphim-ai', 'commit': 'a' * 40,
                         'tag': 'terraphim_lsp-v1.22.0-rc.4', 'version': '1.22.0-rc.4', 'toolchain': '1.96'}
        self.item = {'name': 'terraphim_lsp', 'version': '1.22.0-rc.4', 'path': 'crates/terraphim_lsp/Cargo.toml'}

    def lock(self, source=b.REGISTRY):
        return 'version=4\n[[package]]\nname="terraphim_lsp"\nversion="1.22.0-rc.4"\n[[package]]\nname="serde"\nversion="1.0.0"\nsource="'+source+'"\nchecksum="'+'b'*64+'"\n'+''.join('[[package]]\nname="'+name+'"\nversion="'+version+'"\nsource="'+source+'"\nchecksum="'+'c'*64+'"\n' for name,version in b.OWNED_DEPENDENCIES.items())

    def lane(self):
        lane = self.root / 'lane'; lane.mkdir()
        prefix = self.item['name'] + '-' + self.item['version']; path = lane / 'source.crate'
        content = {'Cargo.toml': '[package]\nname="terraphim_lsp"\nversion="1.22.0-rc.4"\n',
                   '.cargo_vcs_info.json': json.dumps({'git': {'sha1': self.expected['commit']}, 'path_in_vcs': 'crates/terraphim_lsp'}),
                   'Cargo.lock': self.lock(), 'src/main.rs': 'fn main() {}'}
        with tarfile.open(path, 'w:gz') as archive:
            for name, text in content.items():
                info = tarfile.TarInfo(prefix + '/' + name); raw = text.encode(); info.size = len(raw)
                archive.addfile(info, io.BytesIO(raw))
        b.crates.unpack_package(path, self.item, self.expected['commit'], lane/'unpacked', self.root/'checkout')
        receipt = {'schema': 1, 'source': self.expected, 'package': self.item,
                   'archive': b.crates.inspect_package(path,self.item,self.expected['commit']),
                   'registryChecksum': b.publisher.digest(path), 'downloadUrl': 'https://static.crates.io/crates/terraphim_lsp/test.crate',
                   'lock': b.public_lock(self.lock(),self.item)}
        b.publisher.write_new(lane/'source.json',receipt)
        return lane, receipt

    def executable(self, target):
        if target.startswith('universal'):
            return b'\xca\xfe\xba\xbe'+struct.pack('>I',2)+struct.pack('>IIIII',0x01000007,0,48,32,0)+struct.pack('>IIIII',0x0100000c,0,80,32,0)+b'\0'*64
        if target.endswith('apple-darwin'):
            return b'\xcf\xfa\xed\xfe'+struct.pack('<I',0x0100000c if target.startswith('aarch64') else 0x01000007)+b'\0'*24
        value=bytearray(64);value[:6]=b'\x7fELF\x02\x01';struct.pack_into('<H',value,18,183 if target.startswith('aarch64') else 62)
        return bytes(value)

    def fixture_bundle(self):
        # These are explicit format fixtures, never a claim of compiled LSP output.
        lane, source = self.lane(); q=self.root/'qualification';q.mkdir();(q/'logs').mkdir()
        commands=[]
        for name,args in [('clippy',['clippy','--locked','--all-targets','--','-D','warnings']),('test',['test','--locked','--all-targets']),('doctest',['test','--locked','--doc'])]:
            log=q/'logs'/(name+'.log');log.write_text('Fixture evidence\n')
            commands.append({'argv':['rustup','run','1.96','cargo',*args],'exitCode':0,'log':log.name,'sha256':b.publisher.digest(log)})
        b.publisher.write_new(q/'qualification.json',{'schema':1,'status':'PASS','source':source,'commands':commands})
        folder=self.root/'dist';folder.mkdir()
        for name in b.RAW:
            target=name.removeprefix('terraphim-lsp-');path=folder/name;path.write_bytes(self.executable(target))
        for name in b.RAW:
            target=name.removeprefix('terraphim-lsp-');path=folder/name;logs=folder/'logs'/target;logs.mkdir(parents=True)
            log=logs/'build.log';log.write_text('Fixture build log\n')
            argv=['rustup','run','1.96','cargo','build','--locked','--offline','--release','--target',target,'--bin','terraphim-lsp']
            extra={}
            if target.startswith('universal'):
                argv=['lipo','-create'];extra={'inputHashes':{n:b.publisher.digest(folder/n) for n in b.RAW if n in ('terraphim-lsp-aarch64-apple-darwin','terraphim-lsp-x86_64-apple-darwin')}}
            record={'schema':1,'status':'PASS','source':source,'target':target,'asset':{'name':name,'sha256':b.publisher.digest(path),'bytes':path.stat().st_size},'architectures':b.architecture(path),'host':{'system':'Fixture','machine':'Fixture'},'nativeVersion':{'status':'PASS','exitCode':0,'stdout':'terraphim-lsp 1.22.0-rc.4\n'},'commands':[{'argv':argv,'exitCode':0,'log':log.name,'sha256':b.publisher.digest(log)}],'extra':extra}
            smoke=folder/'smoke'/target;smoke.mkdir(parents=True)
            (smoke/'input.md').write_text('Risk: delivery may slip.\n')
            (smoke/'brief.md').write_text('Risk: delivery may slip.\nFixture annotation: Concern\n')
            b.publisher.write_new(smoke/'role.json',{'name':'release-smoke-empty-role','data':{}})
            b.publisher.write_new(smoke/'brief.knowledge-graph.json',{'data':{'risk':{'id':1,'nterm':'risk'},'concern':{'id':1,'nterm':'risk'}}})
            b.publisher.write_new(smoke/'transcript.json',[{'direction':'received','message':{'result':{'written':True}}}])
            (smoke/'stderr.log').write_text('Fixture protocol evidence\n')
            record['nativeProtocol']={'status':'PASS','binarySha256':b.publisher.digest(path),'scope':b.SMOKE_SCOPE,'exitCode':0,'sourceBeforeExport':b.publisher.digest(smoke/'brief.md'),'sourceAfterExport':b.publisher.digest(smoke/'brief.md'),'registeredConceptId':1,'files':[{'name':n,'sha256':b.publisher.digest(smoke/n),'bytes':(smoke/n).stat().st_size} for n in b.SMOKE_FILES]}
            b.publisher.write_new(folder/(target+'.json'),record)
        return folder,q

    def test_registry_only_lock_and_checksums(self):
        self.assertEqual(b.public_lock(self.lock(),self.item)['packageCount'],2+len(b.OWNED_DEPENDENCIES))
        for source in ['git+https://github.com/terraphim/core#abc','registry+https://git.terraphim.cloud/api/packages/terraphim/cargo']:
            with self.subTest(source=source),self.assertRaises(ValueError):b.public_lock(self.lock(source),self.item)
        with self.assertRaises(ValueError):b.public_lock(self.lock().replace('checksum=', 'other='),self.item)
        with self.assertRaises(ValueError):b.public_lock(self.lock()+'[[package]]\nname="private"\nversion="1"\n',self.item)

    def test_published_archive_source_and_extracted_bytes_are_rechecked(self):
        lane,_=self.lane();_,source=b.source_receipt(lane,self.expected)
        (source/'src/main.rs').write_text('changed')
        with self.assertRaisesRegex(ValueError,'bytes changed'):b.source_receipt(lane,self.expected)

    def test_extra_extracted_configuration_is_rejected(self):
        lane,_=self.lane();_,source=b.source_receipt(lane,self.expected)
        (source/'.cargo').mkdir();(source/'.cargo/config.toml').write_text('[source.crates-io]\nreplace-with="private"')
        with self.assertRaisesRegex(ValueError,'file set'):b.source_receipt(lane,self.expected)

    def test_real_subprocess_log_is_saved_and_hash_bound(self):
        lane=self.root/'command';lane.mkdir()
        record=b.command([sys.executable,'-c','print("actual subprocess output")'],lane,lane,'real-command')
        self.assertEqual(record['exitCode'],0)
        self.assertEqual((lane/'logs/real-command.log').read_text(),'actual subprocess output\n')
        self.assertEqual(record['sha256'],b.publisher.digest(lane/'logs/real-command.log'))
        with self.assertRaisesRegex(ValueError,'failed'):
            b.command([sys.executable,'-c','raise SystemExit(4)'],lane,lane,'real-failure')
        self.assertTrue((lane/'logs/real-failure.log').is_file())

    def test_cross_environment_omits_host_build_dir_without_changing_native(self):
        lane = self.root / 'lane'
        previous = os.environ.get('CARGO_BUILD_BUILD_DIR')
        try:
            os.environ['CARGO_BUILD_BUILD_DIR'] = '/unrelated/inherited/build'
            native = b.environment(lane)
            cross = b.environment(lane, for_cross=True)
        finally:
            if previous is None:
                os.environ.pop('CARGO_BUILD_BUILD_DIR', None)
            else:
                os.environ['CARGO_BUILD_BUILD_DIR'] = previous
        self.assertEqual(native['CARGO_BUILD_BUILD_DIR'], str(lane / 'target'))
        self.assertNotIn('CARGO_BUILD_BUILD_DIR', cross)
        self.assertEqual(cross['CARGO_TARGET_DIR'], str(lane / 'target'))
        self.assertEqual({k: v for k, v in native.items() if k != 'CARGO_BUILD_BUILD_DIR'}, cross)

    def test_actual_command_child_receives_cross_specific_environment(self):
        lane = self.root / 'child-env'; lane.mkdir()
        command = [sys.executable, '-c',
                   'import json,os; print(json.dumps({k:os.environ.get(k) for k in '
                   '["CARGO_BUILD_BUILD_DIR","CARGO_TARGET_DIR"]}))']
        b.command(command, lane, lane, 'native-env')
        b.command(command, lane, lane, 'cross-env', for_cross=True)
        native = json.loads((lane / 'logs/native-env.log').read_text())
        cross = json.loads((lane / 'logs/cross-env.log').read_text())
        self.assertEqual(native['CARGO_BUILD_BUILD_DIR'], str(lane / 'target'))
        self.assertIsNone(cross['CARGO_BUILD_BUILD_DIR'])
        self.assertEqual(cross['CARGO_TARGET_DIR'], native['CARGO_TARGET_DIR'])

    def test_architecture_refuses_unknown_and_mismatched_format(self):
        for target in (*b.TARGETS,'universal-apple-darwin'):
            path=self.root/target;path.write_bytes(self.executable(target));self.assertTrue(b.architecture(path))
        path=self.root/'invalid';path.write_bytes(b'not a binary')
        with self.assertRaises(ValueError):b.architecture(path)
        path.write_bytes(b'\x7fELF')
        with self.assertRaises(ValueError):b.architecture(path)

    def test_real_compiled_native_version_is_executed(self):
        compiler=shutil.which('cc')
        if compiler is None:self.skipTest('Native C compiler not installed')
        source=self.root/'fixture.c';source.write_text('#include <stdio.h>\nint main(void){puts("terraphim-lsp 1.22.0-rc.4");return 0;}\n')
        binary=self.root/'fixture';subprocess.run([compiler,str(source),'-o',str(binary)],check=True,capture_output=True)
        self.assertTrue(b.architecture(binary))
        self.assertEqual(b.native_version(binary,'1.22.0-rc.4',self.root)['stdout'],'terraphim-lsp 1.22.0-rc.4\n')
        with self.assertRaises(ValueError):b.native_version(binary,'wrong',self.root)

    def test_bundle_round_trip_preserves_six_raw_names(self):
        folder,q=self.fixture_bundle();paths=b.bundle(folder,q,self.expected,write=True)
        self.assertEqual({p.name for p in paths},set(b.publisher.RELEASE_FILES))
        self.assertEqual(b.bundle(folder,q,self.expected),paths)

    def test_mutated_asset_and_extra_file_refused(self):
        folder,q=self.fixture_bundle();b.bundle(folder,q,self.expected,write=True)
        (folder/b.RAW[0]).write_bytes(b'changed')
        with self.assertRaises(ValueError):b.bundle(folder,q,self.expected)

    def test_changed_build_log_and_wrong_target_refused(self):
        folder,q=self.fixture_bundle();target=b.RAW[0].removeprefix('terraphim-lsp-')
        log=folder/'logs'/target/'build.log';log.write_text('changed')
        with self.assertRaisesRegex(ValueError,'Build log'):b.bundle(folder,q,self.expected,write=True)

    def test_skipped_native_test_and_changed_source_refused(self):
        folder,q=self.fixture_bundle();p=folder/'x86_64-unknown-linux-gnu.json';record=json.loads(p.read_text());record['nativeVersion']={'status':'NOT_RUN_CROSS_TARGET'};p.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'Required native'):b.bundle(folder,q,self.expected,write=True)

    def test_wrong_build_target_is_refused(self):
        folder,q=self.fixture_bundle();p=folder/'aarch64-unknown-linux-musl.json';record=json.loads(p.read_text())
        record['commands'][0]['argv'][-3]='x86_64-unknown-linux-musl';p.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'Build target'):b.bundle(folder,q,self.expected,write=True)

    def test_extra_asset_is_refused(self):
        folder,q=self.fixture_bundle();(folder/'unexpected.zip').write_bytes(b'x')
        with self.assertRaisesRegex(ValueError,'Unexpected staged'):b.bundle(folder,q,self.expected,write=True)

    def test_wrong_lane_source_and_malformed_metadata_are_refused(self):
        folder,q=self.fixture_bundle();p=folder/'aarch64-unknown-linux-musl.json';record=json.loads(p.read_text())
        record['source']['source']['commit']='b'*40;p.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'provenance'):b.bundle(folder,q,self.expected,write=True)
        p.write_text('null')
        with self.assertRaises(ValueError):b.bundle(folder,q,self.expected,write=True)
        (q/'qualification.json').write_text('[]')
        with self.assertRaises(ValueError):b.bundle(folder,q,self.expected,write=True)

    def test_universal_must_bind_actual_input_bytes(self):
        folder,q=self.fixture_bundle();p=folder/'universal-apple-darwin.json';record=json.loads(p.read_text());record['extra']['inputHashes']={};p.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'Universal input'):b.bundle(folder,q,self.expected,write=True)

    def test_qualification_log_traversal_refused(self):
        folder,q=self.fixture_bundle();p=q/'qualification.json';record=json.loads(p.read_text());record['commands'][0]['log']='../../outside';p.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'log path'):b.bundle(folder,q,self.expected,write=True)

    def test_owned_published_dependency_versions_and_duplicates_refused(self):
        for name, version in b.OWNED_DEPENDENCIES.items():
            with self.subTest(name=name),self.assertRaisesRegex(ValueError,'Owned dependency'):
                b.public_lock(self.lock().replace('name="'+name+'"\nversion="'+version+'"','name="'+name+'"\nversion="9.9.9"'),self.item)
        with self.assertRaisesRegex(ValueError,'Owned dependency'):
            b.public_lock(self.lock()+'[[package]]\nname="terraphim_types"\nversion="1.22.3"\nsource="'+b.REGISTRY+'"\nchecksum="'+'d'*64+'"\n',self.item)

    def test_saved_graph_corruption_refused(self):
        folder,q=self.fixture_bundle();target='x86_64-unknown-linux-gnu'
        (folder/'smoke'/target/'brief.knowledge-graph.json').write_text('{}')
        with self.assertRaisesRegex(ValueError,'smoke bytes'):b.bundle(folder,q,self.expected,write=True)

    def test_false_synonym_receipt_with_updated_file_hash_refused(self):
        folder,q=self.fixture_bundle();target='x86_64-unknown-linux-gnu';p=folder/(target+'.json')
        record=json.loads(p.read_text());smoke=folder/'smoke'/target
        graph=json.loads((smoke/'brief.knowledge-graph.json').read_text());graph['data']['concern']['id']=2
        (smoke/'brief.knowledge-graph.json').write_text(json.dumps(graph))
        record['nativeProtocol']['files']=[{'name':n,'sha256':b.publisher.digest(smoke/n),'bytes':(smoke/n).stat().st_size} for n in b.SMOKE_FILES]
        p.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'synonym registration'):b.bundle(folder,q,self.expected,write=True)

    def test_null_qualification_commands_are_refused_cleanly(self):
        folder,q=self.fixture_bundle();path=q/'qualification.json';record=json.loads(path.read_text())
        record['commands']=[None];path.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'Malformed qualification'):b.bundle(folder,q,self.expected,write=True)

    def test_source_edit_scope_and_utf16_are_validated(self):
        edit={'textDocument':{'uri':'file:///brief.md','version':1},'edits':[{'range':{'start':{'line':0,'character':2},'end':{'line':0,'character':3}},'newText':'Z'}]}
        self.assertEqual(b.apply_single_edit('😀a',edit,'file:///brief.md',1),'😀Z')
        with self.assertRaises(ValueError):b.apply_single_edit('😀a',edit,'file:///other.md',1)
        edit['edits'][0]['range']['end']['character']=999
        with self.assertRaises(ValueError):b.apply_single_edit('😀a',edit,'file:///brief.md',1)

    @unittest.skipUnless(os.environ.get('LSP_SMOKE_BINARY'), 'Set LSP_SMOKE_BINARY to exercise a real compiled LSP; release CI runs this smoke on every native asset')
    def test_real_lsp_binary_authors_saves_reopens_and_exports_synonym(self):
        binary=Path(os.environ['LSP_SMOKE_BINARY']).resolve()
        record=b.native_protocol(binary,self.root,self.root/'real-protocol')
        graph=b.validate_saved_smoke(self.root/'real-protocol',record,b.publisher.digest(binary))
        self.assertEqual(graph['data']['risk']['id'],graph['data']['concern']['id'])
        self.assertIn('Concern',(self.root/'real-protocol/brief.md').read_text())

    def test_transport_unchanged_and_workflow_call_only(self):
        self.assertEqual(b.publisher.digest(ROOT/'.github/scripts/release/publish_lsp_release.py'),'c87fdaa79a4901b039bce51c6a440a42bd950d1079968b97de17dab4752016c0')
        text=(ROOT/'.github/workflows/lsp-publish.yml').read_text()
        self.assertIn('workflow_call:',text);self.assertNotIn('  push:',text)
        for forbidden in ('secrets.','GITEA_','lsp_handoff.py publish','--allow-dirty','--no-verify'):
            self.assertNotIn(forbidden,text)
        self.assertEqual(text.count('contents: write'),1)
        self.assertIn('needs: [qualify, linux, macos]',text)
        self.assertIn('environment: lsp-releases',text)
        for target in b.TARGETS:self.assertIn(target,text)


if __name__=='__main__':unittest.main()
