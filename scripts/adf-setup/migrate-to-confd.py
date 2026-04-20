#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "tomli-w>=1.0.0",
# ]
# ///
"""migrate-to-confd.py -- Split monolithic orchestrator TOML files into a
base config + per-project conf.d/ files.

Usage:
    uv run migrate-to-confd.py \\
        --input orchestrator.toml \\
        --input odilo-orchestrator.toml \\
        --output-dir conf.d/ \\
        --base-output orchestrator.toml

The script is idempotent: running it twice produces byte-identical output.

Banned model prefixes (exits non-zero on violation):
  opencode/  github-copilot/  google/  huggingface/

Allowed providers:
  kimi-for-coding/  minimax-coding-plan/  zai-coding-plan/
  opencode-go/  anthropic/  bare sonnet/opus/haiku
  /path/to/claude  /path/to/opencode  (absolute paths)
"""

import argparse
import sys
import re
from pathlib import Path

# Python 3.11+ has tomllib in stdlib
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ImportError:
        print("ERROR: tomllib not available. Use Python 3.11+ or install tomli.", file=sys.stderr)
        sys.exit(1)

import tomli_w

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BANNED_PREFIXES = [
    "opencode/",
    "github-copilot/",
    "google/",
    "huggingface/",
]

# Global keys kept in base orchestrator.toml (not per-project).
BASE_GLOBAL_KEYS = {
    "working_dir",
    "restart_cooldown_secs",
    "max_restart_count",
    "restart_budget_window_secs",
    "disk_usage_threshold",
    "tick_interval_secs",
    "handoff_buffer_ttl_secs",
    "persona_data_dir",
    "skill_data_dir",
    "flow_state_dir",
    "role_config_path",
    "nightwatch",
    "compound_review",
    "routing",
    "webhook",
}

# Per-project keys that belong in [[projects]] entries.
PROJECT_LEVEL_KEYS = {
    "working_dir",
    "gitea",
    "quickwit",
    "workflow",
    "mentions",
}


# ---------------------------------------------------------------------------
# Filename -> project_id mapping
# ---------------------------------------------------------------------------

def project_id_from_path(path: Path) -> str:
    """Derive project_id from the input filename stem.

    Rules:
        orchestrator.toml               -> "terraphim"
        odilo-orchestrator.toml         -> "odilo"
        digital-twins-orchestrator.toml -> "digital-twins"
        <name>-orchestrator.toml        -> "<name>"
        <name>.toml                     -> "<name>"
    """
    stem = path.stem  # e.g. "odilo-orchestrator" or "orchestrator"
    if stem == "orchestrator":
        return "terraphim"
    # Strip "-orchestrator" suffix if present.
    suffix = "-orchestrator"
    if stem.endswith(suffix):
        return stem[: -len(suffix)]
    return stem


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------

def _is_banned(model_value: str) -> bool:
    """Return True if the model value starts with a banned prefix."""
    for prefix in BANNED_PREFIXES:
        if model_value.startswith(prefix):
            return True
    return False


def validate_models(data: dict, source_path: Path) -> None:
    """Validate model/fallback_model fields across all agents.

    Exits non-zero with a clear error message if a banned provider is found.
    """
    agents = data.get("agents", [])
    for agent in agents:
        agent_name = agent.get("name", "<unnamed>")
        for field in ("model", "fallback_model"):
            value = agent.get(field)
            if value and _is_banned(value):
                print(
                    f"ERROR: Agent '{agent_name}' in {source_path} uses banned provider"
                    f" '{value}' (field: {field}).",
                    file=sys.stderr,
                )
                sys.exit(1)

    # Also check compound_review.model and compound_review.fallback_model
    cr = data.get("compound_review", {})
    for field in ("model", "fallback_model"):
        value = cr.get(field) if cr else None
        if value and _is_banned(value):
            print(
                f"ERROR: compound_review in {source_path} uses banned provider"
                f" '{value}' (field: {field}).",
                file=sys.stderr,
            )
            sys.exit(1)


# ---------------------------------------------------------------------------
# Transformation helpers
# ---------------------------------------------------------------------------

def _deep_copy(obj):
    """Return a plain Python deep copy without external deps."""
    import copy
    return copy.deepcopy(obj)


def build_project_entry(data: dict, project_id: str) -> dict:
    """Build a [[projects]] dict from top-level fields of a monolithic config."""
    entry: dict = {"id": project_id}
    # working_dir is required on Project
    if "working_dir" in data:
        entry["working_dir"] = str(data["working_dir"])
    for key in ("gitea", "quickwit", "workflow", "mentions"):
        if key in data:
            entry[key] = _deep_copy(data[key])
    return entry


def build_agent_entries(data: dict, project_id: str) -> list:
    """Return agents list with project field injected."""
    agents = []
    for agent in data.get("agents", []):
        a = _deep_copy(agent)
        # Only set project if not already set
        if "project" not in a:
            a["project"] = project_id
        agents.append(a)
    return agents


def build_flow_entries(data: dict, project_id: str) -> list:
    """Return flows list with project field injected."""
    flows = []
    for flow in data.get("flows", []):
        f = _deep_copy(flow)
        if "project" not in f:
            f["project"] = project_id
        flows.append(f)
    return flows


