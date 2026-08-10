#!/bin/sh
# upstream-pick-detect.sh -- classify upstream commits as already-picked or missing.
#
# The upstream-synchronizer agent used to decide "missing" purely from SHA
# reachability (`git log HEAD..upstream/main`). Cherry-picks get fresh SHAs and
# [ferrox] adaptations rebadge the subject, so every pick the fork had already
# landed kept resurfacing as missing (terraphim/gitea#51, parent #43).
#
# Reads candidate upstream commits on stdin, one per line, in `git log --oneline`
# form (`<sha> <subject>`) or as a bare SHA. Writes one verdict line per
# candidate to stdout:
#
#   PRESENT <upstream-sha> <mechanism> <fork-sha> <subject>
#   MISSING <upstream-sha> - - <subject>
#
# Mechanisms, applied in ascending cost and reported by first match:
#
#   reachable  upstream commit is an ancestor of the fork ref (merged, not picked)
#   trailer    a fork commit carries `(cherry picked from commit <sha>)` or
#              `Adapted-from: <sha>` naming this upstream commit
#   verified   an operator-attested (upstream, fork) pair from --verified-picks
#   patch-id   `git patch-id --stable` equivalence
#
# Every mechanism above is either provenance (the fork commit names the upstream
# commit), content equivalence (patch-id), or a recorded human verification.
# A matching subject is NOT one of them: subjects collide, generic wording such
# as "Fix basic auth bug" recurs, and treating one as proof would let the fork's
# own unrelated commit hide a genuinely missing security fix. Subject collisions
# are reported as advisory notes on stderr and never change a verdict.
#
# Exit status is 0 when every candidate was classified, 1 on usage or git error.
# The verdict stream on stdout is the product; callers filter on the leading
# field. Advisory notes go to stderr so the verdict stream stays parseable.

set -eu

REPO="."
FORK_REF="origin/main"
UPSTREAM_REF="upstream/main"
WINDOW=500
VERIFIED_FILE=""

usage() {
    cat >&2 <<'USAGE'
Usage: upstream-pick-detect.sh [options] < candidates

  --repo PATH            fork working tree (default: .)
  --fork-ref REF         fork branch to search (default: origin/main)
  --upstream-ref REF     upstream branch (default: upstream/main)
  --window N             max fork commits to index (default: 500)
  --verified-picks FILE  operator-attested "<upstream-sha> <fork-sha>" pairs,
                         one per line, '#' comments allowed

Candidates are read from stdin as `<sha> [subject]` lines.
USAGE
    exit 1
}

while [ $# -gt 0 ]; do
    case "$1" in
        --repo) REPO="${2:?--repo needs a value}"; shift 2 ;;
        --fork-ref) FORK_REF="${2:?--fork-ref needs a value}"; shift 2 ;;
        --upstream-ref) UPSTREAM_REF="${2:?--upstream-ref needs a value}"; shift 2 ;;
        --window) WINDOW="${2:?--window needs a value}"; shift 2 ;;
        --verified-picks) VERIFIED_FILE="${2:?--verified-picks needs a value}"; shift 2 ;;
        -h|--help) usage ;;
        *) echo "upstream-pick-detect: unknown option: $1" >&2; usage ;;
    esac
done

git_fork() {
    git -C "$REPO" "$@"
}

if ! git_fork rev-parse --git-dir >/dev/null 2>&1; then
    echo "upstream-pick-detect: not a git repository: $REPO" >&2
    exit 1
fi

for ref in "$FORK_REF" "$UPSTREAM_REF"; do
    if ! git_fork rev-parse --verify --quiet "$ref" >/dev/null; then
        echo "upstream-pick-detect: unknown ref: $ref" >&2
        exit 1
    fi
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

TRAILERS="${WORK}/trailers"   # <referenced-full-oid> <fork-sha>
VERIFIED="${WORK}/verified"   # <upstream-full-oid> <fork-full-oid>
SUBJECTS="${WORK}/subjects"   # <fork-sha> <TAB> <subject>
PATCHIDS="${WORK}/patchids"   # <patch-id> <fork-sha>
: > "$TRAILERS"
: > "$SUBJECTS"
: > "$PATCHIDS"
: > "$VERIFIED"

