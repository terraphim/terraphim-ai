#!/bin/bash
# build-runner-llm.sh - Fast/cheap LLM build runner with KG-first architecture
# Part of Epic #1423: fast/cheap LLM build-runner
#
# This script:
# 1. Detects CI configuration (GitHub Actions, Earthfile, Makefile)
# 2. Extracts build commands using terraphim-agent with DevOpsRunner role
# 3. Validates commands against whitelist
# 4. Transforms commands via terraphim-agent replace (KG lookup)
# 5. Executes with rch remote compilation support
# 6. Posts status to Gitea
# 7. Tracks costs and alerts on threshold breaches

set -e

export GITEA_URL=https://git.terraphim.cloud
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/bin:$HOME/.bun/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# Cost tracking configuration
COST_THRESHOLD_WARN=0.01    # $0.01 - warning threshold
COST_THRESHOLD_FAIL=0.05    # $0.05 - fail threshold
COST_PER_KG_LOOKUP=0.0001   # $0.0001 per KG lookup (Aho-Corasick)
COST_PER_LLM_CALL=0.005     # $0.005 per LLM extraction call
TOTAL_COST=0
LLM_CALLS=0
KG_LOOKUPS=0

# Status helper
POST_STATUS() {
  local STATE="$1"
  local DESC="$2"
  if [ -n "$GITEA_TOKEN" ] && [ -n "$ADF_PUSH_SHA" ]; then
    curl -fsS -X POST \
      -H "Authorization: token $GITEA_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{\"state\":\"$STATE\",\"context\":\"adf/build\",\"description\":\"$DESC\"}" \
      "$GITEA_URL/api/v1/repos/$GITEA_OWNER/$GITEA_REPO/statuses/$ADF_PUSH_SHA" \
      >/dev/null 2>&1 || true
  fi
}

# Logging helpers
log_info() { echo "[INFO] $1" >&2; }
log_success() { echo "[SUCCESS] $1" >&2; }
log_warning() { echo "[WARNING] $1" >&2; }
log_error() { echo "[ERROR] $1" >&2; }

# Cost tracking helpers
track_kg_lookup() {
  KG_LOOKUPS=$((KG_LOOKUPS + 1))
  TOTAL_COST=$(echo "$TOTAL_COST + $COST_PER_KG_LOOKUP" | bc -l 2>/dev/null || echo "$TOTAL_COST")
}

track_llm_call() {
  LLM_CALLS=$((LLM_CALLS + 1))
  TOTAL_COST=$(echo "$TOTAL_COST + $COST_PER_LLM_CALL" | bc -l 2>/dev/null || echo "$TOTAL_COST")
}

check_cost_thresholds() {
  # Warn if over warning threshold
  if (( $(echo "$TOTAL_COST > $COST_THRESHOLD_WARN" | bc -l 2>/dev/null || echo 0) )); then
    log_warning "Build cost \$$TOTAL_COST exceeds warning threshold (\$$COST_THRESHOLD_WARN)"
    if (( $(echo "$TOTAL_COST > $COST_THRESHOLD_FAIL" | bc -l 2>/dev/null || echo 0) )); then
      log_error "Build cost \$$TOTAL_COST exceeds fail threshold (\$$COST_THRESHOLD_FAIL)"
      POST_STATUS failure "build failed: cost \$$TOTAL_COST exceeds threshold \$$COST_THRESHOLD_FAIL"
      send_cost_metrics "cost_exceeded"
      exit 1
    fi
  fi
}

# Send cost metrics to Quickwit
send_cost_metrics() {
  local status="${1:-success}"
  local timestamp
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  local metrics_payload
  metrics_payload=$(cat <<EOF
{
  "timestamp": "$timestamp",
  "agent": "build-runner-llm",
  "project": "${GITEA_REPO:-unknown}",
  "sha": "${ADF_PUSH_SHA:-unknown}",
  "model": "haiku",
  "cost_cents": $TOTAL_COST,
  "kg_lookups": $KG_LOOKUPS,
  "llm_calls": $LLM_CALLS,
  "status": "$status"
}
EOF
)

  # Send to Quickwit if configured
  if [ -n "$QUICKWIT_ENDPOINT" ] && [ -n "$QUICKWIT_INDEX" ]; then
    curl -fsS -X POST \
      -H "Content-Type: application/json" \
      -d "$metrics_payload" \
      "$QUICKWIT_ENDPOINT/api/v1/$QUICKWIT_INDEX/ingest" \
      >/dev/null 2>&1 || true
  fi

  # Also log to stdout
  log_info "Cost report: \$$TOTAL_COST total (KG: $KG_LOOKUPS lookups, LLM: $LLM_CALLS calls)"
}

