#!/usr/bin/env python3
"""Build/check the existing link-replacement wrapper using public pinned crates."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
INPUTS = ["rust-lib/Cargo.toml", "rust-lib/Cargo.lock", "rust-lib/rust-toolchain.toml", "rust-lib/src/lib.rs"]
ASSETS = ["rust-lib/pkg/terraphim_it.js", "rust-lib/pkg/terraphim_it_bg.wasm"]
LOCK = "media/legacy-build.json"
RUST = "1.96.1"
WASM_PACK = "0.13.1"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inputs(root):
    dependencies = tomllib.loads((root / "rust-lib/Cargo.lock").read_text())["package"]
    for dependency in dependencies:
        if "source" in dependency and dependency["source"] != "registry+https://github.com/rust-lang/crates.io-index":
            raise ValueError("Legacy WASM must build from public locked crates.io dependencies")
    automata = [entry for entry in dependencies if entry["name"] == "terraphim_automata"]
    if len(automata) != 1 or automata[0]["version"] != "1.20.4":
        raise ValueError("Expected pinned public Terraphim automata 1.20.4")
    return {name: digest((root / name).read_bytes()) for name in INPUTS}


def verify(root):
    report = json.loads((root / LOCK).read_bytes())
    if report.get("schema") != 1 or report.get("inputs") != inputs(root):
        raise ValueError("Link replacement source changed; run npm run build-rust")
    if report.get("target") != "wasm32-unknown-unknown" or report.get("wasmPack") != WASM_PACK or report.get("rust") != RUST:
        raise ValueError("Unexpected link replacement build toolchain")
    if set(report.get("assets", {})) != set(ASSETS):
        raise ValueError("Missing link replacement assets")
    for name, expected in report["assets"].items():
        if digest((root / name).read_bytes()) != expected:
            raise ValueError("Link replacement asset changed: " + name)
    if not (root / ASSETS[1]).read_bytes().startswith(b"\x00asm"):
        raise ValueError("Link replacement WASM is invalid")
    return report


def build(root):
    source_hashes = inputs(root)
    if subprocess.check_output(["wasm-pack", "--version"], text=True).strip() != "wasm-pack " + WASM_PACK:
        raise ValueError("Build requires wasm-pack " + WASM_PACK)
    environment = dict(os.environ)
    environment.update({"RUSTUP_TOOLCHAIN": RUST, "RUSTC_WRAPPER": "", "RUSTC_WORKSPACE_WRAPPER": "",
                        "CARGO_TARGET_DIR": str(root / "rust-lib/target"),
                        "CARGO_BUILD_BUILD_DIR": str(root / "rust-lib/target/build")})
    subprocess.run(["wasm-pack", "build", "rust-lib", "--target", "nodejs", "--release", "--", "--locked"],
                   cwd=root, env=environment, check=True)
    if source_hashes != inputs(root):
        raise ValueError("Link replacement source changed during build")
    report = {"schema": 1, "rust": RUST, "wasmPack": WASM_PACK,
              "target": "wasm32-unknown-unknown", "inputs": source_hashes,
              "assets": {name: digest((root / name).read_bytes()) for name in ASSETS}}
    (root / LOCK).write_text(json.dumps(report, indent=2) + "\n")
    return verify(root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", action="store_true")
    args = parser.parse_args()
    try:
        report = build(ROOT) if args.build else verify(ROOT)
        print(json.dumps(report))
    except (OSError, KeyError, ValueError, subprocess.CalledProcessError) as error:
        print(f"legacy_assets: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