# Operator-attested pairs. Each entry is a claim that a human checked this fork
# commit against this upstream commit; both ends are re-validated here so a typo
# or a rewritten history cannot silently suppress a real gap. Entries that fail
# validation are dropped with a note rather than aborting the run -- on a shallow
# clone an unresolvable entry is expected, and reporting MISSING is the safe way
# to fail.
if [ -n "$VERIFIED_FILE" ]; then
    if [ ! -r "$VERIFIED_FILE" ]; then
        echo "upstream-pick-detect: cannot read --verified-picks file: $VERIFIED_FILE" >&2
        exit 1
    fi
    set -f  # field-splitting below must not glob a '*' in the file
    while IFS= read -r vline || [ -n "$vline" ]; do
        vline=${vline%%#*}
        set -- $vline
        [ $# -ge 2 ] || continue
        v_up=$(git_fork rev-parse --verify --quiet "${1}^{commit}" || true)
        v_fork=$(git_fork rev-parse --verify --quiet "${2}^{commit}" || true)
        if [ -z "$v_up" ] || [ -z "$v_fork" ]; then
            echo "upstream-pick-detect: verified-picks entry does not resolve, ignoring: $1 $2" >&2
            continue
        fi
        # The attested fork commit must actually be on the branch being searched.
        if ! git_fork merge-base --is-ancestor "$v_fork" "$FORK_REF" 2>/dev/null; then
            echo "upstream-pick-detect: verified-picks fork commit ${2} is not on ${FORK_REF}, ignoring" >&2
            continue
        fi
        printf '%s %s\n' "$v_up" "$v_fork" >> "$VERIFIED"
    done < "$VERIFIED_FILE"
    set +f
fi

# Index the fork's divergent commits once. Commits the fork shares with upstream
# need no index: reachability already answers for those. Bounding the walk at
# UPSTREAM_REF..FORK_REF is the divergence point the issue asks for, and --window
# caps the work on a long-lived fork.
FORK_COMMITS="${WORK}/fork-commits"
git_fork rev-list --no-merges --max-count="$WINDOW" \
    "${UPSTREAM_REF}..${FORK_REF}" > "$FORK_COMMITS"

while read -r fork_sha; do
    [ -n "$fork_sha" ] || continue

    git_fork log -1 --format=%s "$fork_sha" \
        | sed "s|^|${fork_sha}\t|" >> "$SUBJECTS"

    # Both provenance conventions. `-x` writes `(cherry picked from commit <40-hex>)`
    # unkeyed and parenthesised; the [ferrox] convention writes an `Adapted-from:`
    # trailer whose SHA is usually abbreviated and often followed by `(#NNNNN)`.
    # Repetition intervals ({7,40}) are not portable across awk implementations
    # -- mawk on the deployment box lacks them -- so the seven-plus-hex-digit
    # requirement is spelled out longhand.
    git_fork log -1 --format=%B "$fork_sha" | awk '
        BEGIN { hex = "[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]+" }
        match($0, "cherry picked from commit " hex) {
            s = substr($0, RSTART, RLENGTH)
            sub(/^cherry picked from commit /, "", s)
            print s
        }
        match($0, "^[ \t]*Adapted-from:[ \t]*" hex) {
            s = substr($0, RSTART, RLENGTH)
            sub(/^[ \t]*Adapted-from:[ \t]*/, "", s)
            print s
        }
    ' | while read -r ref; do
        # A prefix comparison would let any candidate sharing an abbreviation's
        # leading hex digits be marked present, so abbreviations are resolved by
        # git and only full object IDs are ever compared. git rev-parse fails on
        # an ambiguous abbreviation, which is exactly the outcome wanted: an
        # abbreviation that does not uniquely identify a commit is not proof of
        # anything and must not suppress.
        if [ ${#ref} -eq 40 ]; then
            # Already a full object ID -- `git cherry-pick -x` always writes one.
            # Comparing it directly keeps -x picks working on a shallow clone
            # where the referenced commit may not be present locally.
            printf '%s %s\n' "$ref" "$fork_sha" >> "$TRAILERS"
        else
            ref_full=$(git_fork rev-parse --verify --quiet "${ref}^{commit}" || true)
            if [ -n "$ref_full" ]; then
                printf '%s %s\n' "$ref_full" "$fork_sha" >> "$TRAILERS"
            else
                echo "upstream-pick-detect: trailer ref '${ref}' in ${fork_sha} is ambiguous or unknown, ignoring" >&2
            fi
        fi
    done
done < "$FORK_COMMITS"

# patch-id is the expensive index, so it is built lazily: only if some candidate
# survives the three cheaper checks.
patchids_built=0
build_patchids() {
    [ "$patchids_built" -eq 0 ] || return 0
    patchids_built=1
    while read -r fork_sha; do
        [ -n "$fork_sha" ] || continue
        pid=$(git_fork show "$fork_sha" 2>/dev/null \
            | git_fork patch-id --stable 2>/dev/null \
            | cut -d' ' -f1)
        [ -n "$pid" ] && echo "$pid $fork_sha" >> "$PATCHIDS"
    done < "$FORK_COMMITS"
    return 0
}

# Both sides are full object IDs by construction, so equality is exact.
lookup_trailer() {
    awk -v full="$1" '$1 == full { print $2; exit }' "$TRAILERS"
}

lookup_verified() {
    awk -v full="$1" '$1 == full { print $2; exit }' "$VERIFIED"
}

lookup_subject() {
    awk -F'\t' -v want="$1" '$2 == want { print $1; exit }' "$SUBJECTS"
}

while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
        ''|'#'*) continue ;;
    esac

    cand=$(printf '%s\n' "$line" | awk '{print $1}')
    full=$(git_fork rev-parse --verify --quiet "${cand}^{commit}" || true)
    if [ -z "$full" ]; then
        echo "upstream-pick-detect: cannot resolve candidate: $cand" >&2
        continue
    fi
    subject=$(git_fork log -1 --format=%s "$full")

    if git_fork merge-base --is-ancestor "$full" "$FORK_REF" 2>/dev/null; then
        printf 'PRESENT %s reachable %s %s\n' "$cand" "$full" "$subject"
        continue
    fi

    hit=$(lookup_trailer "$full")
    if [ -n "$hit" ]; then
        printf 'PRESENT %s trailer %s %s\n' "$cand" "$hit" "$subject"
        continue
    fi

    hit=$(lookup_verified "$full")
    if [ -n "$hit" ]; then
        printf 'PRESENT %s verified %s %s\n' "$cand" "$hit" "$subject"
        continue
    fi

    build_patchids
    cand_pid=$(git_fork show "$full" 2>/dev/null \
        | git_fork patch-id --stable 2>/dev/null \
        | cut -d' ' -f1)
    # An empty patch-id (merge commit, empty diff) must never match. Only
    # non-empty ids are indexed, and the lookup is skipped entirely when the
    # candidate has none, so two blanks can never compare equal.
    if [ -n "$cand_pid" ]; then
        hit=$(awk -v pid="$cand_pid" 'pid != "" && $1 == pid { print $2; exit }' "$PATCHIDS")
        if [ -n "$hit" ]; then
            printf 'PRESENT %s patch-id %s %s\n' "$cand" "$hit" "$subject"
            continue
        fi
    fi

    # Advisory only. A shared subject is worth an operator's attention -- it is
    # how the two historical [ferrox] picks were originally spotted -- but it is
    # not provenance, so it never suppresses. Confirmed pairs belong in
    # --verified-picks, where both ends are validated.
    hit=$(lookup_subject "$subject")
    if [ -n "$hit" ]; then
        echo "upstream-pick-detect: note: ${cand} shares its subject with fork commit ${hit}; not treated as proof" >&2
    fi

    printf 'MISSING %s - - %s\n' "$cand" "$subject"
done
