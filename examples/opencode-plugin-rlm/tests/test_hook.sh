#!/usr/bin/env bash
# Smoke tests for terraphim-rlm-hook.sh.
#
# These tests exercise the hook's input parsing, JSON construction, and
# portable timeout wrapper. They do NOT require a running MCP server: each
# test stubs $TERRAPHIM_MCP with a script that captures its stdin so we can
# inspect what would have been sent.
#
# Usage:
#   bash test_hook.sh
#
# Requirements:
#   - bash (>= 4)
#   - jq
#
# Exits non-zero on any failure.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOK="$SCRIPT_DIR/../terraphim-rlm-hook.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! command -v jq >/dev/null 2>&1; then
    echo "FAIL: jq is required to run these tests." >&2
    exit 2
fi

if [[ ! -x "$HOOK" ]]; then
    echo "FAIL: hook script not executable: $HOOK" >&2
    exit 2
fi

# Stub MCP server: capture stdin to a file, exit 0.
cat > "$TMP/mcp-stub.sh" <<'STUB'
#!/usr/bin/env bash
cat > "$TERRAPHIM_TEST_CAPTURE"
echo '{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"ok"}]}}'
STUB
chmod +x "$TMP/mcp-stub.sh"

PASS=0
FAIL=0
fail() { echo "FAIL: $1"; FAIL=$((FAIL + 1)); }
pass() { echo "PASS: $1"; PASS=$((PASS + 1)); }

#
# Test 1: A prompt containing double quotes does not produce malformed JSON.
#
TEST_NAME="quoted_prompt_does_not_break_json"
CAPTURE="$TMP/capture-1.txt"
INPUT_1=$(jq -nc --arg cmd 'rlm_query "hello \"world\""' \
    '{tool_name: "Bash", tool_input: {command: $cmd}}')
echo "$INPUT_1" | TERRAPHIM_TEST_CAPTURE="$CAPTURE" \
    TERRAPHIM_MCP="$TMP/mcp-stub.sh" \
    bash "$HOOK" >/dev/null 2>&1 || true

if [[ ! -s "$CAPTURE" ]]; then
    fail "$TEST_NAME: stub did not capture any input"
elif ! jq -e . "$CAPTURE" >/dev/null 2>&1; then
    fail "$TEST_NAME: captured payload is not valid JSON"
    cat "$CAPTURE"
else
    pass "$TEST_NAME"
fi

#
# Test 2: Non-RLM Bash commands pass through untouched.
#
TEST_NAME="passthrough_non_rlm_command"
INPUT_2='{"tool_name":"Bash","tool_input":{"command":"echo hello"}}'
OUTPUT=$(echo "$INPUT_2" | bash "$HOOK")
if [[ "$OUTPUT" == "$INPUT_2" ]]; then
    pass "$TEST_NAME"
else
    fail "$TEST_NAME: expected passthrough, got: $OUTPUT"
fi

#
# Test 3: run_with_timeout is portable - works without GNU `timeout` or
# `gtimeout` on PATH, and kills its child within ~$timeout seconds.
#
TEST_NAME="run_with_timeout_kills_child"
SOURCE_HOOK="$HOOK" PATH="/usr/bin:/bin" bash -c '
    source "$SOURCE_HOOK"
    if command -v timeout >/dev/null 2>&1; then
        echo "skipped: GNU timeout present on stripped PATH"
        exit 0
    fi
    if command -v gtimeout >/dev/null 2>&1; then
        echo "skipped: gtimeout present on stripped PATH"
        exit 0
    fi
    start=$SECONDS
    run_with_timeout 1 sleep 30
    elapsed=$((SECONDS - start))
    if [[ $elapsed -gt 4 ]]; then
        echo "FAIL: timeout did not fire within 4s, took ${elapsed}s"
        exit 1
    fi
'
case $? in
    0) pass "$TEST_NAME" ;;
    *) fail "$TEST_NAME: timeout wrapper did not behave as expected" ;;
esac

echo
echo "RESULT: $PASS passed, $FAIL failed"
exit $FAIL
