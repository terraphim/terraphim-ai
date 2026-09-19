#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
SCRIPT="$ROOT/.github/scripts/release/github-publish-release.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-github-release.XXXXXX")"
trap 'find "$TMP" -type f -delete 2>/dev/null || true; find "$TMP" -depth -type d -empty -delete 2>/dev/null || true' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }

ASSETS="$TMP/assets"
STORE="$TMP/store"
mkdir -p "$ASSETS" "$STORE/assets"
printf 'server' >"$ASSETS/server.tar.gz"
SERVER_SHA="$(sha256sum "$ASSETS/server.tar.gz" | awk '{print $1}')"
MANIFEST="$TMP/manifest.json"
SIGNATURE="$TMP/manifest.json.sig"
printf '{"assets":[{"name":"server.tar.gz","sha256":"%s"}]}\n' "$SERVER_SHA" >"$MANIFEST"
printf 'signature' >"$SIGNATURE"
APPROVAL="$(sha256sum "$MANIFEST" | awk '{print $1}')"
OWNER_MARKER="<!-- terraphim-release-coordinator manifest-sha256=${APPROVAL} -->"

STUB_GH="$TMP/gh"
cat >"$STUB_GH" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
store="$GH_STUB_STORE"
if [[ "${GH_STUB_TRANSPORT_FAILURE:-}" == "1" ]]; then echo "network timeout" >&2; exit 2; fi
if [[ "$1" == api ]]; then
  [[ -f "$store/release.json" ]] || { echo "HTTP 404: Not Found" >&2; exit 1; }
  api_count_file="$store/.api-count"
  api_count=0
  [[ ! -f "$api_count_file" ]] || api_count="$(cat "$api_count_file")"
  api_count=$((api_count + 1))
  printf '%s' "$api_count" >"$api_count_file"
  if [[ "${GH_STUB_INJECT_EXTRA_ON_API:-0}" == "$api_count" ]]; then
    printf 'race-added' >"$store/assets/race-added.bin"
  fi
  python3 - "$store" <<'PY'
import json, pathlib, sys
store = pathlib.Path(sys.argv[1])
r = json.loads((store / "release.json").read_text())
r["assets"] = [{"name": p.name} for p in sorted((store / "assets").iterdir()) if p.is_file()]
print(json.dumps(r))
PY
  exit 0
fi
[[ "$1" == release ]] || exit 2
sub="$2"; shift 2
case "$sub" in
  create)
    tag="$1"; shift
    notes=""
    while [[ $# -gt 0 ]]; do case "$1" in --notes-file) notes="$2"; shift 2;; *) shift;; esac; done
    python3 - "$store/release.json" "$tag" "$notes" <<'PY'
import json, pathlib, sys
json.dump({"id": 7, "html_url": "https://example.invalid/r", "tag_name": sys.argv[2], "draft": True,
           "body": pathlib.Path(sys.argv[3]).read_text()}, open(sys.argv[1], "w"))
PY
    ;;
  upload)
    tag="$1"; shift
    repo=""
    paths=()
    while [[ $# -gt 0 ]]; do
      case "$1" in --repo) repo="$2"; shift 2;; --) shift;; *) paths+=("$1"); shift;; esac
    done
    for path in "${paths[@]}"; do cp "$path" "$store/assets/$(basename "$path")"; done
    ;;
  download)
    shift # tag
    pattern=""; dir=""
    while [[ $# -gt 0 ]]; do case "$1" in --pattern) pattern="$2"; shift 2;; --dir) dir="$2"; shift 2;; --repo) shift 2;; *) shift;; esac; done
    mkdir -p "$dir"; cp "$store/assets/$pattern" "$dir/$pattern"
    ;;
  edit)
    shift # tag
    python3 - "$store/release.json" <<'PY'
import json, sys
p=sys.argv[1]; d=json.load(open(p)); d["draft"]=False; json.dump(d, open(p,"w"))
PY
    ;;
  *) exit 2;;
esac
STUB
chmod +x "$STUB_GH"

run_mode() {
  local mode="$1"
  shift
  env PATH="$TMP:$PATH" GH_STUB_STORE="$STORE" GH_TOKEN=token GH_REPOSITORY=org/repo \
    RELEASE_TAG=v1.2.3 APPROVAL="$APPROVAL" ASSETS_DIR="$ASSETS" \
    MANIFEST_FILE="$MANIFEST" MANIFEST_SIGNATURE_FILE="$SIGNATURE" MODE="$mode" \
    "$@" "$SCRIPT"
}

