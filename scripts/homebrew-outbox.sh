#!/usr/bin/env bash
# Idempotent Homebrew downstream outbox (Gitea terraphim-ai#3381, MP5a).
#
# Renders the Homebrew formulas for a release manifest into a tap checkout and,
# only with --dispatch, commits them on a branch and opens/updates a pull
# request. The central publication state (the GitHub release record and the R2
# stable manifest) is never touched: this channel is eventually consistent and
# its failure blocks only itself.
#
# Without --dispatch the script is a pure dry run: it writes the formulas and
# reports whether they changed, but never commits, pushes or opens a PR.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GENERATOR="$ROOT/scripts/generate-homebrew-formulas.py"

MANIFEST=""
TAP_DIR=""
DISPATCH=0
BASE="main"
HEAD=""
REPO="terraphim/homebrew-terraphim"

usage() {
  cat >&2 <<'EOF'
Usage: homebrew-outbox.sh --manifest FILE --tap-dir DIR [--dispatch]
                          [--repo OWNER/NAME] [--base BRANCH] [--head BRANCH]
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest) MANIFEST="$2"; shift 2 ;;
    --tap-dir) TAP_DIR="$2"; shift 2 ;;
    --dispatch) DISPATCH=1; shift ;;
    --repo) REPO="$2"; shift 2 ;;
    --base) BASE="$2"; shift 2 ;;
    --head) HEAD="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done

if [[ -z "$MANIFEST" || -z "$TAP_DIR" ]]; then
  usage
  exit 2
fi
[[ -f "$MANIFEST" ]] || { echo "homebrew-outbox: manifest not found: $MANIFEST" >&2; exit 2; }
[[ -d "$TAP_DIR" ]] || { echo "homebrew-outbox: tap dir not found: $TAP_DIR" >&2; exit 2; }

TAG="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("release_tag",""))' "$MANIFEST")"
VERSION="$(printf '%s' "$TAG" | sed 's/^v//')"

python3 "$GENERATOR" --manifest "$MANIFEST" --output-dir "$TAP_DIR/Formula"

if [[ -z "$(git -C "$TAP_DIR" status --porcelain -- Formula)" ]]; then
  echo "homebrew-outbox: no-op (formulas already current for $TAG)"
  exit 0
fi
echo "homebrew-outbox: formulas changed for $TAG"
git -C "$TAP_DIR" status --short -- Formula

if [[ "$DISPATCH" != "1" ]]; then
  echo "homebrew-outbox: dry-run; pass --dispatch to commit and open a PR"
  exit 0
fi

if [[ -z "$HEAD" ]]; then
  HEAD="automation/homebrew-$VERSION"
fi

git -C "$TAP_DIR" checkout -B "$HEAD"
git -C "$TAP_DIR" add Formula
if git -C "$TAP_DIR" diff --cached --quiet; then
  echo "homebrew-outbox: nothing staged; no-op"
  exit 0
fi
git -C "$TAP_DIR" -c user.name="terraphim-release-bot" \
  -c user.email="release@terraphim.ai" \
  commit -m "chore: bump terraphim formulas to $TAG"
git -C "$TAP_DIR" push -u origin "$HEAD" --force-with-lease

EXISTING="$(gh pr list --repo "$REPO" --head "$HEAD" --state open --json url --jq '.[0].url // empty' 2>/dev/null || true)"
if [[ -n "$EXISTING" ]]; then
  echo "homebrew-outbox: PR already open: $EXISTING"
  exit 0
fi
gh pr create --repo "$REPO" --base "$BASE" --head "$HEAD" \
  --title "chore: bump terraphim formulas to $TAG" \
  --body "Generated from the canonical release manifest for $TAG (Gitea terraphim-ai#3381)."
