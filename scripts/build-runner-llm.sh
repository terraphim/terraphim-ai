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

set -e

export GITEA_URL=https://git.terraphim.cloud
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/bin:$HOME/.bun/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

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
log_info() { echo "[INFO] $1"; }
log_success() { echo "[SUCCESS] $1"; }
log_warning() { echo "[WARNING] $1"; }
log_error() { echo "[ERROR] $1"; }

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
  # Try DevOpsRunner role for build command transformations
  if [ -f "$HOME/.cargo/bin/terraphim-agent" ]; then
    local transformed
    transformed=$(echo "$cmd" | "$HOME/.cargo/bin/terraphim-agent" replace \
      --role "DevOpsRunner" 2>/dev/null || echo "")
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

# Detect CI configuration and extract commands
detect_and_extract() {
  cd "$ADF_WORKING_DIR"

  # Priority 1: GitHub Actions
  if [ -f ".github/workflows/ci-pr.yml" ] && command -v yq >/dev/null 2>&1; then
    log_info "Detected: GitHub Actions workflow (.github/workflows/ci-pr.yml)"
    yq '.jobs[].steps[].run' .github/workflows/ci-pr.yml | grep -v null | grep -v '^$'
    return 0
  fi

  # Priority 2: Earthfile
  if [ -f "Earthfile" ]; then
    log_info "Detected: Earthfile"
    grep -E '^\s+(RUN|BUILD|COPY|ARG)\s+' Earthfile | sed 's/^\s*RUN\s*//' | sed 's/^\s*BUILD\s*//' | grep -v '^$'
    return 0
  fi

  # Priority 3: Makefile
  if [ -f "Makefile" ] && command -v make >/dev/null 2>&1; then
    log_info "Detected: Makefile"
    echo "make"
    return 0
  fi

  # Priority 4: Cargo workspace
  if [ -f "Cargo.toml" ]; then
    log_info "Detected: Cargo workspace"
    echo "cargo fmt --all -- --check"
    echo "cargo clippy --workspace -- -D warnings"
    echo "cargo build --workspace"
    echo "cargo test --workspace"
    return 0
  fi

  # Priority 5: Package.json
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

  if [ "$failed" -eq 0 ]; then
    POST_STATUS success "all build steps passed"
    log_success "Build completed successfully"
    exit 0
  else
    # Status already posted by execute_command
    exit 1
  fi
}

# Run main
main "$@"
