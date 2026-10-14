"""Publication gates on real ZIP files and a real local Git repository; no remote writes."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import publish_release as publication
import verify_vsix
import verify_vsix_test
import legacy_assets


class PublicationTest(unittest.TestCase):
    tag = "terraphim_vscode-v0.1.0"

    def qualified(self, root):
        fixture = verify_vsix_test.VsixTest()
        vsix, lock = fixture.package(root)
        (root / "media").mkdir()
        (root / "media/editor-build.json").write_bytes(lock)
        (root / "package.json").write_text(json.dumps(fixture.manifest))
        for name in legacy_assets.INPUTS:
            destination = root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((legacy_assets.ROOT / name).read_bytes())
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        subprocess.run(["git", "add", "package.json", "media/editor-build.json", "rust-lib"], cwd=root, check=True)
        subprocess.run(["git", "-c", "commit.gpgsign=false", "-c", "user.name=Publication Test",
                        "-c", "user.email=publication-test@example.invalid", "commit", "-qm", "test fixture"], cwd=root, check=True)
        subprocess.run(["git", "-c", "tag.gpgsign=false", "tag", self.tag], cwd=root, check=True)
        source = publication.git(root, "rev-parse", "HEAD")
        report = verify_vsix.verify(vsix, fixture.manifest, lock)
        report.update(sourceCommit=source, sourceTreeDirty=False)
        manifest = root / "release-manifest.json"
        manifest.write_text(json.dumps(report))
        sums = root / "SHA256SUMS"
        sums.write_text(f"{report['sha256']}  {vsix.name}\n")
        return vsix, manifest, sums, report

    def test_real_package_clean_git_and_exact_tag_are_qualified(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            vsix, manifest, sums, report = self.qualified(root)
            actual, source, files = publication.verify_package(vsix, manifest, sums, self.tag, root)
            self.assertEqual(actual, report)
            self.assertEqual(source, report["sourceCommit"])
            self.assertEqual(files, [vsix, manifest, sums])
            sums.write_text(sums.read_text() + f"{publication.digest(manifest)}  {manifest.name}\n")
            publication.verify_package(vsix, manifest, sums, self.tag, root)

    def test_dirty_source_changed_report_and_wrong_tag_are_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            vsix, manifest, sums, report = self.qualified(root)
            for field, value in [("sourceTreeDirty", True), ("sourceTreeDirty", 0),
                                 ("sourceCommit", "0" * 40), ("sha256", "0" * 64),
                                 ("version", "9.9.9"), ("editorSource", "other")]:
                manifest.write_text(json.dumps({**report, field: value}))
                with self.assertRaises(ValueError):
                    publication.verify_package(vsix, manifest, sums, self.tag, root)
            manifest.write_text(json.dumps(report))
            with self.assertRaises(ValueError):
                publication.verify_package(vsix, manifest, sums, "terraphim_vscode-v0.2.0", root)
            (root / "package.json").write_text((root / "package.json").read_text() + "\n")
            with self.assertRaisesRegex(ValueError, "dirty"):
                publication.verify_package(vsix, manifest, sums, self.tag, root)

    def test_local_tag_must_point_to_exact_qualified_head(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            _, _, _, report = self.qualified(root)
            (root / "new-source.txt").write_text("new revision")
            subprocess.run(["git", "add", "new-source.txt"], cwd=root, check=True)
            subprocess.run(["git", "-c", "commit.gpgsign=false", "-c", "user.name=Publication Test",
                            "-c", "user.email=publication-test@example.invalid", "commit", "-qm", "next"], cwd=root, check=True)
            report["sourceCommit"] = publication.git(root, "rev-parse", "HEAD")
            with self.assertRaisesRegex(ValueError, "tag differs"):
                publication.check_source(root, report, self.tag)

    def test_checksum_parser_and_native_binding_refuse_ambiguity(self):
        line = "a" * 64 + "  extension.vsix\n"
        self.assertEqual(publication.checksums(line), {"extension.vsix": "a" * 64})
        for text in ["", line + line, "a" * 64 + "  ../extension.vsix\n", "invalid", line + "\n"]:
            with self.assertRaises(ValueError):
                publication.checksums(text)
        publication.check_native({"status": "PASS", "packageSha256": "a" * 64}, "a" * 64)
        for receipt in [{}, {"status": "FAIL", "packageSha256": "a" * 64},
                        {"status": "PASS", "packageSha256": "b" * 64}]:
            with self.assertRaises(ValueError):
                publication.check_native(receipt, "a" * 64)

    def test_resume_admits_only_matching_assets_and_never_repairs_public_release(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            vsix, manifest, sums, _ = self.qualified(root)
            files = [vsix, manifest, sums]
            asset = {"name": vsix.name, "state": "uploaded", "size": vsix.stat().st_size,
                     "digest": "sha256:" + publication.digest(vsix)}
            current = {"tag_name": self.tag, "prerelease": True, "draft": True, "assets": [asset]}
            self.assertEqual(publication.resume_plan(current, self.tag, files), [manifest, sums])
            for change in [{"draft": False}, {"prerelease": False}, {"assets": [asset, asset]},
                           {"tag_name": "other"}, {"assets": [{**asset, "name": "extra.zip"}]},
                           {"assets": [{**asset, "size": 0}]}, {"assets": [{**asset, "state": "starter"}]},
                           {"assets": [{**asset, "digest": "sha256:" + "0" * 64}]}]:
                with self.assertRaises(ValueError):
                    publication.resume_plan({**current, **change}, self.tag, files)
            current.update(draft=False, assets=[{"name": p.name, "state": "uploaded", "size": p.stat().st_size} for p in files])
            self.assertEqual(publication.resume_plan(current, self.tag, files), [])

    def test_draft_lookup_is_exact_and_rejects_duplicate_releases(self):
        draft = {"tag_name": self.tag, "draft": True}
        self.assertEqual(publication.select_release([[{"tag_name": "other"}], [draft]], self.tag), draft)
        self.assertIsNone(publication.select_release([[]], self.tag))
        with self.assertRaises(ValueError):
            publication.select_release([[draft], [draft]], self.tag)

    def test_created_draft_readback_is_bounded_and_stops_at_first_visible_release(self):
        draft = {"tag_name": self.tag, "draft": True, "prerelease": True, "assets": []}
        observed = []

        def snapshots(values):
            for value in values:
                observed.append(value)
                yield value

        self.assertEqual(publication.recover_visible_release(snapshots([None, None, draft, None])), draft)
        self.assertEqual(observed, [None, None, draft])
        observed.clear()
        self.assertEqual(publication.recover_visible_release(snapshots([None] * 4 + [draft])), draft)
        self.assertEqual(len(observed), 5)
        observed.clear()
        with self.assertRaisesRegex(ValueError, "same qualified artifact"):
            publication.recover_visible_release(snapshots([None] * 5 + [draft]))
        self.assertEqual(observed, [None] * 5)
        self.assertEqual(len(publication.RELEASE_LOOKUP_DELAYS), 5)
        self.assertEqual(sum(publication.RELEASE_LOOKUP_DELAYS), 8)

    def test_visible_mismatch_or_ambiguous_readback_is_never_retried(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            vsix, manifest, sums, _ = self.qualified(root)
            original = {p.name: p.read_bytes() for p in (vsix, manifest, sums)}
            wrong = {"tag_name": "other", "draft": True, "prerelease": True, "assets": []}
            values = iter([wrong, None])
            current = publication.recover_visible_release(values)
            with self.assertRaisesRegex(ValueError, "tag or preview"):
                publication.resume_plan(current, self.tag, [vsix, manifest, sums])
            self.assertIsNone(next(values))
            self.assertEqual(original, {p.name: p.read_bytes() for p in (vsix, manifest, sums)})

        def ambiguous_snapshots():
            yield None
            draft = {"tag_name": self.tag}
            yield publication.select_release([[draft, draft]], self.tag)

        with self.assertRaisesRegex(ValueError, "Ambiguous"):
            publication.recover_visible_release(ambiguous_snapshots())

    def test_readback_policy_executes_in_real_subprocess_without_cloud_calls(self):
        with tempfile.TemporaryDirectory() as folder:
            fixture = Path(folder) / "readbacks.json"
            draft = {"tag_name": self.tag, "draft": True, "prerelease": True, "assets": []}
            fixture.write_text(json.dumps([None, draft]))
            code = ("import json,sys; sys.path.insert(0,sys.argv[1]); import publish_release; "
                    "print(json.dumps(publish_release.recover_visible_release("
                    "iter(json.load(open(sys.argv[2]))))))")
            result = subprocess.run([sys.executable, "-c", code,
                                     str(Path(publication.__file__).parent), str(fixture)],
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), draft)

    def test_anonymous_download_rejects_credentials_and_overwrite_before_network(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "download"
            for url in ["http://example.invalid/file", "https://user:secret@example.invalid/file",
                        "https://example.invalid/file?token=secret", "https://example.invalid/file#fragment"]:
                with self.assertRaises(ValueError):
                    publication.anonymous_download(url, path)
            path.write_bytes(b"retained evidence")
            with self.assertRaises(ValueError):
                publication.anonymous_download("https://example.invalid/file", path)
            self.assertEqual(path.read_bytes(), b"retained evidence")

    def test_cli_records_failure_before_any_remote_operation(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            vsix, manifest, sums, _ = self.qualified(root)
            receipt = root / "publication-failure.json"
            result = subprocess.run([sys.executable, str(Path(publication.__file__)),
                                     "--repo", "terraphim/terraphim-ai", "--tag", "wrong-tag",
                                     "--vsix", str(vsix), "--manifest", str(manifest),
                                     "--sums", str(sums), "--receipt", str(receipt)],
                                    text=True, capture_output=True)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(json.loads(receipt.read_text())["status"], "FAILED")
            self.assertIn("Tag must match", result.stderr)


if __name__ == "__main__":
    unittest.main()
