# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///

"""
Flow State Parser for ADF Shared Workspace.

Parses flow state JSON files to extract step failure patterns, model errors,
timing anomalies, and recurrence counts. Produces JSONL output for SQLite
schema design in Phase 1.

Usage:
    uv run scripts/adf/flow-state-parser.py /opt/ai-dark-factory/flow-states/

Output:
    JSONL to stdout with one learning record per line.
"""

import json
import re
import sys
from collections import Counter
from datetime import datetime
from pathlib import Path
from typing import Any


def extract_model_errors(step: dict, flow_id: str) -> list[dict]:
    """Extract model-related errors from step stderr."""
    errors = []
    stderr = step.get("stderr", "")

    # Pattern: ProviderModelNotFoundError
    if "ProviderModelNotFoundError" in stderr or "Model not found" in stderr:
        # Extract model info
        model_match = re.search(
            r'(?:modelID|model)["\']?\s*[:=]\s*["\']?([^"\'\s,}]+)', stderr, re.I
        )
        provider_match = re.search(
            r'(?:providerID|provider)["\']?\s*[:=]\s*["\']?([^"\'\s,}]+)', stderr, re.I
        )

        model = model_match.group(1) if model_match else "unknown"
        provider = provider_match.group(1) if provider_match else "unknown"

        errors.append(
            {
                "source_agent": step.get("step_name", "unknown"),
                "category": "model_error",
                "summary": f"ProviderModelNotFoundError: {provider}/{model}",
                "details": stderr[:500],  # Truncate for readability
                "confidence": 0.95,
                "timestamp": step.get("finished_at") or step.get("started_at"),
                "flow_correlation_id": flow_id,
                "error_type": "ProviderModelNotFoundError",
                "provider": provider,
                "model": model,
            }
        )

    # Pattern: Generic errors in stderr
    elif stderr and len(stderr) > 0 and step.get("exit_code", 0) != 0:
        error_lines = [
            line
            for line in stderr.split("\n")
            if line.strip() and not line.strip().startswith("at ")
        ]
        if error_lines:
            errors.append(
                {
                    "source_agent": step.get("step_name", "unknown"),
                    "category": "step_failure",
                    "summary": error_lines[0][:100],
                    "details": stderr[:500],
                    "confidence": 0.8,
                    "timestamp": step.get("finished_at") or step.get("started_at"),
                    "flow_correlation_id": flow_id,
                    "error_type": "StepFailure",
                    "exit_code": step.get("exit_code"),
                }
            )

    return errors


def extract_timing_anomalies(flow: dict) -> list[dict]:
    """Extract timing anomalies from flow execution."""
    anomalies = []
    steps = flow.get("step_envelopes", [])
    flow_id = flow.get("correlation_id", "unknown")

    for i, step in enumerate(steps):
        started = step.get("started_at")
        finished = step.get("finished_at")

        if started and finished:
            try:
                start_dt = datetime.fromisoformat(started.replace("Z", "+00:00"))
                finish_dt = datetime.fromisoformat(finished.replace("Z", "+00:00"))
                duration = (finish_dt - start_dt).total_seconds()

                # Flag steps taking longer than 30 seconds
                if duration > 30:
                    anomalies.append(
                        {
                            "source_agent": step.get("step_name", "unknown"),
                            "category": "timing_anomaly",
                            "summary": f"Step took {duration:.1f}s (threshold: 30s)",
                            "details": f"Duration: {duration}s, Exit code: {step.get('exit_code')}",
                            "confidence": 0.7,
                            "timestamp": finished,
                            "flow_correlation_id": flow_id,
                            "duration_seconds": duration,
                        }
                    )
            except (ValueError, TypeError):
                pass

    return anomalies


