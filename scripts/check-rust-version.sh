#!/usr/bin/env bash
#
# Workspace rust-version declaration gate (#3078)
#
# Verifies that every effective workspace member Cargo.toml declares a
# `rust-version` (either a literal string, e.g. `rust-version = "1.91"`, or
# the inheritance form `rust-version.workspace = true`).
#
# This is the CI-enforcement twin of #3071/#3108: those issues filled in the
# missing declarations, but nothing structurally prevents a NEW crate from
# being added without one -- which is exactly how #2754 happened
# (terraphim_rlm v1.20.5 shipped a Rust 1.91 API with no declared MSRV).
# cargo-deny / clippy MSRV lints are advisory-dependent and only fire on the
# crate that breaks; this gate fails fast, at PR time, listing every offender.
#
# The check replicates Cargo's own member resolution (expand `members` globs,
# subtract `exclude`) WITHOUT invoking `cargo`, so it runs in <1s and needs no
# Rust toolchain -- identical design to scripts/check-crate-manifests.sh (#3062).
#
# Usage: ./scripts/check-rust-version.sh [path/to/Cargo.toml]
# Exit:  0 if all effective members declare rust-version; 1 otherwise.
#
# Refs #3078

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Allow overriding the manifest (used by the self-test for temp fixtures).
ROOT_MANIFEST="${1:-$PROJECT_ROOT/Cargo.toml}"

if [[ ! -f "$ROOT_MANIFEST" ]]; then
    echo "ERROR: root manifest not found: $ROOT_MANIFEST" >&2
    exit 2
fi

# Colours (house style — matches scripts/check-crate-manifests.sh).
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; BLUE=''; NC=''
fi

echo -e "${BLUE}📋 Workspace rust-version Declaration Gate${NC}"
echo "=========================================="
echo "Root manifest: ${ROOT_MANIFEST#$PROJECT_ROOT/}"
echo ""

# Resolve members/exclude and verify each effective member declares rust-version.
# Python prints lines of the form "OK <path> [<detail>]" or "MISSING <path>";
# the last line is "COUNT <n>". Bash translates MISSING into a non-zero exit.
#
# A member is considered compliant if its [package] table contains a
# `rust-version` key that is EITHER a string (literal) OR a table containing
# `workspace = true` (inheritance). Any other shape (or absence) is MISSING.
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
        entry = entry.replace("\\", "/")
        path = os.path.join(base, entry) if not os.path.isabs(entry) else entry
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


def declare_kind(pkg):
    """Return (is_declared, detail) for a package table's rust-version field."""
    rv = pkg.get("rust-version")
    if rv is None:
        return (False, "absent")
    # Literal: rust-version = "1.91"
    if isinstance(rv, str):
        return (True, "literal %s" % rv)
    # Inheritance: rust-version.workspace = true
    if isinstance(rv, dict):
        if rv.get("workspace") is True:
            return (True, "inherited")
        return (False, "malformed table %r" % rv)
    # Numbers / bools are not valid rust-version values.
    return (False, "malformed %r" % (rv,))


effective = expand_members(member_globs, root_dir)

missing = []
for rel in effective:
    manifest = os.path.join(root_dir, rel, "Cargo.toml")
    if not os.path.isfile(manifest):
        # A missing manifest is a different failure (#3062 gate). We only own
        # the rust-version contract here, so skip members with no Cargo.toml.
        continue
    with open(manifest, "rb") as fh:
        member = tomllib.load(fh)
    pkg = member.get("package") or {}
    declared, detail = declare_kind(pkg)
    name = pkg.get("name", rel)
    if declared:
        print("OK %s [%s -> %s]" % (rel, name, detail))
    else:
        missing.append((rel, name, detail))
        print("MISSING %s [%s -> %s]" % (rel, name, detail))

print("COUNT %d" % len(effective))
PY
)

checked_ok=0
missing=()
for line in "${LINES[@]}"; do
    tag="${line%% *}"
    rest="${line#* }"
    case "$tag" in
        OK) checked_ok=$((checked_ok + 1)) ;;
        MISSING) missing+=("$rest") ;;
        COUNT) total="$rest" ;;
    esac
done

if [[ ${#missing[@]} -gt 0 ]]; then
    echo -e "${RED}✗ FAIL: ${#missing[@]} workspace member(s) missing rust-version${NC}"
    for m in "${missing[@]}"; do
        echo -e "  ${RED}- ${m}${NC}"
    done
    echo ""
    echo -e "${YELLOW}These effective workspace members do not declare [package] rust-version.${NC}"
    echo -e "${YELLOW}Add one of (in the member's Cargo.toml):${NC}"
    echo -e "${YELLOW}    rust-version.workspace = true   # inherit the workspace MSRV${NC}"
    echo -e "${YELLOW}  or${NC}"
    echo -e "${YELLOW}    rust-version = \"1.91\"            # literal MSRV${NC}"
    echo ""
    echo "Checked ${total:-0} effective member(s), ${#missing[@]} missing rust-version."
    exit 1
fi

echo -e "${GREEN}✓ PASS: all ${checked_ok} declared member(s) declare rust-version${NC}"
echo "Checked ${total:-0} effective member(s)."
exit 0
