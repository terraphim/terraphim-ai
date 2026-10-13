"""Registry comparison uses real ZIP payloads; no network substitutes or remote writes."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import verify_registry as registry
import verify_vsix_test


class RegistryTest(unittest.TestCase):
    def test_outer_signing_and_zip_compression_can_differ_with_exact_payload(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, _ = verify_vsix_test.VsixTest().package(root)
            downloaded = root / "registry.vsix"
            with zipfile.ZipFile(original) as source, zipfile.ZipFile(downloaded, "w", zipfile.ZIP_DEFLATED) as target:
                for entry in source.infolist():
                    target.writestr(entry.filename, source.read(entry))
                target.writestr("package/signature.p7s", b"outer signature test bytes")
            self.assertNotEqual(original.read_bytes(), downloaded.read_bytes())
            actual = registry.compare_payloads(original, downloaded)
            self.assertIn("extension/out/extension.js", actual)
            self.assertNotIn("package/signature.p7s", actual)

    def test_changed_extra_missing_and_aliased_payload_files_are_refused(self):
        for change in ["changed", "extra", "missing", "alias"]:
            with tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                original, _ = verify_vsix_test.VsixTest().package(root)
                downloaded = root / "registry.vsix"
                with zipfile.ZipFile(original) as source, zipfile.ZipFile(downloaded, "w") as target:
                    for entry in source.infolist():
                        if change == "missing" and entry.filename == "extension/readme.md":
                            continue
                        data = source.read(entry)
                        if change == "changed" and entry.filename == "extension/out/extension.js":
                            data += b"modified"
                        target.writestr(entry.filename, data)
                    if change == "extra":
                        target.writestr("extension/unqualified.js", b"extra")
                    if change == "alias":
                        target.writestr("extension/OUT/extension.js", b"alias")
                with self.assertRaises(ValueError):
                    registry.compare_payloads(original, downloaded)

    def test_exact_identity_endpoints_never_request_latest(self):
        manifest = verify_vsix_test.VsixTest.manifest
        self.assertEqual(registry.registry_url("marketplace", manifest),
                         "https://marketplace.visualstudio.com/_apis/public/gallery/publisher/applied-knowledge-systems/extension/terraphim-it/0.1.0/assetbyname/Microsoft.VisualStudio.Services.VSIXPackage")
        self.assertEqual(registry.registry_url("openvsx", manifest),
                         "https://open-vsx.org/api/applied-knowledge-systems/terraphim-it/universal/0.1.0")
        for field, value in [("version", "latest"), ("name", "../other"), ("publisher", "x?token=secret")]:
            with self.assertRaises(ValueError):
                registry.registry_url("marketplace", {**manifest, field: value})

    def test_openvsx_metadata_is_bound_to_exact_identity_and_trusted_origin(self):
        manifest = verify_vsix_test.VsixTest.manifest
        url = "https://open-vsx.org/api/applied-knowledge-systems/terraphim-it/0.1.0/file/extension.vsix"
        metadata = {"namespace": manifest["publisher"], "name": manifest["name"],
                    "version": manifest["version"], "targetPlatform": "universal", "files": {"download": url}}
        self.assertEqual(registry.openvsx_download(metadata, manifest), url)
        for change in [{"version": "0.2.0"}, {"targetPlatform": "win32-x64"}, {"namespace": "other"},
                       {"files": {"download": "https://other.invalid/extension.vsix"}},
                       {"files": {"download": url + "?token=secret"}}]:
            with self.assertRaises(ValueError):
                registry.openvsx_download({**metadata, **change}, manifest)

    def test_qualification_compares_actual_package_bytes_and_clean_source(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, _ = verify_vsix_test.VsixTest().package(root)
            _, actual, _ = registry.payload(original)
            report = {**actual, "sourceCommit": "a" * 40, "sourceTreeDirty": False}
            manifest = root / "release-manifest.json"
            manifest.write_text(json.dumps(report))
            self.assertEqual(registry.qualify(original, manifest)[1], report)
            for change in [{"sha256": "0" * 64}, {"sourceTreeDirty": True}, {"sourceCommit": "missing"}, {"extension": "other.name"}]:
                manifest.write_text(json.dumps({**report, **change}))
                with self.assertRaises(ValueError):
                    registry.qualify(original, manifest)

    def test_cli_dirty_qualification_fails_with_receipt_without_network(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            original, _ = verify_vsix_test.VsixTest().package(root)
            _, actual, _ = registry.payload(original)
            manifest = root / "release-manifest.json"
            manifest.write_text(json.dumps({**actual, "sourceCommit": "a" * 40, "sourceTreeDirty": True}))
            receipt = root / "registry-failure.json"
            result = subprocess.run([sys.executable, str(Path(registry.__file__)),
                                     "--registry", "marketplace", "--vsix", str(original),
                                     "--manifest", str(manifest), "--receipt", str(receipt)],
                                    text=True, capture_output=True)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(json.loads(receipt.read_text())["status"], "FAILED")
            self.assertIn("clean source", result.stderr)


if __name__ == "__main__":
    unittest.main()
