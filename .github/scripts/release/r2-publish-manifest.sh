#!/usr/bin/env bash
# Publish or verify the approval-bound, detached-signed stable manifest in R2.
# Existing per-tag objects are immutable: identical bytes are an idempotent
# retry; different bytes fail closed.  Only an explicit NoSuchKey/404 is
# absence -- authentication, DNS, timeout, and other transport failures never
# authorize a create.  The signature is landed and verified before
# manifest.json, which is the stable commit point consumers observe.
#
# In addition to the immutable per-tag objects, publish mode advances the
# SIGNED STABLE DISCOVERY POINTER per component at <component>/manifest.json
# (+ .sig) -- the exact bytes the terraphim_update crate fetches and verifies
# (detached zipsign/Ed25519, context terraphim-release-manifest-v1). Pointer
# keys are the ONLY mutable objects in this contract: they are overwritten by
# each new release, but carry the same signed bytes as the immutable per-tag
# object, so one approval-bound signature authenticates both. Pointer
# components are derived from the canonical manifest itself and must match
# the component key grammar; anything else fails closed before any put.

set -euo pipefail

AWS_BIN="${AWS_BIN:-aws}"
ZIPSIGN_BIN="${ZIPSIGN_BIN:-zipsign}"
MODE="${MODE:-publish}"

fail() {
  echo "::error::r2-publish-manifest: $1" >&2
  exit 1
}

for var in R2_ACCESS_KEY_ID R2_SECRET_ACCESS_KEY R2_ENDPOINT_URL R2_BUCKET MANIFEST_FILE MANIFEST_SIGNATURE_FILE VERIFYING_KEY_FILE STATE_FILE RELEASE_TAG EXPECTED_SHA256; do
  [[ -n "${!var:-}" ]] || fail "required variable $var is not set (fail closed, no R2 mutation attempted)"
done
[[ "$MODE" == "publish" || "$MODE" == "verify" ]] || fail "MODE must be publish or verify"
for file in "$MANIFEST_FILE" "$MANIFEST_SIGNATURE_FILE" "$VERIFYING_KEY_FILE" "$STATE_FILE"; do
  [[ -f "$file" && ! -L "$file" ]] || fail "required regular file not found: $file"
done
command -v "$AWS_BIN" >/dev/null 2>&1 || fail "aws CLI ('$AWS_BIN') not available (fail closed, no R2 mutation attempted)"
command -v "$ZIPSIGN_BIN" >/dev/null 2>&1 || fail "zipsign ('$ZIPSIGN_BIN') not available (fail closed, no R2 mutation attempted)"

# The network script independently enforces the coordinator invariant instead
# of trusting a workflow `if:` expression.  R2 may only be touched after a
# bound approval, promote-begin, and terminal GitHub-release verification.
python3 - "$STATE_FILE" "$EXPECTED_SHA256" <<'PY' || fail "coordinator state does not authorize R2 publication"
import json, sys
with open(sys.argv[1], encoding="utf-8") as state_file:
    state = json.load(state_file)
expected = sys.argv[2]
approval = state.get("approval") or {}
promotion = state.get("phases", {}).get("promotion", {})
github = state.get("central_channels", {}).get("github_release", {})

checks = (
    (state.get("status") in {"promoting", "promoted"}, "status is not promoting/promoted"),
    (state.get("manifest_sha256") == expected, "state manifest digest does not match approval input"),
    (approval.get("manifest_sha256") == expected, "recorded approval is not bound to the manifest digest"),
    (promotion.get("status") in {"in_progress", "complete"}, "promotion has not begun"),
    (github.get("publication", {}).get("outcome") == "success", "GitHub publication is not successful"),
    (github.get("verification", {}).get("outcome") == "success", "GitHub terminal verification is not successful"),
)
for allowed, reason in checks:
    if not allowed:
        raise SystemExit(reason)
PY

