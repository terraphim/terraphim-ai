#!/usr/bin/env bash
#
# Workspace crate-manifest completeness gate (#3062)
#
# Verifies that every effective workspace member has a Cargo.toml, preventing
# the P0 "missing manifest" build breakage seen in #3030.
#
# The check replicates Cargo's own member resolution (expand `members` globs,
# subtract `exclude`) WITHOUT invoking `cargo`, so it catches the exact breakage
# that `cargo metadata` itself trips over.
#
# Usage: ./scripts/check-crate-manifests.sh [path/to/Cargo.toml]
# Exit:  0 if all effective members have manifests; 1 otherwise.
#
# Refs #3062

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Allow overriding the manifest (used by the self-test for temp fixtures).
ROOT_MANIFEST="${1:-$PROJECT_ROOT/Cargo.toml}"

if [[ ! -f "$ROOT_MANIFEST" ]]; then
    echo "ERROR: root manifest not found: $ROOT_MANIFEST" >&2
    exit 2
fi

# Colours (house style — matches scripts/ci-check-rust.sh).
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; BLUE=''; NC=''
fi

echo -e "${BLUE}📋 Crate Manifest Completeness Gate${NC}"
echo "==================================="
echo "Root manifest: ${ROOT_MANIFEST#$PROJECT_ROOT/}"
echo ""

# Resolve members/exclude and verify each effective member has Cargo.toml.
# Python prints lines of the form "OK <path>" or "MISSING <path>"; the last
# line is "COUNT <n>". Bash translates MISSING into a non-zero exit.
mapfile -t LINES < <(ROOT_MANIFEST="$ROOT_MANIFEST" python3 - <<'PY'
import os
import sys
import tomllib

root = os.environ["ROOT_MANIFEST"]
root_dir = os.path.dirname(os.path.abspath(root))

with open(root, "rb") as fh:
    data = tomllib.load(fh)

workspace = data.get("workspace") or {}
member_globs = workspace.get("members") or []
excludes = set(workspace.get("exclude") or [])


def expand_members(globs, base):
    """Expand workspace members, honouring excludes exactly like Cargo."""
    candidates = []
    for entry in globs:
        # Normalise backslashes just in case.
        entry = entry.replace("\\", "/")
        path = os.path.join(base, entry) if not os.path.isabs(entry) else entry
        # Cargo treats a member entry as either a direct directory or a glob.
        if any(ch in entry for ch in "*?["):
            import glob

            for match in sorted(glob.glob(path)):
                if os.path.isdir(match):
                    candidates.append(match)
        elif os.path.isdir(path):
            candidates.append(path)

    effective = []
    seen = set()
    for cand in candidates:
        norm = os.path.relpath(cand, base).replace("\\", "/")
        if norm in excludes:
            continue
        if norm in seen:
            continue
        seen.add(norm)
        effective.append(norm)
    return sorted(effective)


effective = expand_members(member_globs, root_dir)

missing = []
for rel in effective:
    manifest = os.path.join(root_dir, rel, "Cargo.toml")
    if os.path.isfile(manifest):
        print("OK %s" % rel)
    else:
        missing.append(rel)
        print("MISSING %s" % rel)

print("COUNT %d" % len(effective))
PY
)

checked=0
missing=()
for line in "${LINES[@]}"; do
    tag="${line%% *}"
    rest="${line#* }"
    case "$tag" in
        OK) checked=$((checked + 1)) ;;
        MISSING) missing+=("$rest") ;;
        COUNT) total="$rest" ;;
    esac
done

if [[ ${#missing[@]} -gt 0 ]]; then
    echo -e "${RED}✗ FAIL: ${#missing[@]} workspace member(s) missing Cargo.toml${NC}"
    for m in "${missing[@]}"; do
        echo -e "  ${RED}- ${m}/Cargo.toml${NC}"
    done
    echo ""
    echo -e "${YELLOW}These paths are effective workspace members (matched members[], not in exclude[])${NC}"
    echo -e "${YELLOW}but contain no Cargo.toml. Add the directory to exclude[], or restore its manifest.${NC}"
    echo ""
    echo "Checked ${total:-0} effective member(s), ${#missing[@]} missing."
    exit 1
fi

echo -e "${GREEN}✓ PASS: all ${total:-0} effective workspace member(s) have a Cargo.toml${NC}"
exit 0