def identify_recurring_patterns(learnings: list[dict]) -> list[dict]:
    """Identify patterns that recur across multiple flows."""
    # Group by error signature
    signatures = Counter()

    for learning in learnings:
        if learning["category"] == "model_error":
            sig = (
                learning.get("error_type"),
                learning.get("provider"),
                learning.get("model"),
            )
            signatures[sig] += 1
        elif learning["category"] == "step_failure":
            sig = (
                learning["category"],
                learning["source_agent"],
                learning.get("exit_code"),
            )
            signatures[sig] += 1

    # Mark recurring patterns
    recurring = []
    for learning in learnings:
        if learning["category"] == "model_error":
            sig = (
                learning.get("error_type"),
                learning.get("provider"),
                learning.get("model"),
            )
            count = signatures[sig]
            if count > 1:
                recurring.append(
                    {
                        **learning,
                        "category": "recurring_pattern",
                        "summary": f"[Recurring {count}x] {learning['summary']}",
                        "recurrence_count": count,
                    }
                )
            else:
                recurring.append(learning)
        elif learning["category"] == "step_failure":
            sig = (
                learning["category"],
                learning["source_agent"],
                learning.get("exit_code"),
            )
            count = signatures[sig]
            if count > 1:
                recurring.append(
                    {
                        **learning,
                        "category": "recurring_pattern",
                        "summary": f"[Recurring {count}x] {learning['summary']}",
                        "recurrence_count": count,
                    }
                )
            else:
                recurring.append(learning)
        else:
            recurring.append(learning)

    return recurring


def parse_flow_file(filepath: Path) -> list[dict]:
    """Parse a single flow state file and extract learnings."""
    learnings = []

    try:
        with open(filepath, "r") as f:
            flow = json.load(f)
    except (json.JSONDecodeError, IOError) as e:
        print(f"Warning: Failed to parse {filepath}: {e}", file=sys.stderr)
        return []

    flow_id = flow.get("correlation_id", filepath.stem)
    steps = flow.get("step_envelopes", [])

    for step in steps:
        # Extract model errors
        errors = extract_model_errors(step, flow_id)
        learnings.extend(errors)

        # Check for other failure patterns
        if step.get("exit_code", 0) != 0 and not errors:
            learnings.append(
                {
                    "source_agent": step.get("step_name", "unknown"),
                    "category": "step_failure",
                    "summary": f"Non-zero exit code: {step.get('exit_code')}",
                    "details": step.get("stderr", "")[:500]
                    or step.get("stdout", "")[:500],
                    "confidence": 0.75,
                    "timestamp": step.get("finished_at") or step.get("started_at"),
                    "flow_correlation_id": flow_id,
                    "exit_code": step.get("exit_code"),
                }
            )

    # Extract timing anomalies
    anomalies = extract_timing_anomalies(flow)
    learnings.extend(anomalies)

    return learnings


def main() -> int:
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: flow-state-parser.py <flow-states-directory>", file=sys.stderr)
        print(
            "Example: uv run scripts/adf/flow-state-parser.py /opt/ai-dark-factory/flow-states/",
            file=sys.stderr,
        )
        return 1

    flow_dir = Path(sys.argv[1])

    if not flow_dir.exists():
        print(f"Error: Directory not found: {flow_dir}", file=sys.stderr)
        return 1

    # Collect all learnings
    all_learnings = []
    flow_files = sorted(flow_dir.glob("*.json"))

    for filepath in flow_files:
        learnings = parse_flow_file(filepath)
        all_learnings.extend(learnings)

    # Identify recurring patterns
    all_learnings = identify_recurring_patterns(all_learnings)

    # Output JSONL
    for learning in all_learnings:
        # Clean up internal fields not in spec
        output = {
            "source_agent": learning["source_agent"],
            "category": learning["category"],
            "summary": learning["summary"],
            "details": learning["details"],
            "confidence": learning["confidence"],
            "timestamp": learning["timestamp"],
            "flow_correlation_id": learning["flow_correlation_id"],
        }

        # Add recurrence count if present
        if "recurrence_count" in learning:
            output["recurrence_count"] = learning["recurrence_count"]

        print(json.dumps(output, ensure_ascii=False))

    # Print summary to stderr
    print(
        f"\nParsed {len(flow_files)} flow files, extracted {len(all_learnings)} learnings",
        file=sys.stderr,
    )

    categories = Counter(l["category"] for l in all_learnings)
    for cat, count in sorted(categories.items()):
        print(f"  {cat}: {count}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
