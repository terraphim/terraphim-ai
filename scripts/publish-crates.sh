#!/usr/bin/env bash
set -euo pipefail

################################################################################
# publish-crates.sh
#
# Publish Rust crates to crates.io
#
# Usage:
#   ./scripts/publish-crates.sh [OPTIONS]
#
# Options:
#   -v, --version VERSION    Version to publish (e.g., 1.2.3)
#   -d, --dry-run           Dry run mode (validate only)
#   -c, --crate CRATE       Publish specific crate only
#   -t, --token TOKEN       crates.io API token
#   -h, --help              Show help message
#
# Examples:
#   # Publish all crates with version 1.2.3
#   ./scripts/publish-crates.sh -v 1.2.3
#
#   # Dry run for specific crate
#   ./scripts/publish-crates.sh -c terraphim_types -v 1.2.3 -d
#
#   # Use specific token
#   ./scripts/publish-crates.sh -v 1.2.3 -t $CARGO_REGISTRY_TOKEN
#
################################################################################

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DRY_RUN=false
VERSION=""
SPECIFIC_CRATE=""
TOKEN=""

# Crates in dependency order (must publish in this order)
CRATES=(
  "terraphim_types"
  "terraphim_settings"
  "terraphim_persistence"
  "terraphim_automata"
  "terraphim_config"
  "terraphim_rolegraph"
  "terraphim_hooks"
  "terraphim-session-analyzer"
  "terraphim_middleware"
  "terraphim_update"
  "terraphim_service"
  "terraphim_agent"
)

# Logging functions
log_info() {
  echo -e "${BLUE}INFO:${NC} $1"
}

log_success() {
  echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
  echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
  echo -e "${RED}✗${NC} $1"
}

# Help function
show_help() {
  sed -n '2,30p' "$0" | head -n -1 | sed 's/^# //'
  exit 0
}

# Parse command line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case $1 in
      -v|--version)
        VERSION="$2"
        shift 2
        ;;
      -d|--dry-run)
        DRY_RUN=true
        shift
        ;;
      -c|--crate)
        SPECIFIC_CRATE="$2"
        shift 2
        ;;
      -t|--token)
        TOKEN="$2"
        shift 2
        ;;
      -h|--help)
        show_help
        ;;
      *)
        log_error "Unknown option: $1"
        show_help
        ;;
    esac
  done
}

# Validate prerequisites
check_prerequisites() {
  log_info "Checking prerequisites..."

  # Check if cargo is available
  if ! command -v cargo &> /dev/null; then
    log_error "cargo not found. Please install Rust."
    exit 1
  fi

  # Check if jq is available
  if ! command -v jq &> /dev/null; then
    log_warning "jq not found. Installing jq is recommended for better output parsing."
  fi

  # Check version format
  if [[ -z "$VERSION" ]]; then
    log_error "Version is required. Use -v or --version option."
    exit 1
  fi

  if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    log_error "Invalid version format: $VERSION. Expected: X.Y.Z"
    exit 1
  fi

  # Check token - try 1Password first if available, then environment
  if [[ -z "$TOKEN" ]]; then
    # Try to get token from 1Password if op CLI is available
    if command -v op &> /dev/null; then
      TOKEN=$(op read "op://TerraphimPlatform/crates.io.token/token" 2>/dev/null || echo "")
      if [[ -n "$TOKEN" ]]; then
        export CARGO_REGISTRY_TOKEN="$TOKEN"
        log_info "Using crates.io token from 1Password"
      fi
    fi

    # If still no token, try environment variable
    if [[ -z "$TOKEN" ]] && [[ -n "${CARGO_REGISTRY_TOKEN:-}" ]]; then
      TOKEN="$CARGO_REGISTRY_TOKEN"
      log_info "Using crates.io token from environment"
    fi

    # If still no token, show warning
    if [[ -z "$TOKEN" ]]; then
      log_warning "No token provided. Will attempt to use existing cargo credentials."
    fi
  else
    export CARGO_REGISTRY_TOKEN="$TOKEN"
    log_info "Using provided token for authentication"
  fi

  log_success "Prerequisites validated"
}