run_mode create >/dev/null || fail "draft creation failed"
[[ "$(python3 -c "import json; print(json.load(open('$STORE/release.json'))['draft'])")" == True ]] || fail "release not draft"
run_mode reconcile >/dev/null || fail "reconcile failed"
[[ "$(find "$STORE/assets" -maxdepth 1 -type f | wc -l)" -eq 3 ]] || fail "exact inventory not uploaded"
run_mode publish >/dev/null || fail "publish failed"
run_mode verify >/dev/null || fail "verification failed"

# Extra release assets are outside approval and must fail closed.
printf 'rogue' >"$STORE/assets/rogue.bin"
if run_mode verify >"$TMP/out" 2>"$TMP/err"; then fail "unapproved release asset accepted"; fi
grep -Fq "outside the approved manifest" "$TMP/err" || fail "missing extra-asset error"
find "$STORE/assets" -maxdepth 1 -type f -name rogue.bin -delete

# Every published expected asset is downloaded and digest-checked.
printf 'tampered' >"$STORE/assets/server.tar.gz"
if run_mode verify >"$TMP/out" 2>"$TMP/err"; then fail "tampered approved asset accepted"; fi
grep -Fq "immutable release asset server.tar.gz has digest" "$TMP/err" || fail "missing digest mismatch"
cp "$ASSETS/server.tar.gz" "$STORE/assets/server.tar.gz"

# Reconciliation performs a second exact-set check, closing a race where an
# unapproved asset appears after the initial lookup.
python3 - "$STORE/release.json" <<'PY'
import json, sys
p=sys.argv[1]; d=json.load(open(p)); d["draft"]=True; json.dump(d, open(p,"w"))
PY
printf '0' >"$STORE/.api-count"
if run_mode reconcile GH_STUB_INJECT_EXTRA_ON_API=2 >"$TMP/out" 2>"$TMP/err"; then
  fail "post-reconcile extra asset accepted"
fi
grep -Fq "not exactly the approved inventory after reconciliation" "$TMP/err" || fail "missing post-reconcile exact-set failure"
find "$STORE/assets" -maxdepth 1 -type f -name race-added.bin -delete

# Terminal verification requires a public release, not a reconciled draft.
if run_mode verify >"$TMP/out" 2>"$TMP/err"; then fail "draft release passed terminal verification"; fi
grep -Fq "release is still draft" "$TMP/err" || fail "missing draft-state failure"

# Ownership survives LF/CRLF normalization but remains an exact whole-line
# token; colliding prefixes/suffixes cannot claim a release.
python3 - "$STORE/release.json" <<'PY'
import json, sys
p=sys.argv[1]; d=json.load(open(p)); d["body"]=d["body"].replace("\n", "\r\n"); json.dump(d, open(p,"w"))
PY
run_mode create >"$TMP/out" 2>"$TMP/err" || fail "CRLF-normalized owner marker was rejected: $(cat "$TMP/err")"
python3 - "$STORE/release.json" "$OWNER_MARKER" <<'PY'
import json, sys
p=sys.argv[1]; d=json.load(open(p)); d["body"]=sys.argv[2] + "-collision"; json.dump(d, open(p,"w"))
PY
if run_mode create >"$TMP/out" 2>"$TMP/err"; then fail "near-colliding owner marker was accepted"; fi
grep -Fq "not owned" "$TMP/err" || fail "missing near-collision ownership failure"

# A same-tag release without the approval-bound ownership marker is never
# adopted, even if its current inventory happens to look compatible.
python3 - "$STORE/release.json" <<'PY'
import json, sys
p=sys.argv[1]; d=json.load(open(p)); d["body"]="foreign release"; json.dump(d, open(p,"w"))
PY
if run_mode create >"$TMP/out" 2>"$TMP/err"; then fail "foreign release was adopted"; fi
grep -Fq "not owned" "$TMP/err" || fail "missing ownership failure"

# Transport failure is never interpreted as release absence/create permission.
if env GH_STUB_TRANSPORT_FAILURE=1 PATH="$TMP:$PATH" GH_STUB_STORE="$STORE" \
  GH_TOKEN=token GH_REPOSITORY=org/repo RELEASE_TAG=v1.2.3 APPROVAL="$APPROVAL" \
  ASSETS_DIR="$ASSETS" MANIFEST_FILE="$MANIFEST" MANIFEST_SIGNATURE_FILE="$SIGNATURE" \
  MODE=create "$SCRIPT" >"$TMP/out" 2>"$TMP/err"; then fail "transport failure accepted"; fi
grep -Fq "not explicit absence" "$TMP/err" || fail "transport failure treated as absence"

echo "github-publish-release tests passed (9 cases)"
