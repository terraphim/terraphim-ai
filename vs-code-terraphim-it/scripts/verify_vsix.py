#!/usr/bin/env python3
"""Inspect the actual VSIX and bind its bytes to the release source and assets."""
import argparse
import hashlib
import hmac
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import stat
import unicodedata
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def verify(package, expected, asset_lock):
    with zipfile.ZipFile(package) as archive:
        names = archive.namelist()
        if len(set(names)) != len(names):
            raise ValueError("Duplicate VSIX entries")
        aliases = set()
        if sum(entry.file_size for entry in archive.infolist()) > 128 * 1024 * 1024:
            raise ValueError("VSIX exceeds unpacked size limit")
        for entry in archive.infolist():
            name = entry.filename
            path = PurePosixPath(name)
            if (not name or path.is_absolute() or ".." in path.parts or "\\" in name
                    or path.as_posix() != name or any(part.endswith((" ", ".")) or ":" in part for part in path.parts)
                    or any(ord(char) < 32 for char in name)
                    or stat.S_ISLNK(entry.external_attr >> 16)):
                raise ValueError("Unsafe or noncanonical VSIX path")
            alias = unicodedata.normalize("NFC", name).casefold()
            if alias in aliases:
                raise ValueError("Ambiguous VSIX extraction path")
            aliases.add(alias)
        xml = ET.fromstring(archive.read("extension.vsixmanifest"))
        ns = {"v": "http://schemas.microsoft.com/developer/vsx-schema/2011"}
        identities = xml.findall("v:Metadata/v:Identity", ns)
        if len(identities) != 1:
            raise ValueError("Missing or ambiguous VSIX XML identity")
        identity = identities[0].attrib
        if any(identity.get(key) != expected[value] for key, value in
               (("Id", "name"), ("Publisher", "publisher"), ("Version", "version"))):
            raise ValueError("VSIX XML identity mismatch")
        if identity.get("TargetPlatform", "universal") != "universal":
            raise ValueError("VSIX must be platform independent")
        targets = xml.findall("v:Installation/v:InstallationTarget", ns)
        if len(targets) != 1 or targets[0].get("Id") != "Microsoft.VisualStudio.Code":
            raise ValueError("VSIX installation target mismatch")
        properties = xml.findall("v:Metadata/v:Properties/v:Property", ns)
        property_map = {entry.get("Id"): entry.get("Value") for entry in properties}
        if len(property_map) != len(properties):
            raise ValueError("Ambiguous VSIX properties")
        if (property_map.get("Microsoft.VisualStudio.Code.Engine") != expected["engines"]["vscode"]
                or property_map.get("Microsoft.VisualStudio.Code.PreRelease") != "true"):
            raise ValueError("VSIX engine or preview channel mismatch")
        manifest_assets = [entry for entry in xml.findall("v:Assets/v:Asset", ns)
                           if entry.get("Type") == "Microsoft.VisualStudio.Code.Manifest"]
        if len(manifest_assets) != 1 or manifest_assets[0].get("Path") != "extension/package.json":
            raise ValueError("VSIX JSON manifest path mismatch")
        actual = json.loads(archive.read("extension/package.json"))
        for key in ("name", "publisher", "version", "license", "engines"):
            if actual[key] != expected[key]:
                raise ValueError("VSIX manifest mismatch: " + key)
        if archive.read("extension/media/editor-build.json") != asset_lock:
            raise ValueError("VSIX asset lock mismatch")
        if archive.read("extension/media/editor/host-assets.json") != asset_lock:
            raise ValueError("VSIX embedded asset manifest mismatch")
        assets = json.loads(asset_lock)["assets"]
        for name, digest in assets.items():
            if not hmac.compare_digest(hashlib.sha256(archive.read("extension/media/editor/" + name)).hexdigest(), digest):
                raise ValueError("VSIX asset bytes changed: " + name)
        for name in ("out/extension.js", "out/alternativeControl.js", "media/knowledge-graph.cjs",
                     "media/export-intent.js", "media/export-target.cjs", "media/alternative-control.js", "LICENSE.txt", "readme.md",
                     "node_modules/@tomic/lib/package.json"):
            if "extension/" + name not in names:
                raise ValueError("Missing runtime file: " + name)
        forbidden = ("extension/tests/", "extension/scripts/", "extension/src/", "extension/rust-lib/target/")
        if any(name.startswith(forbidden) or name.endswith(".vsix") for name in names):
            raise ValueError("Development artifacts in VSIX")
    return {"schema": 1, "extension": actual["publisher"] + "." + actual["name"],
            "version": actual["version"], "sha256": hashlib.sha256(Path(package).read_bytes()).hexdigest(),
            "bytes": Path(package).stat().st_size,
            "editorSource": json.loads(asset_lock)["sourceHead"], "assets": len(assets)}


def check_tag(tag, version):
    if not re.fullmatch(r"terraphim_vscode-v\d+\.\d+\.\d+", tag) or tag != "terraphim_vscode-v" + version:
        raise ValueError("Tag must match the exact extension version")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    parser.add_argument("--tag")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads((ROOT / "package.json").read_text())
    if args.tag:
        check_tag(args.tag, manifest["version"])
    report = verify(args.package, manifest, (ROOT / "media/editor-build.json").read_bytes())
    report["sourceCommit"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    report["sourceTreeDirty"] = bool(subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=no"], cwd=ROOT, text=True).strip())
    if args.tag and report["sourceTreeDirty"]:
        raise ValueError("Release qualification requires a clean tracked source tree")
    report["qualification"] = "Package integrity; native acceptance is a separate CI gate"
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    args.output.with_name("SHA256SUMS").write_text(f"{report['sha256']}  {args.package.name}\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
