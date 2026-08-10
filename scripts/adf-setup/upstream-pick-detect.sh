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
#   subject    a fork commit has a byte-identical subject line
#   patch-id   `git patch-id --stable` equivalence, as a last resort
#
# Exit status is 0 when every candidate was classified, 1 on usage or git error.
# The verdict stream is the product; callers filter on the leading field.

set -eu

REPO="."
FORK_REF="origin/main"
UPSTREAM_REF="upstream/main"
WINDOW=500

usage() {
    cat >&2 <<'USAGE'
Usage: upstream-pick-detect.sh [options] < candidates

  --repo PATH          fork working tree (default: .)
  --fork-ref REF       fork branch to search (default: origin/main)
  --upstream-ref REF   upstream branch (default: upstream/main)
  --window N           max fork commits to index (default: 500)

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

TRAILERS="${WORK}/trailers"   # <referenced-sha> <fork-sha>
SUBJECTS="${WORK}/subjects"   # <fork-sha> <TAB> <subject>
PATCHIDS="${WORK}/patchids"   # <patch-id> <fork-sha>
: > "$TRAILERS"
: > "$SUBJECTS"
: > "$PATCHIDS"

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
    git_fork log -1 --format=%B "$fork_sha" | awk -v fork="$fork_sha" '
        BEGIN { hex = "[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]+" }
        match($0, "cherry picked from commit " hex) {
            s = substr($0, RSTART, RLENGTH)
            sub(/^cherry picked from commit /, "", s)
            print s, fork
        }
        match($0, "^[ \t]*Adapted-from:[ \t]*" hex) {
            s = substr($0, RSTART, RLENGTH)
            sub(/^[ \t]*Adapted-from:[ \t]*/, "", s)
            print s, fork
        }
    ' >> "$TRAILERS"
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

# A trailer may abbreviate the SHA it names and the candidate list may abbreviate
# too, so compare on the shorter of the two, never below 7 characters.
lookup_trailer() {
    awk -v full="$1" '
        {
            ref = $1; n = length(ref)
            if (n >= 7 && substr(full, 1, n) == substr(ref, 1, n)) {
                print $2; exit
            }
        }
    ' "$TRAILERS"
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

    hit=$(lookup_subject "$subject")
    if [ -n "$hit" ]; then
        printf 'PRESENT %s subject %s %s\n' "$cand" "$hit" "$subject"
        continue
    fi

    build_patchids
    cand_pid=$(git_fork show "$full" 2>/dev/null \
        | git_fork patch-id --stable 2>/dev/null \
        | cut -d' ' -f1)
    if [ -n "$cand_pid" ]; then
        hit=$(awk -v pid="$cand_pid" '$1 == pid { print $2; exit }' "$PATCHIDS")
        if [ -n "$hit" ]; then
            printf 'PRESENT %s patch-id %s %s\n' "$cand" "$hit" "$subject"
            continue
        fi
    fi

    printf 'MISSING %s - - %s\n' "$cand" "$subject"
done
