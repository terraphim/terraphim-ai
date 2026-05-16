#!/bin/bash
# @file terraphim-rlm-hook.sh
# Claude Code hook: RLM integration for secure code execution.
#
# This hook allows Claude Code to use terraphim_rlm for:
# - Secure code execution in isolated VMs
# - Recursive LLM query loops
# - Knowledge graph validation
#
# Install:
#   cp terraphim-rlm-hook.sh ~/.claude/hooks/
#   chmod +x ~/.claude/hooks/terraphim-rlm-hook.sh
#
# Configure in ~/.claude/settings.local.json:
#   {
#     "hooks": {
#       "PreToolUse": [{
#         "matcher": "Bash",
#         "hooks": [{
#           "type": "command",
#           "command": "~/.claude/hooks/terraphim-rlm-hook.sh"
#         }]
#       }]
#     }
#   }
#
# Requirements:
#   - bash (>= 4)
#   - jq (POSIX-portable JSON encoding)
#   - terraphim_mcp_server on PATH or set $TERRAPHIM_MCP

TERRAPHIM_AGENT="${TERRAPHIM_AGENT:-$HOME/.cargo/bin/terraphim-agent}"
TERRAPHIM_MCP="${TERRAPHIM_MCP:-terraphim_mcp_server}"

RLM_TIMEOUT_SECS=30

log_debug() {
    if [[ "${TERRAPHIM_DEBUG:-0}" == "1" ]]; then
        echo "[terraphim-rlm] $1" >&2
    fi
}

# Portable timeout wrapper. Reads stdin, runs the command in its own process
# group with a hard kill after $1 seconds, propagates stdout/stderr. Uses
# pure POSIX shell so it works on macOS without GNU coreutils (`gtimeout`).
#
# Process-group semantics: the child runs under `setsid` (or, on macOS where
# setsid lacks the `--` form, an inline pgid trick). Both the watchdog
# escalation kill and the watchdog teardown kill use negative pids, so a
# recycled pid cannot be hit even if the wait() race fires.
run_with_timeout() {
    local timeout_secs="$1"; shift

    # Start command in a new process group so kill -- -PGID hits exactly the
    # descendant tree. `setsid` is POSIX on Linux; macOS bash ships it via
    # /usr/bin/setsid (since 10.15). Fall back to plain & on systems lacking
    # setsid (rare).
    if command -v setsid >/dev/null 2>&1; then
        setsid -- "$@" &
    else
        "$@" &
    fi
    local pid=$!

    (
        sleep "$timeout_secs"
        # Negative pid = process group; harmless if pid already exited.
        kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null
        sleep 1
        kill -KILL -- "-$pid" 2>/dev/null || kill -KILL "$pid" 2>/dev/null
    ) &
    local watcher=$!
    local rc=0
    if wait "$pid" 2>/dev/null; then
        rc=0
    else
        rc=$?
    fi
    # Tear down the watchdog. It targets its own pid (this shell's child),
    # not the recycled pid space.
    kill -KILL "$watcher" 2>/dev/null
    wait "$watcher" 2>/dev/null
    return $rc
}

# Build a JSON-RPC `tools/call` request body using jq so all string values
# are correctly escaped, then send it on stdin to the MCP server. Stderr is
# propagated (not silenced) so operators can debug failed invocations.
call_mcp_tool() {
    local tool="$1"
    local args_json="$2"   # caller passes a pre-built JSON object string

    local request
    request=$(jq -nc --arg tool "$tool" --argjson args "$args_json" \
        '{jsonrpc: "2.0", id: 1, method: "tools/call",
          params: {name: $tool, arguments: $args}}')

    printf '%s\n' "$request" | run_with_timeout "$RLM_TIMEOUT_SECS" "$TERRAPHIM_MCP"
}

rlm_query() {
    local prompt="$1"
    local session_id="${2:-}"
    local args_json
    if [[ -n "$session_id" ]]; then
        args_json=$(jq -nc --arg p "$prompt" --arg s "$session_id" \
            '{prompt: $p, session_id: $s}')
    else
        args_json=$(jq -nc --arg p "$prompt" '{prompt: $p}')
    fi
    call_mcp_tool "rlm_query" "$args_json"
}

rlm_code() {
    local code="$1"
    local session_id="${2:-}"
    local args_json
    if [[ -n "$session_id" ]]; then
        args_json=$(jq -nc --arg c "$code" --arg s "$session_id" \
            '{code: $c, session_id: $s}')
    else
        args_json=$(jq -nc --arg c "$code" '{code: $c}')
    fi
    call_mcp_tool "rlm_code" "$args_json"
}

rlm_bash() {
    local command="$1"
    local session_id="${2:-}"
    local args_json
    if [[ -n "$session_id" ]]; then
        args_json=$(jq -nc --arg c "$command" --arg s "$session_id" \
            '{command: $c, session_id: $s}')
    else
        args_json=$(jq -nc --arg c "$command" '{command: $c}')
    fi
    call_mcp_tool "rlm_bash" "$args_json"
}

rlm_status() {
    local session_id="${1:-}"
    local args_json
    if [[ -n "$session_id" ]]; then
        args_json=$(jq -nc --arg s "$session_id" '{session_id: $s}')
    else
        args_json='{}'
    fi
    call_mcp_tool "rlm_status" "$args_json"
}

main() {
    local input_json="$1"

    if [[ -z "$input_json" ]]; then
        cat
        return 0
    fi

    local tool_name
    local tool_input
    local command

    tool_name=$(echo "$input_json" | jq -r '.tool_name // .toolName // empty')
    tool_input=$(echo "$input_json" | jq -r '.tool_input // .toolInput // .command // empty')

    if [[ "$tool_name" != "Bash" ]]; then
        echo "$input_json"
        return 0
    fi

    if [[ -z "$tool_input" ]]; then
        echo "$input_json"
        return 0
    fi

    command=$(echo "$tool_input" | jq -r '.command // . // empty')

    if [[ -z "$command" ]]; then
        echo "$input_json"
        return 0
    fi

    log_debug "Processing command: $command"

    if echo "$command" | grep -qE '^\s*(rlm|rlm_query|rlm_code|rlm_bash|rlm_status)\s'; then
        local rlm_cmd
        local rlm_args
        local result

        # Bash parameter expansion preserves internal whitespace and any
        # literal quotes in the prompt, unlike `awk '{$1=""; print $0}'`
        # which collapses runs of whitespace.
        rlm_cmd="${command%% *}"
        rlm_cmd="$(printf '%s' "$rlm_cmd" | tr '[:upper:]' '[:lower:]')"
        # Strip the first word (and exactly one separating space) to get
        # everything else verbatim. If the command has no trailing args,
        # the result is empty.
        if [[ "$command" == *" "* ]]; then
            rlm_args="${command#* }"
        else
            rlm_args=""
        fi

        log_debug "RLM command: $rlm_cmd"
        log_debug "RLM args: $rlm_args"

        case "$rlm_cmd" in
            rlm|rlm_query)
                result=$(rlm_query "$rlm_args")
                ;;
            rlm_code)
                result=$(rlm_code "$rlm_args")
                ;;
            rlm_bash)
                result=$(rlm_bash "$rlm_args")
                ;;
            rlm_status)
                result=$(rlm_status "$rlm_args")
                ;;
            *)
                echo "$input_json"
                return 0
                ;;
        esac

        if [[ -n "$result" ]]; then
            log_debug "RLM result: $result"
            echo "$result"
        else
            echo "$input_json"
        fi
    else
        echo "$input_json"
    fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$(cat)"
else
    main "$@"
fi
