"""Exercise real VSIX ZIP bytes, extraction aliases and both manifests."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from verify_vsix import verify, check_tag
import legacy_assets

XML = '''<PackageManifest xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
<Metadata><Identity Id="terraphim-it" Publisher="applied-knowledge-systems" Version="0.1.0"/>
<Properties><Property Id="Microsoft.VisualStudio.Code.Engine" Value="^1.126.0"/>
<Property Id="Microsoft.VisualStudio.Code.PreRelease" Value="true"/></Properties></Metadata>
<Installation><InstallationTarget Id="Microsoft.VisualStudio.Code"/></Installation>
<Assets><Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json"/></Assets>
</PackageManifest>'''

class VsixTest(unittest.TestCase):
    manifest = {'name':'terraphim-it','publisher':'applied-knowledge-systems','version':'0.1.0','license':'Apache-2.0','engines':{'vscode':'^1.126.0'}}

    def package(self, root, changed=None):
        data = b'actual asset fixture'
        lock = json.dumps({'sourceHead':'source','assets':{'engine.wasm':hashlib.sha256(data).hexdigest()}}).encode()
        entries = {'extension/package.json':json.dumps(self.manifest).encode(),
                   'extension.vsixmanifest':XML.encode(),
                   'extension/media/editor-build.json':lock, 'extension/media/editor/host-assets.json':lock,
                   'extension/media/editor/engine.wasm':data}
        for name in ['out/extension.js','out/alternativeControl.js','out/checkpointStage.js','out/helpers/linkReplacement.js','media/knowledge-graph.cjs','media/export-intent.js','media/export-target.cjs','media/host-notice.js','media/history-checkpoint.js','media/alternative-control.js','LICENSE.txt','readme.md','node_modules/@tomic/lib/package.json']:
            entries['extension/'+name] = b'fixture'
        legacy_data = {"rust-lib/pkg/terraphim_it.js": b'package parser fixture',
                       "rust-lib/pkg/terraphim_it_bg.wasm": b'\x00asm\x01\x00\x00\x00'}
        legacy_lock = {"schema": 1, "rust": legacy_assets.RUST, "wasmPack": legacy_assets.WASM_PACK,
                       "target": "wasm32-unknown-unknown", "inputs": legacy_assets.inputs(legacy_assets.ROOT),
                       "assets": {name: hashlib.sha256(value).hexdigest() for name, value in legacy_data.items()}}
        entries["extension/media/legacy-build.json"] = json.dumps(legacy_lock).encode()
        for name, value in legacy_data.items():
            entries["extension/" + name] = value
        if changed:
            entries.update(changed)
        path = root / 'fixture.vsix'
        with zipfile.ZipFile(path,'w') as output:
            for name, value in entries.items():
                output.writestr(name,value)
        return path, lock

    def assert_rejected(self, changed):
        with tempfile.TemporaryDirectory() as temp:
            path, lock = self.package(Path(temp), changed)
            with self.assertRaises(ValueError):
                verify(path, self.manifest, lock)

    def test_exact_files_are_admitted(self):
        with tempfile.TemporaryDirectory() as temp:
            path, lock = self.package(Path(temp))
            self.assertEqual(verify(path,self.manifest,lock)['version'],'0.1.0')

    def test_corrupted_runtime_or_development_files_are_refused(self):
        for changed in [{'extension/media/editor/engine.wasm':b'changed'}, {'extension/tests/private.json':b'development'}, {'../escaped':b'escape'}]:
            self.assert_rejected(changed)

    def test_extraction_aliases_are_refused(self):
        for name in ['extension//package.json','extension/./package.json','extension\\package.json',
                     'extension/PACKAGE.JSON','extension/package.json.', 'extension/package.json ',
                     'extension/package.json:stream', '/extension/package.json']:
            self.assert_rejected({name:b'overwrites verified manifest'})
        self.assert_rejected({'extension/café.txt':b'first','extension/cafe\u0301.txt':b'alias'})

    def test_symlink_is_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            path, lock = self.package(Path(temp))
            link = zipfile.ZipInfo('extension/link')
            link.create_system = 3
            link.external_attr = 0o120777 << 16
            with zipfile.ZipFile(path, 'a') as archive:
                archive.writestr(link, '../../outside')
            with self.assertRaises(ValueError):
                verify(path,self.manifest,lock)

    def test_marketplace_xml_is_bound_to_json_and_preview_channel(self):
        for old,new in [('Publisher="applied-knowledge-systems"','Publisher="other"'),
                        ('Version="0.1.0"','Version="0.2.0"'),
                        ('Id="terraphim-it"','Id="different"'),
                        ('Id="Microsoft.VisualStudio.Code"','Id="OtherIDE"'),
                        ('Value="true"','Value="false"'),
                        ('Value="^1.126.0"','Value="^1.74.0"'),
                        ('Path="extension/package.json"','Path="extension/other.json"'),
                        ('Version="0.1.0"','Version="0.1.0" TargetPlatform="win32-x64"')]:
            self.assert_rejected({'extension.vsixmanifest':XML.replace(old,new).encode()})

    def test_missing_changed_or_wrong_source_link_module_is_refused(self):
        for changed in [{"extension/rust-lib/pkg/terraphim_it_bg.wasm": b"changed"},
                        {"extension/media/legacy-build.json": b"{}"}]:
            self.assert_rejected(changed)
        with tempfile.TemporaryDirectory() as temp:
            path, lock = self.package(Path(temp))
            wrong = dict(legacy_assets.inputs(legacy_assets.ROOT))
            wrong["rust-lib/src/lib.rs"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "source differs"):
                verify(path, self.manifest, lock, wrong)

    def test_tag_is_exact_and_immutable(self):
        check_tag('terraphim_vscode-v0.1.0','0.1.0')
        for tag in ['v0.1.0','terraphim_vscode-v0.2.0','terraphim_vscode-v0.1.0/extra']:
            with self.assertRaises(ValueError):
                check_tag(tag,'0.1.0')

if __name__ == '__main__':
    unittest.main()
