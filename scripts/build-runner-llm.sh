#!/bin/bash
set -euo pipefail
# build-runner-llm.sh -- KG-first adaptive build runner

# Error tracing: print line number and failing command on ERR
trap 'echo "[build-runner ERROR] line $LINENO: $BASH_COMMAND" >&2' ERR
# Design: .docs/design-build-runner-llm-v4-leverage-existing.md
# Epic: #1423

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="${ADF_WORKING_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"
BUILD_MD="${PROJECT_DIR}/BUILD.md"
TERRAPHIM_AGENT="${HOME}/.cargo/bin/terraphim-agent"
RCH="$(which rch 2>/dev/null || echo "")"

# --- Helpers ---
red()    { printf '\033[31m%s\033[0m\n' "$*"; }
green()  { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }

post_status() {
    local state="$1" desc="$2"
    if [ -n "${GITEA_TOKEN:-}" ] && [ -n "${ADF_PUSH_SHA:-}" ]; then
        curl -fsS -X POST \
            -H "Authorization: token $GITEA_TOKEN" \
            -H "Content-Type: application/json" \
            -d "{\"state\":\"$state\",\"context\":\"adf/build\",\"description\":\"$desc\"}" \
            "$GITEA_URL/api/v1/repos/${GITEA_OWNER:-terraphim}/${GITEA_REPO:-terraphim-ai}/statuses/$ADF_PUSH_SHA" \
            >/dev/null 2>&1 || true
    fi
}

# --- Phase 1: Detect build commands ---
detect_commands() {
    # BUILD_COMMANDS env var overrides detection (for proofs/tests)
    if [ -n "${BUILD_COMMANDS:-}" ]; then
        echo "$BUILD_COMMANDS"
        return
    fi

    # Priority: BUILD.md bash blocks > ci-pr.yml > Cargo.toml defaults
    if [ -f "$BUILD_MD" ]; then
        local cmds
        # Extract bash code blocks but filter to only build-relevant commands
        cmds=$(awk '/^```bash$/,/^```$/' "$BUILD_MD" 2>/dev/null \
            | grep -v '^```' | grep -v '^$' | grep -v '^#' \
            | grep -v '^# ' \
            | grep -E '^(cargo|make|bun|npm|yarn|pnpm|rch|docker|poetry|uv|go |zig )' || true)
        if [ -n "$cmds" ]; then
            echo "$cmds"
            return
        fi
    fi

    if [ -f "$PROJECT_DIR/.github/workflows/ci-pr.yml" ]; then
        if command -v yq &>/dev/null; then
            yq '.jobs[].steps[].run' "$PROJECT_DIR/.github/workflows/ci-pr.yml" 2>/dev/null | grep -v null || true
            return
        fi
    fi

    # Hardcoded Rust workspace fallback
    echo "cargo fmt --all -- --check"
    echo "cargo clippy --workspace --all-targets -- -D warnings"
    echo "cargo build --workspace --profile ci"
    echo "cargo test --workspace --no-fail-fast --profile ci"
}

# --- Phase 2: Transform command via KG ---
# Uses terraphim-agent replace for tool substitution.
# Only transforms if a DevOpsRunner role exists; otherwise passes through.
transform_command() {
    local cmd="$1"
    if [ ! -x "$TERRAPHIM_AGENT" ]; then
        echo "$cmd"
        return
    fi
    local transformed
    transformed=$(echo "$cmd" | "$TERRAPHIM_AGENT" replace --role "Terraphim Engineer" 2>/dev/null || true)
    # Only accept the transformation if it still starts with a known tool
    if [ -n "$transformed" ] && echo "$transformed" | grep -qE '^(cargo|rch|make|bun|npm|yarn|pnpm|docker)'; then
        echo "$transformed"
    else
        echo "$cmd"
    fi
}

