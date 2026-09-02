#!/usr/bin/env bash
#
# Self-test for check-bind-addresses.sh (#3122)
#
# Exercises the gate against synthetic temp repos (unsafe / annotated /
# loopback-safe / excluded scenarios) and the real repo. No mocks of internal
# logic -- the gate runs in full against real fixtures on disk.
#
# Usage: ./scripts/test-check-bind-addresses.sh
# Exit:  0 if all assertions hold, 1 otherwise.
#
# Refs #3122

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATE="$SCRIPT_DIR/check-bind-addresses.sh"

if [[ ! -x "$GATE" ]]; then
    chmod +x "$GATE" 2>/dev/null || true
fi
if [[ ! -r "$GATE" ]]; then
    echo "ERROR: gate script not found: $GATE" >&2
    exit 2
fi

if [[ -t 1 ]]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    NC='\033[0m'
else
    GREEN=''; RED=''; NC=''
fi

PASS_COUNT=0
FAIL_COUNT=0

cleanup() {
    [[ -n "${TMPROOT:-}" ]] && rm -rf "$TMPROOT"
}
TMPROOT=""
trap cleanup EXIT

# run_gate <root> -- invoke the gate, capture combined output, echo exit code.
run_gate() {
    local root="$1"
    "$GATE" "$root"
}