# Update crate versions
update_versions() {
  log_info "Updating crate versions to $VERSION..."

  for crate in "${CRATES[@]}"; do
    local crate_path="crates/$crate/Cargo.toml"

    if [[ -f "$crate_path" ]]; then
      log_info "Updating $crate to version $VERSION"

      # Update version in Cargo.toml
      if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$crate_path"
      else
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$crate_path"
      fi

      # Update workspace dependencies
      find crates -name "Cargo.toml" -type f -exec sed -i.bak "s/$crate = { path = \"\.$crate\", version = \"[0-9.]\"\+ }/$crate = { path = \"\.$crate\", version = \"$VERSION\" }/g" {} \; 2>/dev/null || true
      find crates -name "*.bak" -delete 2>/dev/null || true
    else
      log_warning "Crate $crate not found at $crate_path"
    fi
  done

  log_success "Versions updated"
}

# Check if crate is already published
check_if_published() {
  local crate="$1"
  local version="$2"

  log_info "Checking if $crate v$version is already published..."

  # Use crates.io API directly for reliable version detection
  local response
  response=$(curl -s "https://crates.io/api/v1/crates/$crate/versions" 2>/dev/null || echo "")

  if echo "$response" | grep -q "\"num\":\"$version\""; then
    log_warning "$crate v$version already exists on crates.io"
    return 0
  else
    log_info "$crate v$version not published yet"
    return 1
  fi
}

# Publish a single crate
publish_crate() {
  local crate="$1"
  local version="$2"

  log_info "Publishing $crate v$version..."

  if check_if_published "$crate" "$version"; then
    log_warning "Skipping $crate (already published)"
    return 0
  fi

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: cargo publish --package $crate --dry-run --allow-dirty"
    cargo publish --package "$crate" --dry-run --allow-dirty
  else
    log_info "Running: cargo publish --package $crate --allow-dirty"

    local output
    if output=$(cargo publish --package "$crate" --allow-dirty 2>&1); then
      log_success "Published $crate v$version successfully"
      log_info "Waiting 60 seconds for crates.io to process..."
      sleep 60
    else
      # Check if it failed because the crate already exists
      if echo "$output" | grep -q "already exists on"; then
        log_warning "$crate v$version already exists - skipping"
        return 0
      else
        log_error "Failed to publish $crate"
        echo "$output"
        return 1
      fi
    fi
  fi
}

# Get current version of a crate
get_current_version() {
  local crate="$1"
  cargo metadata --format-version 1 --no-deps |
    jq -r ".packages[] | select(.name == \"$crate\") | .version" 2>/dev/null ||
    grep -A 5 "name = \"$crate\"" "crates/$crate/Cargo.toml" |
    grep "^version" | head -1 | cut -d'"' -f2
}

# Main publishing function
main() {
  local -a crates_to_publish

  if [[ -n "$SPECIFIC_CRATE" ]]; then
    # Publish specific crate and all its dependencies (crates that come before it in the list)
    log_info "Publishing specific crate: $SPECIFIC_CRATE and its dependencies"

    local found=false
    for crate in "${CRATES[@]}"; do
      crates_to_publish+=("$crate")
      if [[ "$crate" == "$SPECIFIC_CRATE" ]]; then
        found=true
        break
      fi
    done

    if [[ "$found" != "true" ]]; then
      log_error "Crate $SPECIFIC_CRATE not found in dependency chain"
      exit 1
    fi
  else
    # Publish all crates
    crates_to_publish=("${CRATES[@]}")
  fi

  # Update versions if needed
  if [[ -n "$VERSION" ]]; then
    update_versions
  fi

  # Publish crates
  for crate in "${crates_to_publish[@]}"; do
    if [[ ! -f "crates/$crate/Cargo.toml" ]]; then
      log_warning "Crate $crate not found, skipping"
      continue
    fi

    local current_version
    current_version=$(get_current_version "$crate")

    if [[ -z "$current_version" ]]; then
      log_error "Could not determine version for $crate"
      exit 1
    fi

    publish_crate "$crate" "$current_version" || {
      log_error "Publishing failed at $crate"
      exit 1
    }
  done

  log_success "All crates processed successfully!"

  # Summary
  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run completed - no packages were actually published"
  else
    log_success "Publishing completed successfully!"
  fi
}

# Parse arguments and run
parse_args "$@"
check_prerequisites
main "$@"
