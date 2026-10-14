"""Immutable-release admission tests using actual ZIP/files, without mocked services."""
import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import download_registry_release as download
import verify_registry
import verify_vsix_test


class ReleaseDownloadTest(unittest.TestCase):
    tag = "terraphim_vscode-v0.1.0"

    def fixture(self, root):
        package, _ = verify_vsix_test.VsixTest().package(root)
        package.rename(root / "terraphim-it.vsix")
        _, report, _ = verify_registry.payload(root / "terraphim-it.vsix")
        report.update(sourceCommit="a" * 40, sourceTreeDirty=False)
        (root / "release-manifest.json").write_text(json.dumps(report))
        (root / "SHA256SUMS").write_text(download.digest(root / "terraphim-it.vsix") + "  terraphim-it.vsix\n")
        assets = [{"name": name, "state": "uploaded", "size": (root / name).stat().st_size,
                   "digest": "sha256:" + download.digest(root / name),
                   "browser_download_url": f"https://github.com/{download.REPOSITORY}/releases/download/{self.tag}/{name}"}
                  for name in sorted(download.ASSETS)]
        return {"tag_name": self.tag, "draft": False, "prerelease": True, "assets": assets}

    def test_exact_release_bytes_pass_and_any_asset_mutation_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            metadata = self.fixture(root)
            assets = download.release_assets(metadata, self.tag)
            self.assertEqual(download.verify_downloads(root, assets, self.tag)["version"], "0.1.0")
            for name in download.ASSETS:
                path = root / name
                original = path.read_bytes()
                path.write_bytes(original + b"modified")
                with self.assertRaises(ValueError):
                    download.verify_downloads(root, assets, self.tag)
                path.write_bytes(original)

    def test_draft_wrong_tag_channel_url_digest_duplicates_fail_closed(self):
        with tempfile.TemporaryDirectory() as folder:
            original = self.fixture(Path(folder))
            for mutation in ["draft", "tag", "channel", "url", "digest", "duplicate", "size"]:
                value = copy.deepcopy(original)
                if mutation == "draft": value["draft"] = True
                elif mutation == "tag": value["tag_name"] = "other"
                elif mutation == "channel": value["prerelease"] = False
                elif mutation == "url": value["assets"][0]["browser_download_url"] = "https://other.invalid/a"
                elif mutation == "digest": value["assets"][0]["digest"] = None
                elif mutation == "duplicate": value["assets"][0] = value["assets"][1]
                else: value["assets"][0]["size"] = True
                with self.subTest(mutation=mutation), self.assertRaises((ValueError, TypeError)):
                    download.release_assets(value, self.tag)


if __name__ == "__main__":
    unittest.main()
