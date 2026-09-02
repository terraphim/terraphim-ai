#!/usr/bin/env bash
#
# Bind-address security gate (#3122)
#
# Flags unsafe network bind addresses that expose services on all interfaces
# without authentication -- the regression class discovered in #3115
# (terraphim-llm-proxy bound to 0.0.0.0:3456, externally reachable, no auth,
# undetected by any CI check).
#
# A bind is UNSAFE when it would make a service reachable on 0.0.0.0 (all
# interfaces) on the HOST. Two shapes are caught:
#
#   1. Explicit wildcard binds: `0.0.0.0`, `::`, `[::]` appearing in an
#      ExecStart / Environment / command / environment line that also exposes
#      a host port (docker-compose `ports:` with no loopback/private prefix).
#
#   2. Unprefixed docker-compose host ports: `ports: - "8000:8000"` binds the
#      host port on 0.0.0.0 by default (Docker documented behaviour). A
#      loopback (`127.0.0.1:NNNN:NNNN`), private-interface, or Tailscale IP
#      prefix is SAFE.
#
# A flagged line is ALLOWED if it (or the immediately preceding line) carries
# a justification marker: `# security-ok: <reason>`. This mirrors the
# `deny.toml` / `audit.toml` documented-exception philosophy.
#
# This is advisory-INDEPENDENT (unlike cargo-audit / cargo-deny): it does not
# depend on any external advisory database, so it catches a brand-new unsafe
# bind the moment it lands in a PR. Identical design family to
# scripts/check-crate-manifests.sh (#3062), scripts/check-no-atty.sh (#3072)
# and scripts/check-rust-version.sh (#3078).
#
# Scans: systemd *.service, docker-compose *.yml/*.yaml, Dockerfile*, and
# config/* files. The check needs no Rust toolchain and no network -- it runs
# in <1s.
#
# Usage: ./scripts/check-bind-addresses.sh [repo-root]
# Exit:  0 if no unsafe unannotated binds; 1 otherwise.
#
# Refs #3122

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${1:-$(dirname "$SCRIPT_DIR")}"

if [[ ! -d "$PROJECT_ROOT" ]]; then
    echo "ERROR: project root not found: $PROJECT_ROOT" >&2
    exit 2
fi

# Colours (house style -- matches scripts/check-rust-version.sh).
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; BLUE=''; NC=''
fi

echo -e "${BLUE}🛡️  Bind-Address Security Gate${NC}"
echo "============================"
echo "Scanning: ${PROJECT_ROOT#$PWD/}"
echo ""

