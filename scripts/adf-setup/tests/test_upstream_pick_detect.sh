#!/bin/sh
# test_upstream_pick_detect.sh -- POSIX shell driver for upstream-pick-detect.sh.
#
# Builds a real git repository in a temporary directory with an "upstream"
# branch and a "fork" branch, lands the upstream commits on the fork through
# each provenance form the detector must recognise, and asserts the verdict and
# the mechanism that produced it. No mocks: every commit, trailer and patch-id
# in this test is produced by git itself.
#
# Covered (terraphim/gitea#51):
#   reachable  shared ancestor
#   trailer    `git cherry-pick -x` writes `(cherry picked from commit <sha>)`
#   trailer    the [ferrox] `Adapted-from: <abbrev> (#NNNNN)` convention
#   subject    byte-identical subject line, divergent content
#   patch-id   rebadged subject, no trailer, identical patch content
#   MISSING    an upstream commit genuinely absent from the fork
#   cascade    one upstream commit answered by several fork commits

set -eu

THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
DETECT_SH="${THIS_DIR}/../upstream-pick-detect.sh"

[ -x "$DETECT_SH" ] || { echo "FAIL: not executable: $DETECT_SH" >&2; exit 1; }

TMP="$(mktemp -d)"
trap '/bin/rm -rf "$TMP"' EXIT

REPO="${TMP}/repo"
mkdir -p "$REPO"

# The deployment box has no global git identity, so pin one on the repo.
git -C "$REPO" -c init.defaultBranch=base init -q
git -C "$REPO" config user.email test@example.com
git -C "$REPO" config user.name Test

g() { git -C "$REPO" "$@"; }

