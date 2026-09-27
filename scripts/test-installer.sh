#!/bin/bash
# Installer release-gate test.
#
# Exercises the installer against the live public release channel and proves
# that a manipulated release cannot be installed. This test downloads real
# archives and, where it can, runs the real binary; it never substitutes a
# fixture for the channel.
#
# Usage: bash scripts/test-installer.sh [--keep]
#
# Exit codes: 0 all checks passed, 1 one or more checks failed.

set -uo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly INSTALLER="$SCRIPT_DIR/install.sh"
readonly RESOLVER="$SCRIPT_DIR/binary-resolution.sh"
readonly WORK="$(mktemp -d)"

KEEP=0
[[ "${1:-}" == "--keep" ]] && KEEP=1

PASS=0
FAIL=0

cleanup() {
    [[ $KEEP -eq 1 ]] && { echo "work dir kept: $WORK"; return; }
    rm -rf "$WORK"
}
trap cleanup EXIT

blue()  { echo -e "\033[0;34m$*\033[0m"; }
green() { echo -e "\033[0;32m$*\033[0m"; }
red()   { echo -e "\033[0;31m$*\033[0m"; }

check() {
    local label=$1 expected=$2 actual=$3
    if [[ "$expected" == "$actual" ]]; then
        green "  PASS  $label"
        PASS=$((PASS + 1))
    else
        red   "  FAIL  $label (expected '$expected', got '$actual')"
        FAIL=$((FAIL + 1))
    fi
}

blue "Installer release-gate test"
echo "  installer: $INSTALLER"
echo "  work dir:  $WORK"
echo

# --------------------------------------------------------------------------
# 1. Static checks
# --------------------------------------------------------------------------
blue "1. Static checks"

bash -n "$INSTALLER" 2>/dev/null
check "install.sh parses" "0" "$?"
bash -n "$RESOLVER" 2>/dev/null
check "binary-resolution.sh parses" "0" "$?"

# The installer must not resolve releases from the version-less GitHub assets.
if grep -q 'terraphim-ai/releases/download' "$INSTALLER" "$RESOLVER"; then
    check "no GitHub Releases download path" "absent" "present"
else
    check "no GitHub Releases download path" "absent" "absent"
fi

# --------------------------------------------------------------------------
# 2. Manifest resolution
# --------------------------------------------------------------------------
blue "2. Manifest resolution"

resolution=$("$RESOLVER" terraphim-agent latest 2>/dev/null)
check "resolver reports a channel version" "1.21.16" \
    "$(echo "$resolution" | grep '^ASSET_VERSION=' | cut -d'=' -f2-)"
check "resolver reports a strict-manifest checksum" "64" \
    "$(echo "$resolution" | grep '^ASSET_CHECKSUM=' | cut -d'=' -f2- | tr -d '\n' | wc -c | tr -d ' ')"

# An unavailable version must be refused, not silently substituted.
"$RESOLVER" terraphim-agent 1.20.5 >/dev/null 2>&1
check "unavailable version refused (exit 2)" "2" "$?"

# --------------------------------------------------------------------------
# 3. Install and run the real binary
# --------------------------------------------------------------------------
blue "3. Install and run"

install_dir="$WORK/install"
"$INSTALLER" --install-dir "$install_dir" --verbose >"$WORK/install.log" 2>&1
check "installer exits 0" "0" "$?"
check "binary is present" "present" \
    "$([[ -x "$install_dir/terraphim-agent" ]] && echo present || echo absent)"

if [[ -x "$install_dir/terraphim-agent" ]]; then
    reported=$("$install_dir/terraphim-agent" --version 2>/dev/null || echo "no-version")
    check "installed binary reports 1.21.16" "terraphim-agent 1.21.16" "$reported"
    size=$(wc -c <"$install_dir/terraphim-agent" | tr -d ' ')
    check "installed binary is non-trivial" "yes" \
        "$([[ "$size" -gt 1000000 ]] && echo yes || echo no)"
else
    check "installed binary runs" "runs" "binary absent"
fi

# --------------------------------------------------------------------------
# 4. Fail-closed on a manipulated release
# --------------------------------------------------------------------------
blue "4. Fail-closed on manipulation"

# Serve a manifest whose digests are wrong, proxying archive bytes to the real
# channel. The installer must refuse to install.
cat >"$WORK/proxy.py" <<'PY'
import http.server, socketserver, urllib.request

CHANNEL = "https://downloads.terraphim.ai"
UA = "terraphim-installer/2.0.0"

class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path.endswith("stable-v2.json"):
            return super().do_GET()
        req = urllib.request.Request(CHANNEL + self.path, headers={"User-Agent": UA})
        with urllib.request.urlopen(req, timeout=60) as upstream:
            body = upstream.read()
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass

socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", 0), Handler) as httpd:
    print(httpd.server_address[1], flush=True)
    httpd.serve_forever()
PY

manifest_dir="$WORK/manifest/terraphim-agent"
mkdir -p "$manifest_dir"
if curl -fsS --user-agent "terraphim-installer/2.0.0" \
        "https://downloads.terraphim.ai/terraphim-agent/stable-v2.json" \
        -o "$WORK/real-manifest.json"; then
    check "live manifest fetchable" "ok" "ok"
    python3 - "$WORK/real-manifest.json" "$manifest_dir/stable-v2.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
for asset in data["assets"].values():
    asset["sha256"] = "0" * 64
json.dump(data, open(sys.argv[2], "w"))
PY

    (cd "$WORK/manifest" && exec python3 "$WORK/proxy.py") >"$WORK/proxy.port" 2>"$WORK/proxy.err" &
    proxy_pid=$!
    for _ in $(seq 1 50); do
        [[ -s "$WORK/proxy.port" ]] && break
        sleep 0.1
    done

    if [[ -s "$WORK/proxy.port" ]]; then
        port=$(head -1 "$WORK/proxy.port")
        bad_dir="$WORK/bad-install"
        TERRAPHIM_CHANNEL_BASE="http://127.0.0.1:${port}" \
            "$INSTALLER" --install-dir "$bad_dir" >"$WORK/bad.log" 2>&1
        check "tampered release refused (exit 5)" "5" "$?"
        check "nothing installed from tampered release" "0" \
            "$(ls -A "$bad_dir" 2>/dev/null | wc -l | tr -d ' ')"
    else
        check "tamper test server started" "started" "failed: $(cat "$WORK/proxy.err" 2>/dev/null)"
    fi
    kill "$proxy_pid" 2>/dev/null
    wait "$proxy_pid" 2>/dev/null
else
    check "live manifest fetchable" "ok" "unreachable"
fi

# --------------------------------------------------------------------------
echo
blue "Results: ${PASS} passed, ${FAIL} failed"
[[ $FAIL -eq 0 ]] || exit 1

green "Installer release gate passed."