#!/usr/bin/env python3
"""Validate Terraphim's canonical release-manifest contract."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any

try:
    import jsonschema
except ImportError as exc:  # pragma: no cover - exercised only on missing dependency.
    raise SystemExit(
        "jsonschema is required; install scripts/requirements-release-manifest.txt"
    ) from exc


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / ".release/release-manifest.schema.json"

COMPONENT_SOURCE = {
    "terraphim-server": "terraphim-ai",
    "terraphim-agent": "terraphim-clients",
    "terraphim-grep": "terraphim-clients",
}
RFC3339_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}"
    r"(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$"
)


class ValidationError(ValueError):
    """Manifest validation failed."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValidationError(f"duplicate JSON object key {key!r}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        return json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
    except json.JSONDecodeError as exc:
        raise ValidationError(f"{path}: invalid JSON: {exc}") from exc
    except OSError as exc:
        raise ValidationError(f"{path}: unable to read: {exc}") from exc


def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValidationError(f"{path}: expected object")
    return value


def json_path(error: jsonschema.ValidationError) -> str:
    parts: list[str] = ["manifest"]
    for part in error.absolute_path:
        if isinstance(part, int):
            parts[-1] = f"{parts[-1]}[{part}]"
        else:
            parts.append(str(part))
    return ".".join(parts)


def is_rfc3339_date_time(value: object) -> bool:
    if not isinstance(value, str) or not RFC3339_RE.fullmatch(value):
        return False
    normalized = value[:-1] + "+00:00" if value.endswith("Z") else value
    try:
        dt.datetime.fromisoformat(normalized)
    except ValueError:
        return False
    return True


def format_schema_error(error: jsonschema.ValidationError) -> str:
    message = error.message
    if error.validator == "contains":
        expected_component = (
            error.schema.get("properties", {}).get("component", {}).get("const")
            or error.schema.get("contains", {})
            .get("properties", {})
            .get("component", {})
            .get("const")
        )
        if expected_component:
            message = f"missing required asset for {expected_component}"
    return f"{json_path(error)}: {message}"


def format_checker() -> jsonschema.FormatChecker:
    checker = jsonschema.FormatChecker()
    checker.checks("date-time")(is_rfc3339_date_time)
    return checker


def validate_schema_file(schema_path: Path = SCHEMA) -> dict[str, Any]:
    schema = require_object(load_json(schema_path), "schema")
    try:
        jsonschema.Draft202012Validator.check_schema(schema)
    except jsonschema.SchemaError as exc:
        raise ValidationError(f"schema: {exc.message}") from exc
    return schema


def validate_against_schema(manifest: Any, schema: dict[str, Any]) -> None:
    validator = jsonschema.Draft202012Validator(
        schema,
        format_checker=format_checker(),
    )
    errors = sorted(validator.iter_errors(manifest), key=lambda error: list(error.absolute_path))
    if errors:
        raise ValidationError(format_schema_error(errors[0]))


def validate_business_invariants(manifest: Any) -> None:
    data = require_object(manifest, "manifest")

    release_version = data["release_version"]
    if data["release_tag"] != f"v{release_version}":
        raise ValidationError("release_tag: must be exact v${release_version}")

    sources = data["sources"]
    for repo in sorted(sources):
        source = sources[repo]
        if source["gitea_sha"] != source["github_sha"]:
            raise ValidationError(f"sources.{repo}: Gitea SHA must match GitHub SHA")

    seen_names: set[str] = set()
    seen_hashes: set[str] = set()
    for index, asset in enumerate(data["assets"]):
        path = f"assets[{index}]"
        name = asset["name"]
        if name in seen_names:
            raise ValidationError(f"{path}.name: duplicate asset name {name!r}")
        seen_names.add(name)

        sha256 = asset["sha256"]
        if sha256 in seen_hashes:
            raise ValidationError(f"{path}.sha256: duplicate hash {sha256}")
        seen_hashes.add(sha256)

        component = asset["component"]
        source_repo = asset["source_repo"]
        expected_repo = COMPONENT_SOURCE[component]
        if source_repo != expected_repo:
            raise ValidationError(f"{path}.source_repo: does not own component {component}")

        if asset["source_sha"] != sources[source_repo]["gitea_sha"]:
            raise ValidationError(f"{path}.source_sha: source SHA mismatch")

        if asset["format"] != "rb" and len(asset["signature"]) == 0:
            raise ValidationError(f"{path}.signature: non-rb assets require a signature")


def validate_signature_invariants(manifest: Any) -> None:
    if not isinstance(manifest, dict) or not isinstance(manifest.get("assets"), list):
        return

    for index, asset in enumerate(manifest["assets"]):
        if not isinstance(asset, dict):
            continue
        signature = asset.get("signature")
        if asset.get("format") != "rb" and isinstance(signature, str) and len(signature) == 0:
            raise ValidationError(f"assets[{index}].signature: non-rb assets require a signature")


def validate_release_manifest(manifest: Any, schema: dict[str, Any] | None = None) -> None:
    checked_schema = schema if schema is not None else validate_schema_file()
    validate_signature_invariants(manifest)
    validate_against_schema(manifest, checked_schema)
    validate_business_invariants(manifest)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", nargs="?", help="release-manifest.json to validate")
    parser.add_argument(
        "--schema",
        default=str(SCHEMA),
        help="Draft 2020-12 JSON Schema to validate against",
    )
    parser.add_argument(
        "--self-check",
        action="store_true",
        help="parse and validate the selected Draft 2020-12 JSON Schema",
    )
    parser.add_argument(
        "--self-check-schema",
        action="store_true",
        help="alias for --self-check",
    )
    args = parser.parse_args(argv)

    try:
        schema = validate_schema_file(Path(args.schema))
        if (args.self_check or args.self_check_schema) and args.manifest is None:
            print("release manifest schema self-check passed")
            return 0
        if args.manifest is None:
            raise ValidationError("manifest path is required")
        validate_release_manifest(load_json(Path(args.manifest)), schema)
    except ValidationError as exc:
        print(f"release manifest validation failed: {exc}", file=sys.stderr)
        return 1

    print("release manifest validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
