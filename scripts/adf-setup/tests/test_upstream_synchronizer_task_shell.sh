#!/bin/sh
# test_upstream_synchronizer_task_shell.sh -- runtime behaviour of the agent task.
#
# The static TOML contract tests assert that the wiring is present; they cannot
# see control flow. This driver extracts the real task string from both configs
# and executes the candidate-scan block against a real git repository, under
# `set -e`, so the no-match path is exercised rather than merely inspected.
#
# The specific regression (terraphim/gitea#51 review): the candidate scan ends in
# `grep`, which exits 1 when nothing matches. Under `set -e` that takes the whole
# assignment -- and the run -- down before the "no security-relevant commits"
# branch is reached. The old pipeline ended in `head`, so it never surfaced.
#
# Note on scope: the deployed `task` is delivered to a CLI agent as a *prompt*,
# not executed by `sh -c` (see terraphim_orchestrator spawn_impl.rs, which
# composes it into the agent prompt). This test therefore pins that the block is
# safe to run under `set -e`, which is the contract an agent copying it into a
# shell should be able to rely on. It does not claim the orchestrator itself
# sets `-e`.

set -eu

THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
ADF_SETUP="$(cd "${THIS_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${ADF_SETUP}/../.." && pwd)"

TEMPLATE="${ADF_SETUP}/agents/upstream-synchronizer.toml"
BIGBOX="${REPO_ROOT}/.terraphim/terraphim.toml.bigbox"

TMP="$(mktemp -d)"
trap '/bin/rm -rf "$TMP"' EXIT

FAILURES=0
fail() { echo "FAIL: $*" >&2; FAILURES=$((FAILURES + 1)); }

# A real repository whose upstream branch is ahead but contains nothing the
# security grep matches -- the exact no-candidate state.
REPO="${TMP}/repo"
mkdir -p "$REPO"
git -C "$REPO" -c init.defaultBranch=main init -q
git -C "$REPO" config user.email test@example.com
git -C "$REPO" config user.name Test
echo seed > "${REPO}/seed"
git -C "$REPO" add seed
git -C "$REPO" commit -q -m "seed"
git -C "$REPO" branch -M main
git -C "$REPO" checkout -q -b upstream-main
echo doc > "${REPO}/doc"
git -C "$REPO" add doc
git -C "$REPO" commit -q -m "docs: tidy the changelog wording"
git -C "$REPO" checkout -q main
# The task compares against origin/main and upstream/main; provide both as real
# refs rather than stubbing git.
git -C "$REPO" update-ref refs/remotes/origin/main refs/heads/main
git -C "$REPO" update-ref refs/remotes/upstream/main refs/heads/upstream-main

extract_scan_block() {
    # Pull the task string out of a TOML config and keep the candidate scan,
    # which is the part whose exit status is under test.
    python3 - "$1" <<'PY'
import sys
try:
    import tomllib
except ImportError:
    import tomli as tomllib
with open(sys.argv[1], "rb") as fh:
    config = tomllib.load(fh)
for agent in config.get("agents", []):
    if agent.get("name") == "upstream-synchronizer":
        task = agent["task"]
        break
else:
    raise SystemExit("no upstream-synchronizer agent")

start = task.index("CANDIDATES=$(git log")
end = task.index("if [ -n \"$CANDIDATES\" ]; then", start)
sys.stdout.write(task[start:end])
PY
}

for config in "$TEMPLATE" "$BIGBOX"; do
    name=$(basename "$config")
    [ -f "$config" ] || { fail "${name}: missing config"; continue; }

    block="${TMP}/scan-$(echo "$name" | tr './' '__').sh"
    {
        echo 'set -e'
        echo 'cd "$1"'
        extract_scan_block "$config"
        echo 'echo "REACHED-INFORMATIONAL-BRANCH candidates=[$CANDIDATES]"'
    } > "$block"

    if out=$(sh "$block" "$REPO" 2>&1); then
        case "$out" in
            *REACHED-INFORMATIONAL-BRANCH*candidates=\[\]*) ;;
            *) fail "${name}: unexpected output: ${out}" ;;
        esac
    else
        fail "${name}: scan aborted under set -e with no matching candidates: ${out}"
    fi
done

# The guard must survive an actual match too -- `|| true` must not swallow the
# candidate list itself.
git -C "$REPO" checkout -q upstream-main
echo patch > "${REPO}/patch"
git -C "$REPO" add patch
git -C "$REPO" commit -q -m "fix(security): plug an information leak"
git -C "$REPO" checkout -q main
git -C "$REPO" update-ref refs/remotes/upstream/main refs/heads/upstream-main

block="${TMP}/scan-match.sh"
{
    echo 'set -e'
    echo 'cd "$1"'
    extract_scan_block "$TEMPLATE"
    echo 'echo "MATCHED=[$CANDIDATES]"'
} > "$block"

out=$(sh "$block" "$REPO" 2>&1) || fail "matching scan exited non-zero: ${out}"
case "$out" in
    *"plug an information leak"*) ;;
    *) fail "matching scan lost the candidate: ${out}" ;;
esac

if [ "$FAILURES" -ne 0 ]; then
    echo "test_upstream_synchronizer_task_shell: ${FAILURES} failure(s)" >&2
    exit 1
fi

echo "test_upstream_synchronizer_task_shell: all assertions passed"