# assert_exit <expected_exit> <label> <root>
assert_exit() {
    local expected="$1"
    local label="$2"
    local root="$3"
    if "$GATE" "$root" >/tmp/bind_gate_test.out 2>&1; then
        local actual=0
    else
        local actual=$?
    fi
    if [[ "$actual" -eq "$expected" ]]; then
        echo -e "  ${GREEN}✓${NC} $label (exit $actual)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "  ${RED}✗${NC} $label (expected exit $expected, got $actual)" >&2
        cat /tmp/bind_gate_test.out >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

# assert_output <pattern> <label> <root> -- run the gate and grep its output.
assert_output() {
    local pattern="$1"
    local label="$2"
    local root="$3"
    local out
    out=$("$GATE" "$root" 2>&1) || true
    if grep -qE "$pattern" <<<"$out"; then
        echo -e "  ${GREEN}✓${NC} $label"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "  ${RED}✗${NC} $label (pattern not found: $pattern)" >&2
        echo "--- output ---" >&2
        echo "$out" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

echo "Testing check-bind-addresses.sh"
echo "================================"

TMPROOT="$(mktemp -d -t bind-gate-test.XXXXXX)"

# ---------------------------------------------------------------------------
# Fixture helpers
# ---------------------------------------------------------------------------
write_compose() {
    local root="$1"
    mkdir -p "$root/docker"
}

# ---------------------------------------------------------------------------
# Test 1: unprefixed docker-compose port is FLAGGED (the #3115 class)
# ---------------------------------------------------------------------------
T1="$TMPROOT/t1-unsafe-port"
mkdir -p "$T1/docker"
cat >"$T1/docker/docker-compose.yml" <<'YAML'
services:
  web:
    image: nginx
    ports:
      - "8000:8000"
YAML
assert_exit 1 "unsafe unprefixed port -> exit 1" "$T1"
assert_output 'docker/docker-compose\.yml:5' "finding names file:line" "$T1"

# ---------------------------------------------------------------------------
# Test 2: loopback-prefixed port is SAFE
# ---------------------------------------------------------------------------
T2="$TMPROOT/t2-loopback-safe"
mkdir -p "$T2/docker"
cat >"$T2/docker/docker-compose.yml" <<'YAML'
services:
  redis:
    image: redis
    ports:
      - "127.0.0.1:6379:6379"
YAML
assert_exit 0 "loopback-prefixed port -> exit 0" "$T2"

# ---------------------------------------------------------------------------
# Test 3: Tailscale/private-IP-prefixed port is SAFE
# ---------------------------------------------------------------------------
T3="$TMPROOT/t3-private-safe"
mkdir -p "$T3/docker"
cat >"$T3/docker/docker-compose.yml" <<'YAML'
services:
  s3:
    image: minio
    ports:
      - "100.106.66.7:9000:9000"
YAML
assert_exit 0 "private-IP-prefixed port -> exit 0" "$T3"

# ---------------------------------------------------------------------------
# Test 4: annotation on the line directly above suppresses the finding
# ---------------------------------------------------------------------------
T4="$TMPROOT/t4-annotated-above"
mkdir -p "$T4/docker"
cat >"$T4/docker/docker-compose.yml" <<'YAML'
services:
  proxy:
    image: nginx
    ports:
      # security-ok: reverse proxy front door with auth
      - "80:80"
YAML
assert_exit 0 "annotated port (comment above) -> exit 0" "$T4"

# ---------------------------------------------------------------------------
# Test 5: annotation in a multi-line comment block above suppresses
# ---------------------------------------------------------------------------
T5="$TMPROOT/t5-annotated-block"
mkdir -p "$T5/docker"
cat >"$T5/docker/docker-compose.yml" <<'YAML'
services:
  proxy:
    image: nginx
    ports:
      # security-ok: reverse proxy
      # multi-line rationale documenting why this is safe
      # in production it sits behind an authenticating firewall
      - "80:80"
YAML
assert_exit 0 "annotated port (multi-line block) -> exit 0" "$T5"

# ---------------------------------------------------------------------------
# Test 6: systemd ExecStart wildcard bind FLAGGED
# ---------------------------------------------------------------------------
T6="$TMPROOT/t6-systemd-unsafe"
mkdir -p "$T6"
cat >"$T6/proxy.service" <<'SVC'
[Service]
ExecStart=/usr/bin/proxy --listen 0.0.0.0:3456
SVC
assert_exit 1 "systemd 0.0.0.0 ExecStart -> exit 1" "$T6"
assert_output 'proxy\.service:2' "systemd finding names file:line" "$T6"

# ---------------------------------------------------------------------------
# Test 7: Dockerfile ENV wildcard bind FLAGGED
# ---------------------------------------------------------------------------
T7="$TMPROOT/t7-dockerfile-unsafe"
mkdir -p "$T7"
cat >"$T7/Dockerfile" <<'DOCKER'
FROM alpine
ENV TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8000"
DOCKER
assert_exit 1 "Dockerfile ENV 0.0.0.0 -> exit 1" "$T7"

# ---------------------------------------------------------------------------
# Test 8: container-internal 0.0.0.0 WITHOUT host port is annotated -> SAFE
#         (mirrors OLLAMA_HOST idiom)
# ---------------------------------------------------------------------------
T8="$TMPROOT/t8-container-internal-annotated"
mkdir -p "$T8/docker"
cat >"$T8/docker/docker-compose.yml" <<'YAML'
services:
  ollama:
    image: ollama/ollama
    ports:
      - "127.0.0.1:11434:11434"
    environment:
      # security-ok: container-internal listener; host port is loopback-only
      - OLLAMA_HOST=0.0.0.0
YAML
assert_exit 0 "annotated container-internal bind -> exit 0" "$T8"

# ---------------------------------------------------------------------------
# Test 9: NOISE does not false-positive (0.0.0.0 in prose/comment)
# ---------------------------------------------------------------------------
T9="$TMPROOT/t9-noise-safe"
mkdir -p "$T9"
cat >"$T9/notes.md" <<'MD'
# Security notes
Redis was previously exposed on 0.0.0.0:6379 (fixed in #1313).
This is a prose mention, not a bind.
MD
assert_exit 0 "prose 0.0.0.0 mention -> exit 0 (no false positive)" "$T9"

# ---------------------------------------------------------------------------
# Test 10: regression -- a deliberate 0.0.0.0 service bind is flagged
#         (direct AC from issue #3122: "a test fixture with a deliberate
#          0.0.0.0 bind is correctly flagged")
# ---------------------------------------------------------------------------
T10="$TMPROOT/t10-regression"
mkdir -p "$T10/systemd"
cat >"$T10/systemd/llm-proxy.service" <<'SVC'
[Service]
ExecStart=/usr/local/bin/llm-proxy --addr 0.0.0.0:3456
SVC
assert_exit 1 "regression fixture: 0.0.0.0 bind flagged" "$T10"
assert_output 'llm-proxy\.service' "regression names the offending service file" "$T10"

# ---------------------------------------------------------------------------
# Test 11: real-repo run -- must pass (we annotated the legit exceptions)
# ---------------------------------------------------------------------------
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
assert_exit 0 "real repo -> exit 0 (exceptions annotated)" "$PROJECT_ROOT"

# ---------------------------------------------------------------------------
# Test 12: timing budget (< 10s per AC)
# ---------------------------------------------------------------------------
START=$(date +%s.%N)
"$GATE" "$PROJECT_ROOT" >/dev/null 2>&1 || true
END=$(date +%s.%N)
ELAPSED=$(awk -v s="$START" -v e="$END" 'BEGIN{printf "%.2f", e-s}')
if awk -v t="$ELAPSED" 'BEGIN{exit !(t < 10)}'; then
    echo -e "  ${GREEN}✓${NC} timing budget: ${ELAPSED}s (< 10s)"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "  ${RED}✗${NC} timing budget: ${ELAPSED}s (>= 10s)" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "================================"
if [[ "$FAIL_COUNT" -eq 0 ]]; then
    echo -e "${GREEN}✓ All ${PASS_COUNT} assertions passed${NC}"
    exit 0
else
    echo -e "${RED}✗ ${FAIL_COUNT} assertion(s) failed (${PASS_COUNT} passed)${NC}"
    exit 1
fi
