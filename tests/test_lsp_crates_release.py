"""Release refusals tested with real Git histories, TOML files and tar archives."""
import copy
import importlib.util
import io
import json
import os
import re
import shutil
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("release", ROOT / "scripts/lsp_crates_release.py")
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)


class ReleaseContract(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.plan = r.load_plan(ROOT)
        (self.root / "release").mkdir()
        (self.root / "release/lsp-crates.json").write_text(json.dumps(self.plan))
        (self.root / "Cargo.toml").write_text('[workspace]\nmembers = ["unrelated"]\n[patch.crates-io]\nunrelated = { version = "1", registry = "terraphim" }\n')
        for index, package in enumerate(self.plan["packages"]):
            path = self.root / package["path"]
            path.parent.mkdir(parents=True)
            content = '[package]\nname = "' + package["name"] + '"\nversion = "' + package["version"] + '"\npublish = ["crates-io"]\nrepository = "https://github.com/terraphim/terraphim-ai"\ndescription = "Release fixture"\nlicense = "Apache-2.0"\n'
            if index:
                dependency = self.plan["packages"][index - 1]
                content += '\n[dependencies]\n"' + dependency["name"] + '" = { version = "' + dependency["version"] + '", path = "../' + dependency["name"] + '" }\n'
            if package["name"] == "terraphim_lsp_core":
                content = content.replace('[package]\n', '[package]\nworkspace = "../terraphim_lsp"\n')
            else:
                content += '\n[workspace]\nmembers = [".", "../terraphim_lsp_core"]\n'
            path.write_text(content)
        self.git("init", "-q")
        self.git("config", "user.name", "Release Test")
        self.git("config", "user.email", "release-test@example.invalid")
        self.commit()
        self.head = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", self.head)
        self.item = self.plan["packages"][0]
        self.tag = self.item["name"] + "-v" + self.item["version"]
        self.git("tag", self.tag)
        self.env = {"GITHUB_REPOSITORY": self.plan["repository"], "GITHUB_REF": "refs/tags/" + self.tag,
                    "GITHUB_SHA": self.head, "GITHUB_EVENT_NAME": "push"}

    def git(self, *args):
        return r.git(self.root, *args)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "Fixture")

    def test_reviewed_exact_tag_and_order_pass(self):
        self.assertEqual(r.validate_identity(self.root, self.plan, self.tag, self.env), (self.item, self.head))
        r.validate_manifests(self.root, self.plan)

    def test_wrong_event_repository_ref_sha_or_version_refused(self):
        cases = {"GITHUB_EVENT_NAME": "pull_request_target", "GITHUB_REPOSITORY": "other/core",
                 "GITHUB_REF": "refs/heads/main", "GITHUB_SHA": "0" * 40}
        for key, value in cases.items():
            with self.subTest(key=key), self.assertRaises(ValueError):
                r.validate_identity(self.root, self.plan, self.tag, {**self.env, key: value})
        with self.assertRaises(ValueError):
            r.selected_package(self.plan, self.tag + "-other")

    def test_untracked_source_refused(self):
        (self.root / "unreviewed.rs").write_text("// unreviewed")
        with self.assertRaisesRegex(ValueError, "Dirty"):
            r.validate_identity(self.root, self.plan, self.tag, self.env)

    def test_source_outside_main_refused(self):
        (self.root / "new.txt").write_text("change")
        self.commit()
        new = self.git("rev-parse", "HEAD")
        self.git("tag", "-f", self.tag)
        with self.assertRaises(subprocess.CalledProcessError):
            r.validate_identity(self.root, self.plan, self.tag, {**self.env, "GITHUB_SHA": new})

    def test_moved_tag_refused(self):
        (self.root / "new.txt").write_text("change")
        self.commit()
        new = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", new)
        with self.assertRaisesRegex(ValueError, "Moved"):
            r.validate_identity(self.root, self.plan, self.tag, {**self.env, "GITHUB_SHA": new})

    def test_manifest_version_and_private_publish_refused(self):
        path = self.root / self.item["path"]
        original = path.read_text()
        for before, after in [(self.item["version"], "99.0.0"), ('["crates-io"]', '["terraphim"]'),
                              ('https://github.com/terraphim/terraphim-ai', 'https://git.terraphim.cloud/terraphim/terraphim-ai')]:
            path.write_text(original.replace(before, after))
            with self.subTest(after=after), self.assertRaises(ValueError):
                r.validate_manifests(self.root, self.plan)

    def test_reversed_dependency_order_refused(self):
        bad = copy.deepcopy(self.plan)
        bad["packages"].reverse()
        with self.assertRaisesRegex(ValueError, "order"):
            r.validate_manifests(self.root, bad)

    def test_private_registry_git_and_patch_refused(self):
        path = self.root / self.item["path"]
        original = path.read_text()
        for dep in ['registry = "terraphim", version = "1"', 'git = "https://example.invalid/repo"']:
            path.write_text(original + '\n[dependencies]\nother = {' + dep + '}\n')
            with self.assertRaisesRegex(ValueError, "non-public"):
                r.validate_manifests(self.root, self.plan)
        path.write_text(original)
        with (self.root / self.plan["workspaceManifest"]).open("a") as output:
            output.write('\n[patch.crates-io]\nother = { path = "elsewhere" }\n')
        with self.assertRaisesRegex(ValueError, "overrides"):
            r.validate_manifests(self.root, self.plan)

    def test_source_replacement_refused(self):
        (self.root / ".cargo").mkdir()
        (self.root / ".cargo/config.toml").write_text('[source.crates-io]\nreplace-with = "private"\n')
        with self.assertRaisesRegex(ValueError, "override"):
            r.validate_manifests(self.root, self.plan)

    def test_plan_duplicates_and_escape_refused(self):
        path = self.root / "release/lsp-crates.json"
        path.write_text('{"schema":1,"schema":1}')
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            r.load_plan(self.root)
        bad = copy.deepcopy(self.plan)
        bad["packages"][0]["path"] = "../Cargo.toml"
        path.write_text(json.dumps(bad))
        with self.assertRaisesRegex(ValueError, "manifest path"):
            r.load_plan(self.root)

    def package(self, extra=None, source=None, dependency=""):
        archive_path = self.root / "test.crate"
        prefix = self.item["name"] + "-" + self.item["version"]
        content = {'Cargo.toml': '[package]\nname="' + self.item["name"] + '"\nversion="' + self.item["version"] + '"\n' + dependency,
                   '.cargo_vcs_info.json': json.dumps({'git': {'sha1': source or self.head}, 'path_in_vcs': str(Path(self.item['path']).parent)}),
                   'src/lib.rs': 'pub fn example() {}'}
        with tarfile.open(archive_path, "w:gz") as archive:
            for name, text in content.items():
                info = tarfile.TarInfo(prefix + "/" + name)
                raw = text.encode(); info.size = len(raw)
                archive.addfile(info, io.BytesIO(raw))
            if extra:
                info = tarfile.TarInfo(extra); info.size = 1
                archive.addfile(info, io.BytesIO(b"x"))
        return archive_path

    def test_real_archive_source_hash_and_list_pass(self):
        path = self.package()
        result = r.inspect_package(path, self.item, self.head)
        self.assertEqual(result["sha256"], r.digest(path))
        self.assertEqual(len(result["files"]), 3)

    def test_real_archive_unsafe_paths_duplicate_secrets_and_source_refused(self):
        prefix = self.item["name"] + "-" + self.item["version"]
        for name in ["../outside", "/absolute", prefix + "/.env", prefix + "/.cargo/config.toml", prefix + "/src/lib.rs"]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                r.inspect_package(self.package(extra=name), self.item, self.head)
        with self.assertRaisesRegex(ValueError, "source commit"):
            r.inspect_package(self.package(source="0" * 40), self.item, self.head)

    def test_packaged_private_dependency_refused(self):
        path = self.package(dependency='\n[dependencies.other]\nversion="1"\nregistry-index="sparse+https://example.invalid/"\n')
        with self.assertRaisesRegex(ValueError, "crates.io-only"):
            r.inspect_package(path, self.item, self.head)

    def test_unpacked_package_is_external_and_exact(self):
        package = self.package()
        with tempfile.TemporaryDirectory() as external:
            destination = Path(external) / "unpacked"
            crate = r.unpack_package(package, self.item, self.head, destination, self.root)
            self.assertEqual((crate / "src/lib.rs").read_text(), "pub fn example() {}")
            self.assertFalse((crate / ".cargo/config.toml").exists())
            with self.assertRaisesRegex(ValueError, "new directory"):
                r.unpack_package(package, self.item, self.head, destination, self.root)
        with self.assertRaisesRegex(ValueError, "outside"):
            r.unpack_package(package, self.item, self.head, self.root / "unpacked", self.root)

    def test_unsafe_archive_is_rejected_before_any_extraction(self):
        package = self.package(extra="../outside")
        with tempfile.TemporaryDirectory() as external:
            destination = Path(external) / "unpacked"
            with self.assertRaises(ValueError):
                r.unpack_package(package, self.item, self.head, destination, self.root)
            self.assertFalse(destination.exists())

    def test_unrelated_root_private_patches_do_not_enter_standalone_workspace(self):
        r.validate_manifests(self.root, self.plan)
        workspace = self.root / self.plan["workspaceManifest"]
        workspace.write_text(workspace.read_text().replace('members = [".", "../terraphim_lsp_core"]', 'members = [".", "../terraphim_lsp_core", "../../unrelated"]'))
        with self.assertRaisesRegex(ValueError, "membership"):
            r.validate_manifests(self.root, self.plan)

    def test_package_cannot_silently_use_outer_workspace(self):
        path = self.root / self.item["path"]
        path.write_text(path.read_text().replace('workspace = "../terraphim_lsp"', 'workspace = "../.."'))
        with self.assertRaisesRegex(ValueError, "different workspace"):
            r.validate_manifests(self.root, self.plan)

    def test_nested_cargo_configuration_override_refused(self):
        path = self.root / "crates/.cargo/config.toml"
        path.parent.mkdir()
        path.write_text('[patch.crates-io]\nother = {path = "elsewhere"}\n')
        with self.assertRaisesRegex(ValueError, "override"):
            r.validate_manifests(self.root, self.plan)

    def test_explicit_target_directory_is_bounded_to_runner_temp(self):
        with tempfile.TemporaryDirectory() as td:
            directory = Path(td) / "target"
            env = {"RUNNER_TEMP": td, "CARGO_TARGET_DIR": str(directory)}
            self.assertEqual(r.package_archive(self.root, self.item, env), directory / "package" / (self.item["name"] + "-" + self.item["version"] + ".crate"))
            for bad in [{}, {**env, "CARGO_TARGET_DIR": "relative"}, {**env, "CARGO_TARGET_DIR": str(self.root / "target")}]:
                with self.assertRaises(ValueError):
                    r.package_archive(self.root, self.item, bad)
            directory.symlink_to(self.root, target_is_directory=True)
            with self.assertRaises(ValueError):
                r.package_archive(self.root, self.item, env)

    def lockfile(self, changes=None):
        packages = [{"name": p["name"], "version": p["version"]} for p in self.plan["packages"]]
        packages += [{"name": name, "version": version, "source": "registry+https://github.com/rust-lang/crates.io-index", "checksum": "a" * 64}
                     for name, version in (("terraphim_types", "1.22.3"), ("terraphim_alternatives", "0.1.1"))]
        if changes:
            changes(packages)
        path = self.root / "crates/terraphim_lsp/Cargo.lock"
        path.write_text("version = 4\n" + "".join("\n[[package]]\n" + "".join(key + " = " + json.dumps(value) + "\n" for key, value in package.items()) for package in packages))
        return path

    def test_actual_cargo_package_nested_workspace_and_external_tests(self):
        for package in self.plan["packages"]:
            source = (self.root / package["path"]).parent / "src"
            source.mkdir()
            (source / "lib.rs").write_text("pub fn answer() -> u32 { 42 }\n#[test] fn answer_is_saved() { assert_eq!(answer(), 42); }\n")
        with tempfile.TemporaryDirectory() as td:
            temp = Path(td)
            env = {key: os.environ[key] for key in ("PATH", "HOME", "RUSTUP_HOME") if key in os.environ}
            env.update(CARGO_HOME=str(temp / "cargo-home"), CARGO_TARGET_DIR=str(temp / "target"), RUNNER_TEMP=td)
            manifest = self.root / self.plan["workspaceManifest"]
            subprocess.run(["cargo", "generate-lockfile", "--offline", "--manifest-path", str(manifest)], env=env, check=True, capture_output=True)
            self.commit()
            head = self.git("rev-parse", "HEAD")
            subprocess.run(["cargo", "package", "--locked", "--offline", "--manifest-path", str(manifest), "-p", self.item["name"]], env=env, check=True, capture_output=True)
            archive = r.package_archive(self.root, self.item, env)
            result = r.inspect_package(archive, self.item, head)
            self.assertIn(self.item["name"] + "-" + self.item["version"] + "/src/lib.rs", result["files"])
            extracted = r.unpack_package(archive, self.item, head, temp / "extracted", self.root)
            output = subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(extracted / "Cargo.toml")], env=env, check=True, capture_output=True, text=True)
            self.assertIn("1 passed; 0 failed", output.stdout)

    def test_registry_lock_accepts_only_exact_workspace_and_shared_public_identity(self):
        self.lockfile()
        r.validate_lock(self.root, self.plan)
        self.lockfile(lambda packages: packages.append({"name": "terraphim_alternatives", "version": "0.1.0", "source": "registry+https://github.com/rust-lang/crates.io-index", "checksum": "b" * 64}))
        with self.assertRaisesRegex(ValueError, "Shared public"):
            r.validate_lock(self.root, self.plan)

    def test_missing_private_git_or_local_lock_sources_refused(self):
        with self.assertRaisesRegex(ValueError, "missing"):
            r.validate_lock(self.root, self.plan)
        for source in ["registry+sparse+https://registry.example.invalid/", "git+https://github.com/example/repo#" + "a" * 40]:
            self.lockfile(lambda packages: packages[-1].update(source=source))
            with self.assertRaisesRegex(ValueError, "not crates.io"):
                r.validate_lock(self.root, self.plan)
        self.lockfile(lambda packages: packages[-1].pop("source"))
        with self.assertRaisesRegex(ValueError, "Unexpected local"):
            r.validate_lock(self.root, self.plan)
        self.lockfile(lambda packages: packages[-1].update(checksum="bad"))
        with self.assertRaisesRegex(ValueError, "checksum"):
            r.validate_lock(self.root, self.plan)

    def test_source_check_does_not_require_or_authorize_a_release_tag(self):
        self.lockfile()
        self.commit()
        result = r.check_source(self.root, self.plan)
        self.assertEqual(result["scope"], "SOURCE_CONTRACT_ONLY")
        self.assertEqual(result["sourceCommit"], self.git("rev-parse", "HEAD"))
        self.assertNotIn("tag", result)
        self.assertNotIn("registryReadback", result)
        (self.root / "unreviewed").write_text("dirty")
        with self.assertRaisesRegex(ValueError, "Dirty"):
            r.check_source(self.root, self.plan)

    def test_cli_source_check_cannot_be_used_for_a_tagless_release(self):
        self.lockfile()
        self.commit()
        with tempfile.TemporaryDirectory() as td:
            receipt = Path(td) / "source.json"
            command = [sys.executable, str(ROOT / "scripts/lsp_crates_release.py"), "check-source", "--root", str(self.root), "--receipt", str(receipt)]
            subprocess.run(command, check=True, capture_output=True)
            self.assertEqual(json.loads(receipt.read_text())["scope"], "SOURCE_CONTRACT_ONLY")
            command[2] = "validate"
            result = subprocess.run(command, check=False, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Release commands require an exact tag", result.stderr)

    def test_pr_ci_runs_both_crates_and_packages_core_without_publication(self):
        workflow = (ROOT / ".github/workflows/lsp-public-ci.yml").read_text()
        self.assertIn("pull_request:", workflow)
        self.assertIn("crates/terraphim_lsp_core/**", workflow)
        self.assertIn("crates/terraphim_lsp/**", workflow)
        self.assertIn("check-source --receipt", workflow)
        self.assertIn('test --locked --manifest-path "$MANIFEST" --workspace --all-targets', workflow)
        self.assertIn('clippy --locked --manifest-path "$MANIFEST" --workspace --all-targets -- -D warnings', workflow)
        self.assertIn('check --locked --manifest-path "$MANIFEST" -p terraphim_lsp_core --target wasm32-unknown-unknown --lib', workflow)
        self.assertIn('package --locked --manifest-path "$MANIFEST" -p terraphim_lsp_core', workflow)
        self.assertIn('package --locked --manifest-path "$MANIFEST" -p terraphim_lsp --list', workflow)
        self.assertIn('cd "$RUNNER_TEMP/unpacked/terraphim_lsp_core-$core_version"', workflow)
        for forbidden in ("contents: write", "id-token: write", "secrets.", "cargo publish", "--allow-dirty", "--no-verify", "pull_request_target"):
            self.assertNotIn(forbidden, workflow)

    def test_legacy_script_refuses_lsp_before_credentials_or_source_mutation(self):
        bash = shutil.which("bash")
        self.assertIsNotNone(bash)
        script = ROOT / "scripts/publish-crates.sh"
        content = script.read_text()
        defaults = content.split("CRATES=(", 1)[1].split(")", 1)[0]
        self.assertNotIn("terraphim_lsp", defaults)
        for name in ("terraphim_lsp", "terraphim_lsp_core"):
            with self.subTest(name=name):
                before = (self.root / "Cargo.toml").read_bytes()
                result = subprocess.run(
                    [bash, str(script), "--crate", name, "--version", "1.23.1", "--dry-run"],
                    cwd=self.root, env={"PATH": ""}, capture_output=True, text=True, timeout=10,
                )
                self.assertEqual(result.returncode, 1)
                self.assertIn("owned by publish-lsp-crates.yml", result.stdout)
                self.assertNotIn("Checking prerequisites", result.stdout)
                self.assertNotIn("not found", result.stderr)
                self.assertEqual((self.root / "Cargo.toml").read_bytes(), before)

    def test_legacy_workflow_refuses_both_input_forms_before_secret_access(self):
        content = (ROOT / ".github/workflows/publish-crates.yml").read_text()
        guard = content.split("      - name: Refuse packages", 1)[1].split("      - name:", 1)[0]
        script = "\n".join(line[10:] for line in guard.split("        run: |\n", 1)[1].splitlines())
        self.assertLess(content.index("      - name: Refuse packages"), content.index("1Password"))
        self.assertNotIn("terraphim_lsp", content.split("selected='terraphim_update", 1)[1].split("'", 1)[0])
        for key in ("REQUESTED_CRATES", "REQUESTED_CRATE"):
            for name in ("terraphim_lsp", "terraphim_lsp_core"):
                with self.subTest(key=key, name=name):
                    env = {"PATH": "", "REQUESTED_CRATES": "", "REQUESTED_CRATE": "", key: name}
                    result = subprocess.run([shutil.which("bash"), "-c", script], env=env, capture_output=True, text=True, timeout=10)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("owned by publish-lsp-crates.yml", result.stdout)

    def test_legacy_workflow_input_is_literal_and_allowlisted(self):
        content = (ROOT / ".github/workflows/publish-crates.yml").read_text()
        guard = content.split("      - name: Refuse packages", 1)[1].split("      - name:", 1)[0]
        script = "\n".join(line[10:] for line in guard.split("        run: |\n", 1)[1].splitlines())
        marker = self.root / "must-not-exist"
        for value in ("unknown", "terraphim_*", "terraphim_update; exit 0", f"$(touch {marker})", '`echo unsafe`'):
            result = subprocess.run([shutil.which("bash"), "-c", script],
                                    env={"PATH": "", "REQUESTED_CRATES": value, "REQUESTED_CRATE": ""},
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 1)
            self.assertIn("Unsupported legacy package", result.stdout)
            self.assertFalse(marker.exists())
        allowed = "terraphim_update terraphim_github_runner terraphim_gitea_runner terraphim_rlm"
        result = subprocess.run([shutil.which("bash"), "-c", script],
                                env={"PATH": "", "REQUESTED_CRATES": allowed, "REQUESTED_CRATE": ""},
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        resolve = content.split("      - name: Resolve crate list", 1)[1].split("      - name:", 1)[0]
        body = resolve.split("        run: |\n", 1)[1]
        self.assertNotIn("${{", body)
        self.assertIn('printf \'list=%s\\n\' "$normalized" >> "$GITHUB_OUTPUT"', body)
        script = "\n".join(line[10:] for line in body.splitlines())
        output = self.root / "github-output"
        result = subprocess.run([shutil.which("bash"), "-c", script],
                                env={"PATH": "", "REQUESTED_CRATES": "terraphim_update\n\tterraphim_rlm", "REQUESTED_CRATE": "", "GITHUB_OUTPUT": str(output)},
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output.read_text(), "list=terraphim_update terraphim_rlm\n")

    def test_comprehensive_keeps_lsp_output_path_and_refuses_lsp_tag_ownership(self):
        content = (ROOT / ".github/workflows/release-comprehensive.yml").read_text()
        builds = re.findall(r"      - name: Build LSP binary[^\n]*\n(.*?)(?=\n      - name:)", content, re.S)
        self.assertEqual(len(builds), 2)
        for block in builds:
            self.assertIn("CARGO_TARGET_DIR: ${{ github.workspace }}/target", block)
            self.assertIn("--manifest-path crates/terraphim_lsp/Cargo.toml", block)
            self.assertIn("-p terraphim_lsp --bin terraphim-lsp", block)
        step = content.split("      - name: Resolve and validate release source", 1)[1].split("      - name:", 1)[0]
        body = step.split("        run: |\n", 1)[1]
        lines = []
        for line in body.splitlines():
            if line and not line.startswith("          "):
                break
            lines.append(line[10:])
        for name in ("terraphim_lsp-v1.22.0", "terraphim_lsp_core-v1.23.1"):
            env = {"PATH": "", "EVENT_NAME": "workflow_dispatch", "MANUAL_RELEASE_TAG": name,
                   "MANUAL_EXPECTED_SOURCE_SHA": "a" * 40}
            result = subprocess.run([shutil.which("bash"), "-c", "\n".join(lines)], env=env, capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 1)
            self.assertIn("LSP tags are owned by publish-lsp-crates.yml", result.stdout)
            self.assertNotIn("command not found", result.stderr)

    def test_private_producer_cannot_trigger_or_write_the_public_release(self):
        producer = (ROOT / ".github/workflows/lsp-producer.yml").read_text()
        self.assertIn("workflow_dispatch:", producer)
        for forbidden in ("push:", "contents: write", "gh release", "lsp_handoff.py publish"):
            self.assertNotIn(forbidden, producer)
        publisher = (ROOT / ".github/workflows/lsp-publish.yml").read_text()
        self.assertIn("workflow_call:", publisher)
        for forbidden in ("workflow_dispatch:", "workflow_run:", "push:", "lsp_handoff.py", "producer_run"):
            self.assertNotIn(forbidden, publisher)

    def test_binary_workflow_requires_successful_crate_readback(self):
        workflow = (ROOT / ".github/workflows/publish-lsp-crates.yml").read_text()
        binaries = workflow.split("  binaries:", 1)[1]
        self.assertIn("needs: [qualify, publish, bootstrap]", binaries)
        self.assertIn("needs.qualify.result == 'success'", binaries)
        self.assertIn("needs.qualify.outputs.crate == 'terraphim_lsp'", binaries)
        self.assertIn("needs.publish.result == 'success' || needs.bootstrap.result == 'success'", binaries)
        self.assertIn("uses: ./.github/workflows/lsp-publish.yml", binaries)
        self.assertIn("source-sha: ${{ github.sha }}", binaries)
        self.assertIn("contents: write", binaries)
        self.assertNotIn("secrets:", binaries)
        for job in (workflow.split("  publish:", 1)[1].split("  bootstrap:", 1)[0], workflow.split("  bootstrap:", 1)[1].split("  binaries:", 1)[0]):
            self.assertIn("scripts/lsp_crates_release.py readback", job)
            self.assertLess(job.index('cargo +"$TOOLCHAIN" publish'), job.index("scripts/lsp_crates_release.py readback"))

    def test_workflow_separates_bootstrap_oidc_and_qualification(self):
        workflow = (ROOT / ".github/workflows/publish-lsp-crates.yml").read_text()
        self.assertNotIn("pull_request:", workflow)
        self.assertIn("startsWith(github.ref, 'refs/tags/')", workflow)
        self.assertIn("environment: crates-io-bootstrap", workflow)
        self.assertIn("github.event_name == 'workflow_dispatch' && inputs.auth_mode == 'bootstrap'", workflow)
        self.assertEqual(workflow.count("id-token: write"), 1)
        self.assertEqual(workflow.count("secrets.CARGO_REGISTRY_TOKEN"), 1)
        self.assertIn("rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18", workflow)
        for forbidden in ["--allow-dirty", "--no-verify", "op read", "crate_list"]:
            self.assertNotIn(forbidden, workflow)
        qualify = workflow.split("  publish:")[0]
        self.assertNotIn("secrets.", qualify)
        self.assertIn("cargo +\"$TOOLCHAIN\" test --locked", qualify)
        self.assertIn("cargo +\"$TOOLCHAIN\" clippy --locked", qualify)
        self.assertIn('rustup target add wasm32-unknown-unknown --toolchain "$TOOLCHAIN"', qualify)
        self.assertIn('if [ "$CRATE" = "terraphim_lsp_core" ]; then', qualify)
        self.assertIn('cargo +"$TOOLCHAIN" check --locked --manifest-path "$MANIFEST" -p "$CRATE" --target wasm32-unknown-unknown --lib', qualify)
        self.assertIn('CARGO_TARGET_DIR:', workflow)
        self.assertIn('MANIFEST: ${{ steps.request.outputs.manifest }}', workflow)
        unpack = qualify.index('scripts/lsp_crates_release.py unpack')
        isolated_cd = qualify.index('cd "$RUNNER_TEMP/unpacked/$CRATE-$VERSION"')
        archive_tests = qualify.index('cargo +"$TOOLCHAIN" test --locked --manifest-path Cargo.toml --all-targets')
        qualified = qualify.index('scripts/lsp_crates_release.py package')
        self.assertLess(unpack, isolated_cd)
        self.assertLess(isolated_cd, archive_tests)
        self.assertLess(archive_tests, qualified)


if __name__ == "__main__":
    unittest.main()
