#!/usr/bin/env bash
set -euo pipefail

root="${1:-crates/terraphim_tinyclaw/tests}"
missing=0

while IFS= read -r -d '' file; do
  case "$file" in
    */common/mod.rs) continue ;;
  esac

  if ! grep -Eq '^[[:space:]]*mod[[:space:]]+common[[:space:]]*;' "$file"; then
    echo "missing 'mod common;' in $file" >&2
    missing=1
  fi

  if ! grep -Eq 'common::scrub_env[[:space:]]*\(' "$file"; then
    echo "missing common::scrub_env() call in $file" >&2
    missing=1
  fi
done < <(find "$root" -type f -name '*.rs' -print0 | sort -z)

if [ "$missing" -ne 0 ]; then
  echo "TinyClaw integration tests must scrub credentials/environment." >&2
  exit 1
fi