# Walk candidate files, classify each bind-bearing line, and emit findings.
# Python prints lines of the form "<STATUS>\t<path>:<line>\t<detail>" where
# STATUS is SAFE / FINDING / OK_ANNOTATED. Bash aggregates FINDINGs into a
# non-zero exit. Anchoring on tabs makes the IPC robust to spaces in paths.
mapfile -t LINES < <(PROJECT_ROOT="$PROJECT_ROOT" python3 - <<'PY'
import os
import re
import sys

root = os.environ["PROJECT_ROOT"]

# File globs to scan. We scope narrowly to deployment/service/config surfaces
# so the gate is fast and noise-free.
SCAN_DIRS = [
    "",                 # repo root (for top-level *.service / docker-compose)
    "systemd",
    "scripts/systemd",
    "infrastructure",
    "docker",
    "config",
]
SCAN_SUFFIXES = (
    ".service",
    ".socket",
)
COMPOSE_NAMES = (
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
)
DOCKERFILE_PREFIX = "Dockerfile"

# A port mapping like "8000:8000" or "80:80" (no host-IP prefix) binds 0.0.0.0.
# A prefixed mapping "127.0.0.1:8000:8000" or "100.106.66.7:8333:8333" is safe.
# We match the docker-compose list-item form: optional quote, optional ip:,
# then port:port or port.
UNPREFIXED_PORT = re.compile(
    r"""^\s*-\s*              # list item
        ["']?                 # optional quote
        (?:                   # NO host-IP prefix present:
            (?!               #   not followed by an ip: prefix
                \d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:   # ipv4
              | \[[0-9a-fA-F:]+\]:                          # ipv6 bracketed
            )
        )
        \d{1,5}:\d{1,5}      # hostport:containerport (unprefixed => 0.0.0.0)
    """,
    re.VERBOSE,
)
# Explicit wildcard token anywhere on a bind-bearing line.
WILDCARD = re.compile(r"\b0\.0\.0\.0\b|\[?::\]?")

# Lines that actually declare a bind surface (so a bare `0.0.0.0` mention in
# prose/comment does not trip the gate). For systemd/dockerfiles we look at
# ExecStart/Environment/ENV/command; for compose we look at `ports:` items.
BIND_BEARING_KEYS = re.compile(
    r"(?:^|\s)(?:ExecStart|Environment|EnvironmentFile|ENV|env|"
    r"command|cmd|OLLAMA_HOST|TERRAPHIM_SERVER_HOSTNAME|HOST|BIND|LISTEN)\b",
    re.IGNORECASE,
)

# Justification marker: `# security-ok: <reason>` or shell-style trailing.
ANNOTATION = re.compile(r"security-ok\s*:", re.IGNORECASE)


def candidate_files():
    """Yield (relpath, abspath) for files in the scan surface."""
    seen = set()
    # 1. Compose + Dockerfile + *.service anywhere under scan dirs.
    for sub in SCAN_DIRS:
        base = os.path.join(root, sub) if sub else root
        if not os.path.isdir(base):
            continue
        for dirpath, _dirs, files in os.walk(base):
            # Skip heavy/irrelevant trees.
            parts = os.path.relpath(dirpath, root).split(os.sep)
            if any(p in {"target", "node_modules", ".git", "dist"} for p in parts):
                continue
            for name in files:
                if name.startswith(DOCKERFILE_PREFIX):
                    pass
                elif name.endswith(SCAN_SUFFIXES):
                    pass
                elif name in COMPOSE_NAMES:
                    pass
                else:
                    continue
                abspath = os.path.join(dirpath, name)
                relpath = os.path.relpath(abspath, root)
                if relpath not in seen:
                    seen.add(relpath)
                    yield relpath, abspath


def _in_annotation_block(lines, idx):
    """True if the line at 1-based idx is covered by a security-ok marker.

    A marker applies if it sits on the flagged line itself, or anywhere in the
    contiguous run of comment lines (`#` / `;`) immediately preceding it. This
    lets a multi-line `# security-ok:` rationale document the finding beneath
    it, matching how humans annotate deployment files.
    """
    if idx <= 0 or idx > len(lines):
        return False
    # On the flagged line itself.
    if ANNOTATION.search(lines[idx - 1]):
        return True
    # Walk backwards through the contiguous comment block above the line.
    j = idx - 1
    while j >= 1:
        prev = lines[j - 1]
        if ANNOTATION.search(prev):
            return True
        stripped = prev.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith(";") or stripped.startswith("//"):
            j -= 1
            continue
        # First non-comment, non-blank line ends the block.
        break
    return False


def classify_line(relpath, lineno, lines, is_compose):
    """Return a (status, detail) tuple for the line at 1-based lineno.

    status: SAFE | FINDING | OK_ANNOTATED
    """
    line = lines[lineno - 1].rstrip("\n")
    annotated = _in_annotation_block(lines, lineno)

    if is_compose:
        # docker-compose: unprefixed `ports:` list item is the host-exposure
        # surface. A bare `0.0.0.0` in a compose `environment:` is
        # container-internal (safe unless paired with an unprefixed port).
        if UNPREFIXED_PORT.search(line):
            if annotated:
                return ("OK_ANNOTATED", "unprefixed port (0.0.0.0) -- annotated")
            return ("FINDING", "unprefixed host port binds 0.0.0.0")
        return ("SAFE", "")

    # systemd / Dockerfile / config: only flag wildcard binds on lines that
    # actually declare a listener surface (ExecStart/Environment/ENV/HOST...).
    if WILDCARD.search(line) and BIND_BEARING_KEYS.search(line):
        if annotated:
            return ("OK_ANNOTATED", "wildcard bind -- annotated")
        return ("FINDING", "0.0.0.0 / :: bind on ExecStart/Environment/ENV")
    return ("SAFE", "")


findings = []
checked = 0
for relpath, abspath in candidate_files():
    if not os.path.isfile(abspath):
        continue
    is_compose = relpath.rsplit("/", 1)[-1] in COMPOSE_NAMES
    checked += 1
    try:
        with open(abspath, encoding="utf-8", errors="replace") as fh:
            lines = fh.readlines()
    except OSError:
        continue
    for idx in range(1, len(lines) + 1):
        status, detail = classify_line(relpath, idx, lines, is_compose)
        if status != "SAFE":
            print("%s\t%s:%d\t%s" % (status, relpath, idx, detail))
            if status == "FINDING":
                src = lines[idx - 1].strip()
                findings.append((relpath, idx, detail, src))

print("SCANNED\t%d" % checked)
print("FINDINGS\t%d" % len(findings))
PY
)

ok_annotated=0
findings=()
scanned=0
for line in "${LINES[@]}"; do
    status="${line%%	*}"
    rest="${line#*	}"
    case "$status" in
        OK_ANNOTATED) ok_annotated=$((ok_annotated + 1)) ;;
        FINDING) findings+=("$rest") ;;
        SCANNED) scanned="$rest" ;;
    esac
done

if [[ ${#findings[@]} -gt 0 ]]; then
    echo -e "${RED}✗ FAIL: ${#findings[@]} unsafe unannotated bind(s)${NC}"
    for f in "${findings[@]}"; do
        echo -e "  ${RED}- ${f}${NC}"
    done
    echo ""
    echo -e "${YELLOW}These lines bind a service on all interfaces (0.0.0.0) without a security${NC}"
    echo -e "${YELLOW}justification. Either bind to a loopback/private interface, OR annotate the${NC}"
    echo -e "${YELLOW}line (or the line directly above it) with:${NC}"
    echo -e "${YELLOW}    # security-ok: <one-line justification>${NC}"
    echo ""
    echo "Scanned ${scanned} service/config file(s); ${#findings[@]} unsafe bind(s), ${ok_annotated} annotated exception(s)."
    exit 1
fi

echo -e "${GREEN}✓ PASS: no unsafe unannotated binds${NC}"
echo "Scanned ${scanned} service/config file(s); ${ok_annotated} annotated exception(s)."
exit 0