# --- Phase 3: Validate command against whitelist ---
is_allowed() {
    local cmd="$1"
    local first_word
    first_word=$(echo "$cmd" | awk '{print $1}')
    case "$first_word" in
        cargo|make|bun|npm|yarn|pnpm|rch|docker|echo|mkdir|git|ls|cat|cd|cp|mv|rm|chmod)
            return 0 ;;
        *)
            return 1 ;;
    esac
}

# --- Phase 5: LLM fallback on failure ---
llm_correct_command() {
    local failed_cmd="$1"
    local exit_code="$2"
    local error_tail="$3"
    local claude_bin="${HOME}/.local/bin/claude"

    if [ ! -x "$claude_bin" ]; then
        echo ""
        return
    fi

    local prompt="A build command failed in CI.

Command: $failed_cmd
Exit code: $exit_code
Error output: $(echo "$error_tail" | tail -5)

Diagnose why it failed. Suggest the corrected command.
This is for a Rust workspace project (terraphim-ai).

Output format:
CORRECTION: <corrected bash command>
REASON: <one-line explanation>"

    "$claude_bin" -p "$prompt" --model haiku --output-format text < /dev/null 2>/dev/null || echo ""
}

update_build_md() {
    local failed_cmd="$1"
    local correction="$2"

    if [ ! -f "$BUILD_MD" ]; then
        return
    fi
    {
        echo ""
        echo "## Auto-corrected ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
        echo ""
        echo "Failed: \`$failed_cmd\`"
        echo "$correction"
    } >> "$BUILD_MD"
    yellow "  BUILD.md updated with correction"
}

# --- Main ---
main() {
    echo ""
    green "=== build-runner-llm.sh ==="
    echo "Project: $PROJECT_DIR"
    echo "Design: KG-first, BUILD.md source, LLM fallback on failure"
    echo ""

    # Post pending status (best-effort)
    post_status pending "build started"

    local commands
    commands=$(detect_commands)
    if [ -z "$commands" ]; then
        red "No build commands detected. Aborting."
        exit 1
    fi

    echo "Detected commands:"
    echo "$commands" | while IFS= read -r line; do echo "  $line"; done
    echo ""

    local total=0 success=0

    while IFS= read -r cmd; do
        [ -z "$cmd" ] && continue
        total=$((total + 1))

        # Phase 2: KG Transform
        local transformed
        transformed=$(transform_command "$cmd")
        local kg_note=""
        if [ "$transformed" != "$cmd" ]; then
            kg_note=" ← KG transformed"
        fi

        # Phase 3: Validate
        if ! is_allowed "$transformed"; then
            yellow "[$total] BLOCKED: $transformed (not in whitelist)"
            continue
        fi

        # Phase 4: Execute
        printf "[%d/%d] %s%s ... " "$total" "$(echo "$commands" | wc -l | tr -d ' ')" "$transformed" "$kg_note"

        local error_output
        if error_output=$(eval "$transformed" 2>&1); then
            success=$((success + 1))
            green "PASS"
        else
            local exit_code=$?
            red "FAIL (exit $exit_code)"

            # Show error context
            local error_tail
            error_tail=$(echo "$error_output" | tail -5)
            echo "  Error tail:"
            echo "$error_tail" | while IFS= read -r line; do echo "    $line"; done

            # Phase 5: LLM fallback
            yellow "  Invoking LLM for correction (haiku)..."
            local correction
            correction=$(llm_correct_command "$transformed" "$exit_code" "$error_tail")
            if [ -n "$correction" ]; then
                echo "  LLM response:"
                echo "$correction" | while IFS= read -r line; do echo "    $line"; done
                update_build_md "$transformed" "$correction"
            else
                yellow "  LLM unavailable or returned empty; skipping correction"
            fi
        fi
    done <<< "$commands"

    echo ""
    echo "---"
    if [ "$success" -eq "$total" ]; then
        green "Build PASSED ($success/$total steps)"
        post_status success "build passed ($success/$total steps)"
    else
        local failed=$((total - success))
        red "Build FAILED ($success/$total passed, $failed failed)"
        post_status failure "$failed/$total steps failed"
    fi
    echo ""

    [ "$success" -eq "$total" ]
}

main "$@"