def build_confd_doc(data: dict, project_id: str) -> dict:
    """Build the per-project conf.d TOML document."""
    doc: dict = {}

    project_entry = build_project_entry(data, project_id)
    doc["projects"] = [project_entry]

    agents = build_agent_entries(data, project_id)
    if agents:
        doc["agents"] = agents

    flows = build_flow_entries(data, project_id)
    if flows:
        doc["flows"] = flows

    return doc


def build_base_doc(inputs: list[tuple[Path, dict]], include_glob: str) -> dict:
    """Build the base orchestrator.toml document.

    Global settings are taken from the first input that defines them.
    include glob is set to the provided pattern.
    Keys are emitted in a deterministic order so that repeated runs produce
    byte-identical output.
    """
    # Collect global settings from inputs (first wins), using sorted key order
    # to ensure deterministic serialisation.
    base: dict = {}

    for _path, data in inputs:
        for key in sorted(BASE_GLOBAL_KEYS):
            if key not in base and key in data:
                base[key] = _deep_copy(data[key])

    # Remove per-project keys that leaked into globals (e.g. working_dir is
    # also a Project field, but OrchestratorConfig.working_dir is the global).
    # We keep working_dir in base because OrchestratorConfig has it.

    # Set include glob.
    base["include"] = [include_glob]

    # Ensure required nightwatch / compound_review placeholders exist with
    # sensible defaults if missing from all inputs.
    if "nightwatch" not in base:
        base["nightwatch"] = {
            "eval_interval_secs": 300,
            "minor_threshold": 0.10,
            "moderate_threshold": 0.20,
            "severe_threshold": 0.40,
            "critical_threshold": 0.70,
        }
    if "compound_review" not in base:
        base["compound_review"] = {
            "schedule": "0 2 * * *",
            "repo_path": ".",
        }

    return base


# ---------------------------------------------------------------------------
# TOML serialisation helper
# ---------------------------------------------------------------------------

def _normalise(obj):
    """Recursively normalise types for tomli_w serialisation.

    tomli_w only accepts: str, int, float, bool, dict, list, datetime, Path
    is not accepted -- convert to str.
    """
    if isinstance(obj, dict):
        return {k: _normalise(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_normalise(v) for v in obj]
    if isinstance(obj, Path):
        return str(obj)
    return obj


def serialise_toml(doc: dict) -> bytes:
    """Serialise a dict to TOML bytes using tomli_w."""
    return tomli_w.dumps(_normalise(doc)).encode("utf-8")


# ---------------------------------------------------------------------------
# Main logic
# ---------------------------------------------------------------------------

def run(
    input_paths: list[Path],
    output_dir: Path,
    base_output: Path,
    dry_run: bool,
) -> None:
    # Parse and validate all inputs first.
    inputs: list[tuple[Path, dict]] = []
    for path in input_paths:
        if not path.exists():
            print(f"ERROR: Input file not found: {path}", file=sys.stderr)
            sys.exit(1)
        with open(path, "rb") as fh:
            data = tomllib.load(fh)
        validate_models(data, path)
        inputs.append((path, data))

    # Build per-project conf.d files.
    confd_files: list[tuple[Path, bytes]] = []
    for path, data in inputs:
        pid = project_id_from_path(path)
        confd_doc = build_confd_doc(data, pid)
        confd_bytes = serialise_toml(confd_doc)
        out_path = output_dir / f"{pid}.toml"
        confd_files.append((out_path, confd_bytes))

    # Determine include glob relative to base_output parent.
    # We emit a simple relative glob "conf.d/*.toml".
    include_glob = "conf.d/*.toml"

    # Build base orchestrator.toml.
    base_doc = build_base_doc(inputs, include_glob)
    base_bytes = serialise_toml(base_doc)

    # Report what would be written.
    if dry_run:
        print(f"[dry-run] Would write base config: {base_output}")
        for out_path, _ in confd_files:
            print(f"[dry-run] Would write conf.d file: {out_path}")
        print("[dry-run] No files written.")
        return

    # Write output.
    output_dir.mkdir(parents=True, exist_ok=True)
    base_output.parent.mkdir(parents=True, exist_ok=True)

    base_output.write_bytes(base_bytes)
    print(f"Written: {base_output}")

    for out_path, content in confd_files:
        out_path.write_bytes(content)
        print(f"Written: {out_path}")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Migrate monolithic orchestrator TOML files to conf.d/ layout.",
    )
    parser.add_argument(
        "--input",
        dest="inputs",
        action="append",
        required=True,
        metavar="PATH",
        help="Input TOML file (repeatable).",
    )
    parser.add_argument(
        "--output-dir",
        default="scripts/adf-setup/conf.d/",
        metavar="DIR",
        help="Output directory for per-project conf.d/ files (default: scripts/adf-setup/conf.d/).",
    )
    parser.add_argument(
        "--base-output",
        default="scripts/adf-setup/orchestrator.toml",
        metavar="PATH",
        help="Output path for the base orchestrator.toml (default: scripts/adf-setup/orchestrator.toml).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would be written without actually writing files.",
    )
    args = parser.parse_args()

    input_paths = [Path(p) for p in args.inputs]
    output_dir = Path(args.output_dir)
    base_output = Path(args.base_output)

    run(input_paths, output_dir, base_output, args.dry_run)


if __name__ == "__main__":
    main()
