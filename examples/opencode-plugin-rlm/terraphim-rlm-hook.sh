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

TERRAPHIM_AGENT="${TERRAPHIM_AGENT:-$HOME/.cargo/bin/terraphim-agent}"
TERRAPHIM_MCP="${TERRAPHIM_MCP:-terraphim_mcp_server}"

RLM_TIMEOUT_MS=30000

log_debug() {
    if [[ "${TERRAPHIM_DEBUG:-0}" == "1" ]]; then
        echo "[terraphim-rlm] $1" >&2
    fi
}

call_mcp_tool() {
    local tool="$1"
    local args="$2"

    echo "{\"jsonrpc\":\"2.0\",\"id\":$$,\"method\":\"tools/call\",\"params\":{\"name\":\"$tool\",\"arguments\":$args}}" | \
        timeout 30 "$TERRAPHIM_MCP" 2>/dev/null
}

rlm_query() {
    local prompt="$1"
    local session_id="${2:-}"

    if [[ -n "$session_id" ]]; then
        call_mcp_tool "rlm_query" "{\"prompt\":\"$prompt\",\"session_id\":\"$session_id\"}"
    else
        call_mcp_tool "rlm_query" "{\"prompt\":\"$prompt\"}"
    fi
}

rlm_code() {
    local code="$1"
    local session_id="${2:-}"

    if [[ -n "$session_id" ]]; then
        call_mcp_tool "rlm_code" "{\"code\":\"$code\",\"session_id\":\"$session_id\"}"
    else
        call_mcp_tool "rlm_code" "{\"code\":\"$code\"}"
    fi
}

rlm_bash() {
    local command="$1"
    local session_id="${2:-}"

    if [[ -n "$session_id" ]]; then
        call_mcp_tool "rlm_bash" "{\"command\":\"$command\",\"session_id\":\"$session_id\"}"
    else
        call_mcp_tool "rlm_bash" "{\"command\":\"$command\"}"
    fi
}

rlm_status() {
    local session_id="${1:-}"

    if [[ -n "$session_id" ]]; then
        call_mcp_tool "rlm_status" "{\"session_id\":\"$session_id\"}"
    else
        call_mcp_tool "rlm_status" "{}"
    fi
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

        rlm_cmd=$(echo "$command" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
        rlm_args=$(echo "$command" | awk '{$1=""; print $0}' | sed 's/^[[:space:]]*//')

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
