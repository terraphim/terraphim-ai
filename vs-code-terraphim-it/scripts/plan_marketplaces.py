#!/usr/bin/env python3
"""Plan explicit marketplace attempts after the independent GitHub release."""
import argparse
import json
import os
from pathlib import Path
import re

from verify_vsix import check_tag

ROOT = Path(__file__).resolve().parents[1]
TARGETS = (
    ("visual-studio-marketplace", "VSCODE_PUBLISH_VISUAL_STUDIO_MARKETPLACE"),
    ("open-vsx", "VSCODE_PUBLISH_OPEN_VSX"),
)


def plan(environment):
    """Unset gates skip publication; malformed gates refuse every target."""
    report = {"schema": 1, "scope": "Publication routing only; live registry readback is separate",
              "registries": [], "targets": [], "configurationValid": True}
    for registry, flag in TARGETS:
        value = environment.get(flag, "")
        valid = value in ("", "false", "true")
        enabled = value == "true"
        report["configurationValid"] &= valid
        report["targets"].append({
            "registry": registry, "readinessVariable": flag,
            "status": "PENDING_PUBLICATION" if enabled else "NOT_PUBLISHED",
            "reason": "Explicit readiness gate enabled; publication/readback still required"
                      if enabled else "Readiness gate is unset or disabled",
        })
        if enabled:
            report["registries"].append(registry)
    if not report["configurationValid"]:
        report["registries"] = []
        for target in report["targets"]:
            target.update(status="NOT_PUBLISHED", reason="Invalid readiness configuration; all attempts refused")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    report = plan(os.environ)
    source = os.environ.get("GITHUB_SHA", "")
    tag = os.environ.get("GITHUB_REF_NAME", "")
    try:
        check_tag(tag, json.loads((ROOT / "package.json").read_text())["version"])
        if re.fullmatch(r"[0-9a-f]{40}", source) is None:
            raise ValueError("Expected exact GitHub source commit")
    except ValueError:
        report["configurationValid"] = False
        report["registries"] = []
        for target in report["targets"]:
            target.update(status="NOT_PUBLISHED", reason="Invalid release identity; all attempts refused")
    else:
        report.update(sourceCommit=source, tag=tag)
    # Retain failed/disabled plans as evidence. Never overwrite an earlier plan.
    with args.receipt.open("x") as output:
        output.write(json.dumps(report, indent=2) + "\n")
    if path := os.environ.get("GITHUB_OUTPUT"):
        with Path(path).open("a") as output:
            output.write("registries=" + json.dumps(report["registries"], separators=(",", ":")) + "\n")
    summary = "Marketplace readiness (not publication verification):\n\n" + "\n".join(
        f"- {target['registry']}: **{target['status']}** — {target['reason']}"
        for target in report["targets"]
    ) + "\n"
    if path := os.environ.get("GITHUB_STEP_SUMMARY"):
        with Path(path).open("a") as output:
            output.write(summary)
    print(summary, end="")
    return 0 if report["configurationValid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