actual_manifest_sha256="$(sha256sum -- "$MANIFEST_FILE" | awk '{print $1}')"
[[ "$actual_manifest_sha256" == "$EXPECTED_SHA256" ]] || fail "local manifest digest mismatch: expected $EXPECTED_SHA256, got $actual_manifest_sha256"
"$ZIPSIGN_BIN" verify separate --context terraphim-release-manifest-v1 --quiet \
  "$MANIFEST_FILE" "$MANIFEST_SIGNATURE_FILE" "$VERIFYING_KEY_FILE" \
  || fail "local detached manifest signature is invalid"

# Conditional creation is the immutability boundary. Fail before any object
# lookup or put when the installed AWS CLI cannot express If-None-Match.
if ! LC_ALL=C "$AWS_BIN" s3api put-object help 2>&1 | grep -Fq -- '--if-none-match'; then
  fail "aws CLI does not support s3api put-object --if-none-match (aws-cli >= 2.17 required)"
fi

OBJECT_KEY="${R2_OBJECT_KEY:-releases/${RELEASE_TAG}/manifest.json}"
SIGNATURE_OBJECT_KEY="${R2_SIGNATURE_OBJECT_KEY:-${OBJECT_KEY}.sig}"
export AWS_ACCESS_KEY_ID="$R2_ACCESS_KEY_ID"
export AWS_SECRET_ACCESS_KEY="$R2_SECRET_ACCESS_KEY"
export AWS_DEFAULT_REGION=auto

scratch_dir="$(mktemp -d "${RUNNER_TEMP:-/tmp}/r2-manifest.XXXXXX")"
trap 'find "$scratch_dir" -type f -delete 2>/dev/null || true; rmdir "$scratch_dir" 2>/dev/null || true' EXIT

get_object() {
  local key="$1" output="$2" error_file="$3"
  "$AWS_BIN" s3api get-object \
    --endpoint-url "$R2_ENDPOINT_URL" --bucket "$R2_BUCKET" --key "$key" \
    "$output" >"$scratch_dir/get.stdout" 2>"$error_file"
}

verify_remote_object() {
  local key="$1" expected_file="$2" output="$3" error_file="$4"
  get_object "$key" "$output" "$error_file" \
    || fail "R2 readback failed for $key (not treated as absence): $(tr '\n' ' ' < "$error_file")"
  cmp -s -- "$expected_file" "$output" \
    || fail "R2 object $key differs from the approved immutable bytes"
}

immutable_put() {
  local key="$1" body="$2" content_type="$3"
  local existing="$scratch_dir/existing" error_file="$scratch_dir/get.stderr"
  if get_object "$key" "$existing" "$error_file"; then
    cmp -s -- "$body" "$existing" \
      || fail "immutable R2 object $key already exists with different bytes"
    echo "R2 object already exists with identical bytes; idempotent retry: $key"
    return 0
  fi
  if ! grep -Eqi '(NoSuchKey|Not Found|HTTP[^0-9]*404|status code: 404)' "$error_file"; then
    fail "R2 lookup failed for $key and was not explicit absence: $(tr '\n' ' ' < "$error_file")"
  fi
  "$AWS_BIN" s3api put-object \
    --endpoint-url "$R2_ENDPOINT_URL" --bucket "$R2_BUCKET" --key "$key" \
    --body "$body" --content-type "$content_type" --if-none-match '*' >/dev/null \
    || fail "conditional R2 put-object failed for $key"
}

if [[ "$MODE" == "publish" ]]; then
  # Signed stable discovery pointers: the terraphim_update crate consumes
  # <component>/manifest.json (+ .sig). The pointer carries the SAME signed
  # bytes as the immutable per-tag object, so the approval-bound detached
  # signature authenticates both. Pointer keys are derived from the canonical
  # manifest's component set (never a hard-coded list) and must match the
  # component key grammar. Validation happens BEFORE any put so a malformed
  # manifest cannot leave a half-published release behind.
  if ! components_raw="$(python3 - "$MANIFEST_FILE" <<'PY'
import json, re, sys
with open(sys.argv[1], encoding="utf-8") as manifest_file:
    manifest = json.load(manifest_file)
