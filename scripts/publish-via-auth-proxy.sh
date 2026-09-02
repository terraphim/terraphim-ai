#!/usr/bin/env bash
# publish-via-auth-proxy.sh -- publish a workspace crate to the internal
# `terraphim` Gitea cargo registry through a local auth-injecting proxy.
#
# Why this exists: cargo's builtin token provider sends the registry token
# in the bare legacy `Authorization: <token>` header, which Gitea rejects
# (verified against Gitea 1.26: Bearer / token-prefixed / Basic headers all
# authenticate, the bare header 401s). The proxy keeps cargo's own packaging
# and wire protocol -- only the Authorization header is rewritten to Basic.
#
# Usage:
#   scripts/publish-via-auth-proxy.sh <crate-name> [--dry-run]
#
# Environment:
#   GITEA_TOKEN   Gitea access token with package write for `terraphim`
#
# The proxy serves the registry on 127.0.0.1:8899 and is started on demand;
# stop it with:  pkill -f cargo_registry_proxy.py

set -euo pipefail

CRATE="${1:?usage: publish-via-auth-proxy.sh <crate-name> [--dry-run]}"
DRY_RUN="${2:-}"
: "${GITEA_TOKEN:?GITEA_TOKEN must be set (Gitea access token with package write)}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROXY_SCRIPT="${SCRIPT_DIR}/cargo_registry_proxy.py"
PROXY_PORT=8899
PROXY_REGISTRY="tproxy"

if ! curl -s -o /dev/null "http://127.0.0.1:${PROXY_PORT}/config.json"; then
  echo "starting auth proxy on 127.0.0.1:${PROXY_PORT}..."
  GITEA_TOKEN="$GITEA_TOKEN" nohup python3 "$PROXY_SCRIPT" >/tmp/cargo_registry_proxy.log 2>&1 &
  sleep 1
fi

# One-time registry alias (idempotent).
if ! grep -q "registries.${PROXY_REGISTRY}" ~/.cargo/config.toml 2>/dev/null; then
  cat >> ~/.cargo/config.toml <<EOF

[registries.${PROXY_REGISTRY}]
index = "sparse+http://127.0.0.1:${PROXY_PORT}/api/packages/terraphim/cargo/"
credential-provider = "cargo:token"
EOF
  echo "added [registries.${PROXY_REGISTRY}] to ~/.cargo/config.toml"
fi

extra=()
if [ "$DRY_RUN" = "--dry-run" ]; then
  extra=(--dry-run)
fi

env "CARGO_REGISTRIES_${PROXY_REGISTRY^^}_TOKEN=$GITEA_TOKEN" \
  cargo publish --registry "$PROXY_REGISTRY" -p "$CRATE" "${extra[@]}"
