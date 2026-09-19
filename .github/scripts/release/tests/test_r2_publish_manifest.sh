#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
SCRIPT="$ROOT/.github/scripts/release/r2-publish-manifest.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-r2-publish.XXXXXX")"
trap 'find "$TMP" -type f -delete 2>/dev/null || true; find "$TMP" -depth -type d -empty -delete 2>/dev/null || true' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

MANIFEST="$TMP/manifest.json"
SIGNATURE="$TMP/manifest.json.sig"
PRIVATE_KEY="$TMP/private.key"
VERIFYING_KEY="$TMP/verifying.key"
STATE="$TMP/state.json"
printf '{"release_tag":"v1.2.3"}\n' >"$MANIFEST"
EXPECTED_SHA256="$(sha256sum -- "$MANIFEST" | awk '{print $1}')"
zipsign gen-key "$PRIVATE_KEY" "$VERIFYING_KEY" >/dev/null
zipsign sign separate --context terraphim-release-manifest-v1 --output "$SIGNATURE" --force "$MANIFEST" "$PRIVATE_KEY" >/dev/null

write_state() {
  local status="${1:-promoting}"
  local state_digest="${2:-$EXPECTED_SHA256}"
  local approval_digest="${3:-$EXPECTED_SHA256}"
  local promotion_status="${4:-in_progress}"
  local publication_outcome="${5:-success}"
  local verification_outcome="${6:-success}"
  python3 - "$STATE" "$status" "$state_digest" "$approval_digest" \
    "$promotion_status" "$publication_outcome" "$verification_outcome" <<'PY'
import json, sys
path, status, state_digest, approval_digest, promotion, publication, verification = sys.argv[1:]
json.dump({
  "status": status,
  "manifest_sha256": state_digest,
  "approval": {"manifest_sha256": approval_digest},
  "phases": {"promotion": {"status": promotion}},
  "central_channels": {"github_release": {
    "publication": {"outcome": publication},
    "verification": {"outcome": verification},
  }},
}, open(path, "w"))
PY
}
write_state

STORE="$TMP/store"
mkdir -p "$STORE"
STUB_AWS="$TMP/aws"
cat >"$STUB_AWS" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
store="$STUB_STORE_DIR"
shift # s3api
sub="$1"; shift
if [[ "$sub" == "put-object" && "${1:-}" == "help" ]]; then
  [[ "${STUB_NO_IF_NONE_MATCH_HELP:-}" != "1" ]] && printf '%s\n' '       --if-none-match (string)'
  exit 0
fi
[[ "${STUB_TRANSPORT_FAILURE:-}" != "1" ]] || { echo "connection timed out" >&2; exit 255; }
key=""; body=""; output=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --key) key="$2"; shift 2 ;;
    --body) body="$2"; shift 2 ;;
    --bucket|--endpoint-url|--content-type|--if-none-match) shift 2 ;;
    *) output="$1"; shift ;;
  esac
done
path="$store/$key"
case "$sub" in
  get-object)
    [[ -f "$path" ]] || { echo "An error occurred (NoSuchKey) (404)" >&2; exit 254; }
    corrupt_marker="$store/.corrupted-readback-once"
    if [[ -n "${STUB_CORRUPT_READBACK_ONCE_KEY:-}" && "$key" == "$STUB_CORRUPT_READBACK_ONCE_KEY" && ! -e "$corrupt_marker" ]]; then
      printf 'corrupted-readback' >"$output"
      : >"$corrupt_marker"
    else
      cp "$path" "$output"
    fi
    ;;
  put-object)
    [[ ! -e "$path" ]] || { echo "PreconditionFailed" >&2; exit 253; }
    mkdir -p "$(dirname "$path")"
    cp "$body" "$path"
    ;;
  *) echo "unsupported: $sub" >&2; exit 2 ;;
esac
STUB
chmod +x "$STUB_AWS"

run_script() {
  env R2_ACCESS_KEY_ID=key R2_SECRET_ACCESS_KEY=secret \
    R2_ENDPOINT_URL=https://example.invalid R2_BUCKET=bucket \
    MANIFEST_FILE="$MANIFEST" MANIFEST_SIGNATURE_FILE="$SIGNATURE" \
    VERIFYING_KEY_FILE="$VERIFYING_KEY" STATE_FILE="$STATE" \
    RELEASE_TAG=v1.2.3 EXPECTED_SHA256="$EXPECTED_SHA256" \
    AWS_BIN="$STUB_AWS" STUB_STORE_DIR="$STORE" "$@" "$SCRIPT"
}

