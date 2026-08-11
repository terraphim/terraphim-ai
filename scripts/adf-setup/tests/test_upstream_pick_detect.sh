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
#   patch-id   rebadged subject, no trailer, identical patch content
#   verified   an operator-attested pair from --verified-picks
#   MISSING    an upstream commit genuinely absent from the fork
#   cascade    one upstream commit answered by several fork commits
#
# And the negative cases, which are the ones that matter for a security
# detector -- each asserts that something which looks like evidence does not
# suppress a genuinely missing commit:
#   a byte-identical subject with divergent content
#   a different patch touching the same file
#   an unresolvable abbreviated trailer ref
#   a --verified-picks entry whose fork commit is not on the searched branch

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
commit_file g.txt golf    "fix: golf attested by hand"
U_GOLF=$(g rev-parse HEAD)
commit_file h.txt hotel   "fix: hotel same file different patch"
U_HOTEL=$(g rev-parse HEAD)
commit_file i.txt india   "fix: india bogus trailer"
U_INDIA=$(g rev-parse HEAD)
commit_file j.txt juliet  "fix: juliet stale attestation"
U_JULIET=$(g rev-parse HEAD)

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

# 7. Adapted by hand with no trailer and a diverged patch: only an attested
#    --verified-picks entry can vouch for this, which is how the two historical
#    [ferrox] picks that predate the trailer convention are handled.
commit_file g.txt "golf adapted for the fork" "fix: golf adapted"
F_GOLF=$(g rev-parse HEAD)

# 8. Touches the same file as U_HOTEL with different content. Nothing about this
#    is evidence, and patch-id must not be fooled by the shared path.
commit_file h.txt "hotel unrelated fork change" "chore: unrelated hotel edit"

# 9. A trailer naming a SHA that does not exist in this repository. An
#    unresolvable reference proves nothing and must not suppress.
commit_file i2.txt "india groundwork" "[ferrox] fix: india" \
    "Adapted-from: 0123456"

# 10. U_JULIET gets a --verified-picks entry below pointing at a commit that is
#     never merged into the fork branch, standing in for a stale attestation
#     left behind by a history rewrite.
g checkout -q -b abandoned
commit_file j.txt "juliet abandoned" "fix: juliet abandoned attempt"
F_JULIET_ABANDONED=$(g rev-parse HEAD)
g checkout -q fork

# ------------------------------------------------------------------- run

PICKS="${TMP}/verified-picks.tsv"
cat > "$PICKS" <<PICKS_EOF
# comment line, and a blank line, both ignored

${U_GOLF} ${F_GOLF}   # attested by hand
${U_JULIET} ${F_JULIET_ABANDONED}   # stale: fork commit is not on the fork branch
PICKS_EOF

OUT="${TMP}/verdicts"
NOTES="${TMP}/notes"
printf '%s\n' "$SEED" "$U_ALPHA" "$U_BRAVO" "$U_CHARLIE" "$U_DELTA" \
    "$U_ECHO" "$U_FOXTROT" "$U_GOLF" "$U_HOTEL" "$U_INDIA" "$U_JULIET" \
    | "$DETECT_SH" --repo "$REPO" --fork-ref fork --upstream-ref up \
        --verified-picks "$PICKS" > "$OUT" 2> "$NOTES"

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
expect "never picked"           "$U_ECHO"     MISSING -
expect "cascade"                "$U_FOXTROT"  PRESENT trailer
expect "attested pair"          "$U_GOLF"     PRESENT verified

# --- the negative cases -------------------------------------------------
#
# A subject is not provenance. The fork's d.txt commit carries U_DELTA's exact
# subject over deliberately different content; suppressing on that would let any
# same-titled fork commit hide a real upstream security fix.
expect "identical subject alone" "$U_DELTA"   MISSING -
grep -q "shares its subject with fork commit" "$NOTES" \
    || fail "identical subject: no advisory note was emitted"

# Same file, different patch. patch-id must compare content, not paths.
expect "same file different patch" "$U_HOTEL" MISSING -

# An abbreviated trailer ref that resolves to nothing. Before trailer refs were
# resolved through git this was a bare prefix comparison, so a short ref could
# vouch for any candidate sharing its leading hex digits.
expect "unresolvable trailer ref" "$U_INDIA"  MISSING -
grep -q "is ambiguous or unknown" "$NOTES" \
    || fail "unresolvable trailer: no advisory note was emitted"

# An attestation naming a fork commit that is not reachable from the searched
# branch is stale and must be dropped, not honoured.
expect "stale attestation"      "$U_JULIET"   MISSING -
grep -q "is not on fork" "$NOTES" \
    || fail "stale attestation: no advisory note was emitted"

# The patch-id hit must name the rebadged commit, not merely report a mechanism.
got=$(fork_sha_for "$U_CHARLIE")
[ "$got" = "$F_CHARLIE" ] \
    || fail "rebadged subject: matched ${got}, wanted ${F_CHARLIE}"

# A cascade resolves on its first match; the detector must not demand that every
# fork commit referencing the upstream SHA be found.
[ "$(grep -c "^PRESENT ${U_FOXTROT} " "$OUT")" = "1" ] \
    || fail "cascade: expected exactly one verdict line for ${U_FOXTROT}"

# Every candidate must be classified exactly once.
[ "$(wc -l < "$OUT" | tr -d ' ')" = "11" ] \
    || fail "expected 11 verdict lines, got $(wc -l < "$OUT" | tr -d ' ')"

# The attested verdict must name the commit the operator vouched for.
got=$(fork_sha_for "$U_GOLF")
[ "$got" = "$F_GOLF" ] \
    || fail "attested pair: matched ${got}, wanted ${F_GOLF}"

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
