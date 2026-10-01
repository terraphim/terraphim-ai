#!/usr/bin/env python3
"""Generate Homebrew formulas from the canonical release manifest (BOM).

Gitea terraphim-ai#3381 ([MP5a]). This module is a pure, network-free
function from a validated release manifest
(`.release/release-manifest.schema.json`) to deterministic formula bytes.
PR dispatch lives in `scripts/homebrew-outbox.sh`, which never mutates the
central publication state (the GitHub release record or the R2 stable
manifest).

Contract:
  * formula specs live in `config/homebrew/formulas.json`;
  * formula bodies are templates in `config/homebrew/templates/`;
  * every asset is resolved by exact name and must exist exactly once, be
    non-empty, carry a lowercase 64-hex SHA-256, and match the expected
    target/os/arch for its slot;
  * the same manifest always renders the same bytes.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "config/homebrew/formulas.json"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PLACEHOLDERS = (
    "@@VERSION@@",
    "@@TAG@@",
    "@@SHA_MACOS@@",
    "@@SHA_LINUX_ARM@@",
    "@@SHA_LINUX_INTEL@@",
)
SLOT_PLACEHOLDERS = (
    ("macos", "@@SHA_MACOS@@"),
    ("linux_arm", "@@SHA_LINUX_ARM@@"),
    ("linux_intel", "@@SHA_LINUX_INTEL@@"),
)


class FormulaError(ValueError):
    """A manifest or spec cannot produce a complete, verified formula."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise FormulaError(f"duplicate JSON object key {key!r}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        text = path.read_text()
    except OSError as exc:
        raise FormulaError(f"cannot read {path}: {exc}") from exc
    try:
        return json.loads(text, object_pairs_hook=reject_duplicate_keys)
    except FormulaError:
        raise
    except json.JSONDecodeError as exc:
        raise FormulaError(f"cannot parse {path}: {exc}") from exc


def load_spec(path: Path = SPEC) -> dict[str, Any]:
    spec = load_json(path)
    formulas = spec.get("formulas") if isinstance(spec, dict) else None
    if not isinstance(formulas, list) or not formulas:
        raise FormulaError("formulas spec must contain a non-empty 'formulas' list")
    for formula in formulas:
        for key in ("component", "file", "template", "slots"):
            if key not in formula:
                raise FormulaError(f"formula spec is missing {key!r}: {formula!r}")
        missing = [slot for slot, _ in SLOT_PLACEHOLDERS if slot not in formula["slots"]]
        if missing:
            raise FormulaError(
                f"{formula['component']}: spec is missing slots {missing}"
            )
    return spec


def validate_manifest(manifest: Any) -> tuple[str, str, dict[str, dict[str, Any]]]:
    if not isinstance(manifest, dict):
        raise FormulaError("manifest must be a JSON object")
    version = manifest.get("release_version")
    tag = manifest.get("release_tag")
    if not isinstance(version, str) or not version:
        raise FormulaError("manifest.release_version must be a non-empty string")
    if tag != f"v{version}":
        raise FormulaError(f"manifest.release_tag {tag!r} must equal {('v' + version)!r}")
    assets = manifest.get("assets")
    if not isinstance(assets, list) or not assets:
        raise FormulaError("manifest.assets must be a non-empty list")
    by_name: dict[str, dict[str, Any]] = {}
    for asset in assets:
        if not isinstance(asset, dict):
            raise FormulaError("every manifest asset must be a JSON object")
        name = asset.get("name")
        if not isinstance(name, str) or not name:
            raise FormulaError("every manifest asset must have a non-empty name")
        if name in by_name:
            raise FormulaError(f"duplicate manifest asset name {name!r}")
        by_name[name] = asset
    return version, tag, by_name


def resolve_asset(
    by_name: dict[str, dict[str, Any]],
    component: str,
    version: str,
    slot_name: str,
    slot: dict[str, Any],
    fmt: str,
) -> dict[str, Any]:
    target = slot["target"]
    name = f"{component}-{version}-{target}.{fmt}"
    asset = by_name.get(name)
    if asset is None:
        raise FormulaError(f"{component}/{slot_name}: missing manifest asset {name!r}")
    checks = (
        ("component", component),
        ("target", target),
        ("format", fmt),
        ("os", slot["os"]),
        ("arch", slot["arch"]),
    )
    for field, expected in checks:
        if asset.get(field) != expected:
            raise FormulaError(
                f"{name}: {field} {asset.get(field)!r} != expected {expected!r}"
            )
    size = asset.get("size_bytes")
    if isinstance(size, bool) or not isinstance(size, int) or size < 1:
        raise FormulaError(f"{name}: size_bytes must be a positive integer, got {size!r}")
    sha = asset.get("sha256")
    if not isinstance(sha, str) or not SHA256_RE.match(sha):
        raise FormulaError(f"{name}: sha256 must be 64 lowercase hex characters")
    return asset


def render_formula(
    formula: dict[str, Any],
    version: str,
    tag: str,
    by_name: dict[str, dict[str, Any]],
) -> str:
    template_path = ROOT / formula["template"]
    try:
        rendered = template_path.read_text()
    except OSError as exc:
        raise FormulaError(f"cannot read template {template_path}: {exc}") from exc
    fmt = formula.get("format", "tar.gz")
    rendered = rendered.replace("@@VERSION@@", version).replace("@@TAG@@", tag)
    for slot_name, placeholder in SLOT_PLACEHOLDERS:
        asset = resolve_asset(
            by_name, formula["component"], version, slot_name, formula["slots"][slot_name], fmt
        )
        rendered = rendered.replace(placeholder, asset["sha256"])
    unresolved = [placeholder for placeholder in PLACEHOLDERS if placeholder in rendered]
    if unresolved:
        raise FormulaError(f"{formula['file']}: unresolved placeholders {unresolved}")
    return rendered


def generate(manifest: Any, output_dir: Path, spec: dict[str, Any] | None = None) -> list[Path]:
    spec = spec or load_spec()
    version, tag, by_name = validate_manifest(manifest)
    output_dir = Path(output_dir)
    written: list[Path] = []
    for formula in spec["formulas"]:
        rendered = render_formula(formula, version, tag, by_name)
        path = output_dir / formula["file"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(rendered)
        written.append(path)
    return written


def stale_formulas(manifest: Any, output_dir: Path, spec: dict[str, Any] | None = None) -> list[Path]:
    spec = spec or load_spec()
    version, tag, by_name = validate_manifest(manifest)
    stale: list[Path] = []
    for formula in spec["formulas"]:
        rendered = render_formula(formula, version, tag, by_name)
        path = Path(output_dir) / formula["file"]
        current = path.read_text() if path.is_file() else None
        if current != rendered:
            stale.append(path)
    return stale


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--check",
        action="store_true",
        help="do not write; fail if the output directory is not exactly current",
    )
    args = parser.parse_args(argv)
    try:
        spec = load_spec()
        manifest = load_json(args.manifest)
        if args.check:
            stale = stale_formulas(manifest, args.output_dir, spec)
            if stale:
                print("error: generated Homebrew formulas are stale:", file=sys.stderr)
                for path in stale:
                    print(f"  {path}", file=sys.stderr)
                return 1
            return 0
        for path in generate(manifest, args.output_dir, spec):
            print(path)
    except FormulaError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