commit_file() {
    # commit_file <path> <content> <subject> [body]
    printf '%s\n' "$2" > "${REPO}/$1"
    g add "$1"
    if [ $# -ge 4 ]; then
        g commit -q -m "$3" -m "$4"
    else
        g commit -q -m "$3"
    fi
}

FAILURES=0
fail() { echo "FAIL: $*" >&2; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------- fixtures

commit_file seed seed "seed commit"
SEED=$(g rev-parse HEAD)

g checkout -q -b up
commit_file a.txt alpha   "fix(security): alpha"
U_ALPHA=$(g rev-parse HEAD)
commit_file b.txt bravo   "fix: bravo"
U_BRAVO=$(g rev-parse HEAD)
commit_file c.txt charlie "fix: charlie"
U_CHARLIE=$(g rev-parse HEAD)
commit_file d.txt delta   "fix: delta"
U_DELTA=$(g rev-parse HEAD)
commit_file e.txt echoed  "fix: echo never picked"
U_ECHO=$(g rev-parse HEAD)
commit_file f.txt foxtrot "fix: foxtrot cascade"
U_FOXTROT=$(g rev-parse HEAD)

g checkout -q -b fork "$SEED"

# 1. `-x` cherry-pick: git writes the parenthesised full-SHA trailer itself.
g cherry-pick -x "$U_ALPHA" >/dev/null 2>&1

# 2. [ferrox] adaptation: hand-applied, abbreviated SHA plus a PR reference.
U_BRAVO_ABBREV=$(printf '%s' "$U_BRAVO" | cut -c1-10)
commit_file b.txt bravo "[ferrox] fix: adapt bravo for the fork" \
    "Adapted-from: ${U_BRAVO_ABBREV} (#12345)"

# 3. Rebadged subject with no trailer at all: only patch-id can catch this.
g cherry-pick "$U_CHARLIE" >/dev/null 2>&1
g commit -q --amend -m "[ferrox] chore: unrelated-looking subject"
F_CHARLIE=$(g rev-parse HEAD)

# 4. Identical subject, divergent content: the subject check must still fire
#    where patch-id cannot.
commit_file d.txt "delta reworked for the fork" "fix: delta"

# 5. U_ECHO is deliberately never landed.

# 6. Cascade: one upstream commit answered by two fork commits.
commit_file f.txt "foxtrot part one" "[ferrox] fix: foxtrot groundwork" \
    "Adapted-from: ${U_FOXTROT}"
commit_file f2.txt "foxtrot part two" "[ferrox] fix: foxtrot follow-up" \
    "Adapted-from: ${U_FOXTROT}"

# ------------------------------------------------------------------- run

OUT="${TMP}/verdicts"
printf '%s\n' "$SEED" "$U_ALPHA" "$U_BRAVO" "$U_CHARLIE" "$U_DELTA" \
    "$U_ECHO" "$U_FOXTROT" \
    | "$DETECT_SH" --repo "$REPO" --fork-ref fork --upstream-ref up > "$OUT"

verdict_for() {
    awk -v sha="$1" '$2 == sha { print $1; exit }' "$OUT"
}
mechanism_for() {
    awk -v sha="$1" '$2 == sha { print $3; exit }' "$OUT"
}
fork_sha_for() {
    awk -v sha="$1" '$2 == sha { print $4; exit }' "$OUT"
}

expect() {
    # expect <label> <sha> <verdict> <mechanism>
    got_v=$(verdict_for "$2")
    got_m=$(mechanism_for "$2")
    [ "$got_v" = "$3" ] || fail "$1: verdict was '${got_v}', wanted '$3'"
    [ "$got_m" = "$4" ] || fail "$1: mechanism was '${got_m}', wanted '$4'"
}

# --------------------------------------------------------------- asserts

expect "shared ancestor"        "$SEED"       PRESENT reachable
expect "cherry-pick -x trailer" "$U_ALPHA"    PRESENT trailer
expect "Adapted-from trailer"   "$U_BRAVO"    PRESENT trailer
expect "rebadged subject"       "$U_CHARLIE"  PRESENT patch-id
expect "identical subject"      "$U_DELTA"    PRESENT subject
expect "never picked"           "$U_ECHO"     MISSING -
expect "cascade"                "$U_FOXTROT"  PRESENT trailer

# The patch-id hit must name the rebadged commit, not merely report a mechanism.
got=$(fork_sha_for "$U_CHARLIE")
[ "$got" = "$F_CHARLIE" ] \
    || fail "rebadged subject: matched ${got}, wanted ${F_CHARLIE}"

# A cascade resolves on its first match; the detector must not demand that every
# fork commit referencing the upstream SHA be found.
[ "$(grep -c "^PRESENT ${U_FOXTROT} " "$OUT")" = "1" ] \
    || fail "cascade: expected exactly one verdict line for ${U_FOXTROT}"

# Every candidate must be classified exactly once.
[ "$(wc -l < "$OUT" | tr -d ' ')" = "7" ] \
    || fail "expected 7 verdict lines, got $(wc -l < "$OUT" | tr -d ' ')"

# Abbreviated candidates must resolve the same way full SHAs do -- the upstream
# lists this detector consumes carry 10-character abbreviations.
ABBREV_OUT="${TMP}/abbrev"
printf '%s\n' "$(printf '%s' "$U_ALPHA" | cut -c1-10)" \
    | "$DETECT_SH" --repo "$REPO" --fork-ref fork --upstream-ref up > "$ABBREV_OUT"
[ "$(awk '{print $1, $3}' "$ABBREV_OUT")" = "PRESENT trailer" ] \
    || fail "abbreviated candidate: got '$(cat "$ABBREV_OUT")'"

# `git log --oneline` lines are the real input shape: SHA followed by a subject.
ONELINE_OUT="${TMP}/oneline"
g log --oneline -1 "$U_ECHO" \
    | "$DETECT_SH" --repo "$REPO" --fork-ref fork --upstream-ref up > "$ONELINE_OUT"
[ "$(awk '{print $1}' "$ONELINE_OUT")" = "MISSING" ] \
    || fail "oneline input: got '$(cat "$ONELINE_OUT")'"

if [ "$FAILURES" -ne 0 ]; then
    echo "test_upstream_pick_detect: ${FAILURES} failure(s)" >&2
    echo "--- verdicts ---" >&2
    cat "$OUT" >&2
    exit 1
fi

echo "test_upstream_pick_detect: all assertions passed"