# Command whitelist validation
validate_command() {
  local cmd="$1"
  # Allowed patterns
  if echo "$cmd" | grep -qE '^(cargo|make|npm|yarn|pnpm|bun|docker|docker-compose|yq|test|echo|cat|ls|cd|mkdir|rm|cp|mv|git|curl|wget|tar|unzip|zip|chmod|chown|source|export|eval)' ; then
    return 0
  fi
  # Reject dangerous patterns
  if echo "$cmd" | grep -qE '(sudo|curl.*\|.*sh|wget.*\|.*sh|\bls\b.*\|.*rm|\brm\b.*-rf\s*/|>\s*/dev/(sd|hd|xvd)|mkfs\.|dd\s+if=.*of=|:\(\)\{\s*:\|:\s*\};)'; then
    log_error "Command rejected by whitelist: $cmd"
    return 1
  fi
  return 0
}

# Transform command using terraphim-agent
transform_command() {
  local cmd="$1"
  # Try AI Engineer role for build command transformations
  if [ -f "$HOME/.cargo/bin/terraphim-agent" ]; then
    local transformed
    transformed=$(echo "$cmd" | "$HOME/.cargo/bin/terraphim-agent" replace \
      --role "AI Engineer" 2>/dev/null || echo "")
    if [ -n "$transformed" ] && [ "$transformed" != "$cmd" ]; then
      echo "$transformed"
      return 0
    fi
  fi
  echo "$cmd"
}

# Execute command with error handling
execute_command() {
  local cmd="$1"
  local step="$2"

  log_info "Step $step: $cmd"

  if ! validate_command "$cmd"; then
    POST_STATUS failure "build failed: command rejected at step $step"
    return 1
  fi

  # Transform via terraphim-agent
  local transformed
  transformed=$(transform_command "$cmd")
  if [ "$transformed" != "$cmd" ]; then
    log_info "  Transformed: $cmd → $transformed"
  fi

  # Track KG lookup cost
  track_kg_lookup

  # Execute
  if eval "$transformed"; then
    log_success "  Step $step complete"
    return 0
  else
    local exit_code=$?
    log_error "  Step $step failed with exit code $exit_code"
    POST_STATUS failure "build failed at step $step (exit $exit_code)"
    # Capture learning
    if [ -f "$HOME/.cargo/bin/terraphim-agent" ]; then
      "$HOME/.cargo/bin/terraphim-agent" learn capture "$transformed" \
        --error "exit code $exit_code" --exit-code "$exit_code" 2>/dev/null || true
    fi
    return 1
  fi
}