assert_unauthorized() {
  local description="$1"
  shift
  find "$STORE" -mindepth 1 -delete
  write_state "$@"
  if run_script >"$TMP/out" 2>"$TMP/err"; then
    fail "$description unexpectedly published"
  fi
  [[ ! -e "$STORE/releases/v1.2.3/manifest.json" ]] || fail "$description created manifest object"
  [[ ! -e "$STORE/releases/v1.2.3/manifest.json.sig" ]] || fail "$description created signature object"
  grep -Fq "does not authorize" "$TMP/err" || fail "$description missing authorization failure"
}

# Every authorization conjunct independently fails closed before any object
# mutation. These cases are intentionally separate so deleting/inverting any
# one condition is mutation-detectable.
assert_unauthorized "wrong coordinator status" verified
assert_unauthorized "state manifest digest mismatch" promoting "$(printf '1%.0s' {1..64})"
assert_unauthorized "approval digest mismatch" promoting "$EXPECTED_SHA256" "$(printf '2%.0s' {1..64})"
assert_unauthorized "promotion still pending" promoting "$EXPECTED_SHA256" "$EXPECTED_SHA256" pending
assert_unauthorized "GitHub publication failure" promoting "$EXPECTED_SHA256" "$EXPECTED_SHA256" in_progress failure
assert_unauthorized "GitHub verification failure" promoting "$EXPECTED_SHA256" "$EXPECTED_SHA256" in_progress success failure

# Security gates must survive Python optimization mode.
find "$STORE" -mindepth 1 -delete
write_state verified
if run_script PYTHONOPTIMIZE=1 >"$TMP/out" 2>"$TMP/err"; then
  fail "PYTHONOPTIMIZE=1 bypassed authorization"
fi
[[ ! -e "$STORE/releases/v1.2.3/manifest.json" ]] || fail "optimize-mode bypass mutated R2"

# Authorized publish writes signature first and stable manifest last, then
# terminally verifies both. Identical retry is a no-op success.
write_state promoting
run_script >"$TMP/out" 2>"$TMP/err" || fail "authorized publish failed: $(cat "$TMP/err")"
cmp -s "$MANIFEST" "$STORE/releases/v1.2.3/manifest.json" || fail "manifest bytes differ"
cmp -s "$SIGNATURE" "$STORE/releases/v1.2.3/manifest.json.sig" || fail "signature bytes differ"
run_script >"$TMP/retry.out" 2>"$TMP/retry.err" || fail "identical retry failed"
grep -Fq "idempotent retry" "$TMP/retry.out" || fail "retry was not recognized"

# Immutable objects reject different bytes.
printf 'different' >"$STORE/releases/v1.2.3/manifest.json"
if run_script >"$TMP/out" 2>"$TMP/err"; then
  fail "different existing bytes were accepted"
fi
grep -Fq "different bytes" "$TMP/err" || fail "missing immutable-object failure"
cp "$MANIFEST" "$STORE/releases/v1.2.3/manifest.json"

# A transport failure is not interpreted as object absence and never causes a put.
if run_script STUB_TRANSPORT_FAILURE=1 >"$TMP/out" 2>"$TMP/err"; then
  fail "transport failure unexpectedly succeeded"
fi
grep -Fq "not explicit absence" "$TMP/err" || fail "transport failure was not distinguished from absence"

# A corrupted first readback must stop before the stable manifest commit.
# The stub corrupts only the first signature readback so a mutation deleting
# cmp cannot be masked by the final detached-signature verification.
find "$STORE" -mindepth 1 -delete
write_state promoting
if run_script STUB_CORRUPT_READBACK_ONCE_KEY=releases/v1.2.3/manifest.json.sig >"$TMP/out" 2>"$TMP/err"; then
  fail "corrupted signature readback unexpectedly succeeded"
fi
grep -Fq "differs from the approved immutable bytes" "$TMP/err" || fail "missing corrupted-readback failure"
[[ ! -e "$STORE/releases/v1.2.3/manifest.json" ]] || fail "stable manifest committed after corrupted signature readback"

# An AWS CLI too old to express conditional creation fails before lookup/put.
find "$STORE" -mindepth 1 -delete
if run_script STUB_NO_IF_NONE_MATCH_HELP=1 >"$TMP/out" 2>"$TMP/err"; then
  fail "AWS CLI without --if-none-match support was accepted"
fi
grep -Fq "does not support s3api put-object --if-none-match" "$TMP/err" || fail "missing AWS CLI capability failure"
[[ -z "$(find "$STORE" -mindepth 1 -print -quit)" ]] || fail "AWS preflight failure mutated store"

# Verify mode performs no put and still checks the detached signature.
run_script >"$TMP/out" 2>"$TMP/err" || fail "authorized republish after negative cases failed: $(cat "$TMP/err")"
run_script MODE=verify >"$TMP/out" 2>"$TMP/err" || fail "verify mode failed: $(cat "$TMP/err")"
grep -Fq "terminal verification succeeded" "$TMP/out" || fail "missing terminal verification message"

echo "r2-publish-manifest tests passed (13 cases)"