grammar = re.compile(r"^[a-z0-9][a-z0-9-]*$")
components = sorted({asset.get("component", "") for asset in manifest.get("assets", [])})
if not components or any(not grammar.fullmatch(component) for component in components):
    raise SystemExit(f"asset components outside the pointer key grammar: {components!r}")
for component in components:
    print(component)
PY
)"; then
    fail "could not derive pointer components from the canonical manifest (fail closed, no R2 mutation attempted)"
  fi
  mapfile -t POINTER_COMPONENTS <<<"$components_raw"

  immutable_put "$SIGNATURE_OBJECT_KEY" "$MANIFEST_SIGNATURE_FILE" application/octet-stream
  verify_remote_object "$SIGNATURE_OBJECT_KEY" "$MANIFEST_SIGNATURE_FILE" \
    "$scratch_dir/signature.readback" "$scratch_dir/signature.stderr"
  # Commit point: manifest.json is written only after its detached signature
  # is durable and verified, and only after the GitHub release was verified.
  immutable_put "$OBJECT_KEY" "$MANIFEST_FILE" application/json

  publish_signed_pointer() {
    local component="$1"
    local pointer_key="${component}/manifest.json"
    local pointer_sig_key="${component}/manifest.json.sig"
    local pointer_readback="$scratch_dir/pointer.${component}.readback"
    local pointer_sig_readback="$scratch_dir/pointer.${component}.sig.readback"
    # Signed and mutable: the pointer advances with each release. The bytes
    # are the approval-bound signed manifest, and the readback pair is
    # cryptographically verified below, so overwrite cannot inject content.
    "$AWS_BIN" s3api put-object \
      --endpoint-url "$R2_ENDPOINT_URL" --bucket "$R2_BUCKET" --key "$pointer_sig_key" \
      --body "$MANIFEST_SIGNATURE_FILE" --content-type application/octet-stream >/dev/null \
      || fail "pointer signature put-object failed for $pointer_sig_key"
    "$AWS_BIN" s3api put-object \
      --endpoint-url "$R2_ENDPOINT_URL" --bucket "$R2_BUCKET" --key "$pointer_key" \
      --body "$MANIFEST_FILE" --content-type application/json >/dev/null \
      || fail "pointer manifest put-object failed for $pointer_key"
    verify_remote_object "$pointer_key" "$MANIFEST_FILE" \
      "$pointer_readback" "$scratch_dir/pointer.${component}.stderr"
    verify_remote_object "$pointer_sig_key" "$MANIFEST_SIGNATURE_FILE" \
      "$pointer_sig_readback" "$scratch_dir/pointer.${component}.sig.stderr"
    "$ZIPSIGN_BIN" verify separate --context terraphim-release-manifest-v1 --quiet \
      "$pointer_readback" "$pointer_sig_readback" "$VERIFYING_KEY_FILE" \
      || fail "pointer readback detached signature verification failed for $component"
    echo "signed discovery pointer advanced: s3://$R2_BUCKET/$pointer_key"
  }

  for component in "${POINTER_COMPONENTS[@]}"; do
    publish_signed_pointer "$component"
  done
fi

verify_remote_object "$OBJECT_KEY" "$MANIFEST_FILE" \
  "$scratch_dir/manifest.readback" "$scratch_dir/manifest.stderr"
verify_remote_object "$SIGNATURE_OBJECT_KEY" "$MANIFEST_SIGNATURE_FILE" \
  "$scratch_dir/signature-final.readback" "$scratch_dir/signature-final.stderr"
"$ZIPSIGN_BIN" verify separate --context terraphim-release-manifest-v1 --quiet \
  "$scratch_dir/manifest.readback" "$scratch_dir/signature-final.readback" "$VERIFYING_KEY_FILE" \
  || fail "R2 readback detached signature verification failed"

echo "R2 stable manifest ${MODE} and terminal verification succeeded: s3://$R2_BUCKET/$OBJECT_KEY (sha256=$actual_manifest_sha256)"