# Parse BUILD.md for build sequences
parse_build_md() {
  if [ ! -f "BUILD.md" ]; then
    return 1
  fi

  log_info "Parsing BUILD.md for build sequence"

  # Extract commands from code blocks under "Default Rust Build Sequence" or similar sections
  # Look for bash code blocks that contain build commands
  local in_block=0
  local block_content=""

  while IFS= read -r line; do
    # Start of bash code block
    if [[ "$line" =~ ^'```bash'$ ]]; then
      in_block=1
      block_content=""
      continue
    fi

    # End of code block
    if [[ "$line" =~ ^'```'$ ]] && [ "$in_block" -eq 1 ]; then
      in_block=0
      # Check if block contains build commands
      if echo "$block_content" | grep -qE '^(cargo|make|npm|yarn|pnpm|bun|docker|pytest|python|go|rustc)'; then
        echo "$block_content" | grep -v '^$'
        return 0
      fi
      continue
    fi

    # Collect block content
    if [ "$in_block" -eq 1 ]; then
      block_content="$block_content$line"
      # Check for multi-line content
      if [ ${#block_content} -gt 0 ]; then
        block_content="$block_content"
      fi
      echo "$line"
    fi
  done < BUILD.md | grep -v '^$' | grep -E '^(cargo|make|npm|yarn|pnpm|bun|docker|pytest|python|go|rustc)' | head -20

  return 0
}

# Extract build commands from GitHub Actions workflows
extract_workflow_commands() {
  local workflow_dir=".github/workflows"
  local all_commands=""

  if [ ! -d "$workflow_dir" ]; then
    return 1
  fi

  # Find workflows that trigger on push or pull_request
  for workflow in "$workflow_dir"/*.yml "$workflow_dir"/*.yaml; do
    [ -f "$workflow" ] || continue

    # Check if workflow triggers on push or pull_request
    if grep -qE 'on:\s*(push|pull_request|\[.*push.*\]|\[.*pull_request.*\])' "$workflow" 2>/dev/null; then
      log_info "Detected: GitHub Actions workflow ($workflow)"

      # Extract 'run' commands from steps
      if command -v yq >/dev/null 2>&1; then
        local commands
        commands=$(yq '.jobs[].steps[].run' "$workflow" 2>/dev/null | grep -v null | grep -v '^$')
        if [ -n "$commands" ]; then
          all_commands="$all_commands$commands"
        fi
      fi
    fi
  done

  if [ -n "$all_commands" ]; then
    echo "$all_commands"
    return 0
  fi

  return 1
}

# Detect CI configuration and extract commands
detect_and_extract() {
  cd "$ADF_WORKING_DIR"

  # Priority 1: GitHub Actions (all push/PR workflows)
  local workflow_commands
  workflow_commands=$(extract_workflow_commands)
  if [ -n "$workflow_commands" ]; then
    echo "$workflow_commands"
    return 0
  fi

  # Priority 2: BUILD.md (project-specific build documentation)
  if [ -f "BUILD.md" ]; then
    local build_commands
    build_commands=$(parse_build_md)
    if [ -n "$build_commands" ]; then
      log_info "Detected: BUILD.md with build sequence"
      echo "$build_commands"
      return 0
    fi
  fi

  # Priority 3: Cargo workspace (Rust projects)
  if [ -f "Cargo.toml" ]; then
    log_info "Detected: Cargo workspace"
    echo "cargo fmt --all -- --check"
    echo "cargo clippy --workspace --all-targets -- -D warnings"
    echo "cargo build --workspace"
    echo "cargo test --workspace --no-fail-fast"
    return 0
  fi

  # Priority 4: Makefile
  if [ -f "Makefile" ] && command -v make >/dev/null 2>&1; then
    log_info "Detected: Makefile"
    echo "make"
    return 0
  fi

  # Priority 5: Earthfile (Docker builds - extract only cargo/build commands)
  if [ -f "Earthfile" ]; then
    log_info "Detected: Earthfile"
    # Only extract RUN lines that contain cargo/build/test commands
    grep -E '^\s+RUN\s+' Earthfile | sed 's/^\s*RUN\s*//' | grep -E '(cargo|make|npm|yarn|bun|test|build)' | grep -v '^$'
    return 0
  fi

  # Priority 6: Package.json
  if [ -f "package.json" ]; then
    log_info "Detected: Node.js project"
    echo "bun install"
    echo "bun run build"
    echo "bun test"
    return 0
  fi

  log_error "No CI configuration detected"
  return 1
}

# Main build execution
main() {
  log_info "Starting build-runner-llm (Epic #1423)"
  log_info "Working directory: $ADF_WORKING_DIR"
  log_info "Role: DevOpsRunner (KG-first, LLM disabled)"

  POST_STATUS pending "build started"

  # Check prerequisites
  if [ -z "$ADF_WORKING_DIR" ]; then
    log_error "ADF_WORKING_DIR not set"
    exit 1
  fi

  if [ ! -d "$ADF_WORKING_DIR" ]; then
    log_error "Working directory does not exist: $ADF_WORKING_DIR"
    exit 1
  fi

  # Extract commands
  local commands
  commands=$(detect_and_extract)
  if [ -z "$commands" ]; then
    log_error "No build commands extracted"
    POST_STATUS failure "no build commands found"
    exit 1
  fi

  log_info "Extracted commands:"
  echo "$commands" | nl

  # Execute commands
  local step=1
  local failed=0

  while IFS= read -r cmd; do
    [ -z "$cmd" ] && continue
    [ "${cmd:0:1}" = "#" ] && continue

    if ! execute_command "$cmd" "$step"; then
      failed=1
      break
    fi
    step=$((step + 1))
  done <<< "$commands"

  # Check cost thresholds
  check_cost_thresholds

  if [ "$failed" -eq 0 ]; then
    POST_STATUS success "all build steps passed"
    log_success "Build completed successfully"
    send_cost_metrics "success"
    exit 0
  else
    # Status already posted by execute_command
    send_cost_metrics "failure"
    exit 1
  fi
}

# Run main
main "$@"
