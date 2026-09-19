#!/usr/bin/env bash
# Create, reconcile, publish, or verify the coordinator-owned GitHub release.
# No existing public release is adopted unless it carries the exact
# coordinator ownership marker and already matches the frozen inventory.

set -euo pipefail

fail() {
  echo "::error::github-publish-release: $1" >&2
  exit 1
}

for var in GH_TOKEN GH_REPOSITORY RELEASE_TAG APPROVAL ASSETS_DIR MANIFEST_FILE MANIFEST_SIGNATURE_FILE MODE; do
  [[ -n "${!var:-}" ]] || fail "required variable $var is not set"
done
[[ "$MODE" =~ ^(create|reconcile|publish|verify)$ ]] || fail "invalid MODE: $MODE"
[[ -d "$ASSETS_DIR" && ! -L "$ASSETS_DIR" ]] || fail "assets directory is missing or unsafe"
[[ -f "$MANIFEST_FILE" && -f "$MANIFEST_SIGNATURE_FILE" ]] || fail "manifest/signature missing"

owner_marker="<!-- terraphim-release-coordinator manifest-sha256=${APPROVAL} -->"
scratch_dir="$(mktemp -d "${RUNNER_TEMP:-/tmp}/github-release.XXXXXX")"
trap 'find "$scratch_dir" -type f -delete 2>/dev/null || true; rmdir "$scratch_dir" 2>/dev/null || true' EXIT

release_json="$scratch_dir/release.json"
release_error="$scratch_dir/release.stderr"

lookup_release() {
  if gh api "repos/${GH_REPOSITORY}/releases/tags/${RELEASE_TAG}" >"$release_json" 2>"$release_error"; then
    return 0
  fi
  if grep -Eqi '(HTTP[^0-9]*404|Not Found|status code: 404)' "$release_error"; then
    return 44
  fi
  fail "release lookup failed and was not explicit absence: $(tr '\n' ' ' < "$release_error")"
}

require_owned_release() {
  local body tag
  tag="$(jq -r '.tag_name // empty' "$release_json")"
  body="$(jq -r '.body // empty' "$release_json")"
  [[ "$tag" == "$RELEASE_TAG" ]] || fail "release tag mismatch: $tag"
  # GitHub/UI round-trips may normalize release-body newlines to CRLF. Remove
  # carriage returns, then retain exact whole-line matching so near/colliding
  # markers cannot claim coordinator ownership.
  body="${body//$'\r'/}"
  grep -Fqx -- "$owner_marker" <<<"$body" \
    || fail "existing release is not owned by this approved coordinator manifest"
}

write_evidence() {
  if [[ -n "${RELEASE_EVIDENCE_FILE:-}" ]]; then
    jq '{id, html_url, tag_name, draft}' "$release_json" >"$RELEASE_EVIDENCE_FILE"
  fi
}

if [[ "$MODE" == "create" ]]; then
  set +e
  lookup_release
  lookup_rc=$?
  set -e
  if [[ "$lookup_rc" -eq 44 ]]; then
    notes_file="$scratch_dir/notes.md"
    printf '%s\n\nCoordinated release %s. The signed manifest is the sole machine-readable BOM.\n' \
      "$owner_marker" "$RELEASE_TAG" >"$notes_file"
    gh release create "$RELEASE_TAG" --repo "$GH_REPOSITORY" --draft \
      --title "$RELEASE_TAG" --notes-file "$notes_file"
    lookup_release || fail "created release could not be read back"
  fi
  require_owned_release
  # A published owned release is accepted only for crash recovery; later
  # reconcile/verify modes still require the exact approved inventory.
  write_evidence
  exit 0
fi

lookup_release || fail "coordinator-owned release does not exist"
require_owned_release

expected_tsv="$scratch_dir/expected.tsv"
python3 - "$MANIFEST_FILE" "$MANIFEST_SIGNATURE_FILE" "$ASSETS_DIR" >"$expected_tsv" <<'PY'
import hashlib, json, pathlib, sys
manifest_path = pathlib.Path(sys.argv[1])
signature_path = pathlib.Path(sys.argv[2])
assets_dir = pathlib.Path(sys.argv[3])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
expected = {}
for asset in manifest["assets"]:
    name = asset["name"]
    path = assets_dir / name
    if not path.is_file() or path.is_symlink():
        raise SystemExit(f"missing regular approved asset: {name}")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != asset["sha256"]:
        raise SystemExit(f"approved asset digest drift: {name}")
    expected[name] = (path, digest)
for name, path in (("manifest.json", manifest_path), ("manifest.json.sig", signature_path)):
    expected[name] = (path, hashlib.sha256(path.read_bytes()).hexdigest())
for name in sorted(expected):
    path, digest = expected[name]
    print(f"{name}\t{digest}\t{path}")
PY

remote_names="$scratch_dir/remote-names"
jq -r '.assets[].name' "$release_json" | sort >"$remote_names"
cut -f1 "$expected_tsv" | sort >"$scratch_dir/expected-names"
unexpected="$(comm -13 "$scratch_dir/expected-names" "$remote_names")"
[[ -z "$unexpected" ]] || fail "release contains assets outside the approved manifest: $unexpected"

verify_asset() {
  local name="$1" expected_sha="$2" destination="$scratch_dir/download/$name"
  mkdir -p "$scratch_dir/download"
  gh release download "$RELEASE_TAG" --repo "$GH_REPOSITORY" \
    --pattern "$name" --dir "$scratch_dir/download" --clobber \
    || fail "transport failure downloading existing asset $name"
  local actual_sha
  actual_sha="$(sha256sum -- "$destination" | awk '{print $1}')"
  [[ "$actual_sha" == "$expected_sha" ]] \
    || fail "immutable release asset $name has digest $actual_sha, expected $expected_sha"
}

while IFS=$'\t' read -r name expected_sha path; do
  if grep -Fqx -- "$name" "$remote_names"; then
    verify_asset "$name" "$expected_sha"
  elif [[ "$MODE" == "reconcile" && "$(jq -r '.draft' "$release_json")" == "true" ]]; then
    gh release upload "$RELEASE_TAG" --repo "$GH_REPOSITORY" -- "$path" \
      || fail "upload failed for $name"
  else
    fail "approved release asset is missing: $name"
  fi
done <"$expected_tsv"

if [[ "$MODE" == "reconcile" ]]; then
  lookup_release || fail "release disappeared after reconciliation"
  jq -r '.assets[].name' "$release_json" | sort >"$remote_names"
  cmp -s "$scratch_dir/expected-names" "$remote_names" \
    || fail "release inventory is not exactly the approved inventory after reconciliation"
  while IFS=$'\t' read -r name expected_sha _path; do
    verify_asset "$name" "$expected_sha"
  done <"$expected_tsv"
elif [[ "$MODE" == "publish" ]]; then
  if [[ "$(jq -r '.draft' "$release_json")" == "true" ]]; then
    gh release edit "$RELEASE_TAG" --repo "$GH_REPOSITORY" --draft=false
    lookup_release || fail "published release could not be read back"
    [[ "$(jq -r '.draft' "$release_json")" == "false" ]] \
      || fail "release remained draft after publication"
  else
    echo "Release is already public with the exact approved inventory; idempotent crash recovery."
  fi
elif [[ "$MODE" == "verify" ]]; then
  [[ "$(jq -r '.draft' "$release_json")" == "false" ]] \
    || fail "release is still draft"
  # The loop above downloaded and digest-checked every expected object and
  # the exact-name check rejected every unapproved object.
fi

write_evidence
echo "GitHub release $MODE succeeded for $RELEASE_TAG (manifest $APPROVAL)"
