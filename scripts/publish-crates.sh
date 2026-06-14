#!/usr/bin/env bash
set -euo pipefail

################################################################################
# publish-crates.sh
#
# Publish monorepo-resident Rust crates to crates.io.
# Extracted polyrepo crates are published via polyrepo-publish.sh + crate_list.
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
################################################################################

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

DRY_RUN=false
VERSION=""
SPECIFIC_CRATE=""
TOKEN=""

# Monorepo-resident crates only (post-#1910 polyrepo extraction).
CRATES=(
  "terraphim_update"
  "terraphim_github_runner"
  "terraphim_gitea_runner"
  "terraphim_rlm"
)

declare -A CRATE_DIR_MAP=(
  ["terraphim_gitea_runner"]="terraphim_gitea_runner"
)

get_crate_dir() {
  local crate="$1"
  echo "${CRATE_DIR_MAP[$crate]:-$crate}"
}

log_info() { echo -e "${BLUE}INFO:${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

show_help() {
  sed -n '2,22p' "$0" | head -n -1 | sed 's/^# //'
  exit 0
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case $1 in
      -v|--version) VERSION="$2"; shift 2 ;;
      -d|--dry-run) DRY_RUN=true; shift ;;
      -c|--crate) SPECIFIC_CRATE="$2"; shift 2 ;;
      -t|--token) TOKEN="$2"; shift 2 ;;
      -h|--help) show_help ;;
      *) log_error "Unknown option: $1"; show_help ;;
    esac
  done
}

check_prerequisites() {
  log_info "Checking prerequisites..."

  if ! command -v cargo &> /dev/null; then
    log_error "cargo not found. Please install Rust."
    exit 1
  fi

  if [[ -z "$VERSION" ]]; then
    log_error "Version is required. Use -v or --version option."
    exit 1
  fi

  if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    log_error "Invalid version format: $VERSION. Expected: X.Y.Z"
    exit 1
  fi

  if [[ -z "$TOKEN" ]]; then
    if command -v op &> /dev/null; then
      TOKEN=$(op read "op://TerraphimPlatform/crates.io.token/token" 2>/dev/null || echo "")
      if [[ -n "$TOKEN" ]]; then
        export CARGO_REGISTRY_TOKEN="$TOKEN"
        log_info "Using crates.io token from 1Password"
      fi
    fi

    if [[ -z "$TOKEN" ]] && [[ -n "${CARGO_REGISTRY_TOKEN:-}" ]]; then
      TOKEN="$CARGO_REGISTRY_TOKEN"
      log_info "Using crates.io token from environment"
    fi

    if [[ -z "$TOKEN" ]]; then
      log_warning "No token provided. Will attempt to use existing cargo credentials."
    fi
  else
    export CARGO_REGISTRY_TOKEN="$TOKEN"
    log_info "Using provided token for authentication"
  fi

  log_success "Prerequisites validated"
}

update_versions() {
  log_info "Updating crate versions to $VERSION..."

  for crate in "${CRATES[@]}"; do
    local crate_dir
    crate_dir=$(get_crate_dir "$crate")
    local crate_path="crates/$crate_dir/Cargo.toml"

    if [[ -f "$crate_path" ]]; then
      log_info "Updating $crate to version $VERSION"
      if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' '1,/^version = ".*"/s/^version = ".*"/version = "'"$VERSION"'"/' "$crate_path"
      else
        sed -i '0,/^version = ".*"/s/^version = ".*"/version = "'"$VERSION"'"/' "$crate_path"
      fi
    else
      log_warning "Crate $crate not found at $crate_path"
    fi
  done

  log_success "Versions updated"
}

check_if_published() {
  local crate="$1"
  local version="$2"

  log_info "Checking if $crate v$version is already published..."

  local response
  response=$(curl -s "https://crates.io/api/v1/crates/$crate/versions" 2>/dev/null || echo "")

  if echo "$response" | grep -q "\"num\":\"$version\""; then
    log_warning "$crate v$version already exists on crates.io"
    return 0
  fi

  log_info "$crate v$version not published yet"
  return 1
}

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
      if echo "$output" | grep -q "already exists on"; then
        log_warning "$crate v$version already exists - skipping"
        return 0
      elif echo "$output" | grep -q "cannot be published"; then
        log_warning "$crate has publish = false in Cargo.toml - skipping"
        return 0
      else
        log_error "Failed to publish $crate"
        echo "$output"
        return 1
      fi
    fi
  fi
}

get_current_version() {
  local crate="$1"
  cargo metadata --format-version 1 --no-deps |
    jq -r ".packages[] | select(.name == \"$crate\") | .version" 2>/dev/null
}

main() {
  local -a crates_to_publish

  if [[ -n "$SPECIFIC_CRATE" ]]; then
    log_info "Publishing specific crate: $SPECIFIC_CRATE"
    local found=false
    for crate in "${CRATES[@]}"; do
      if [[ "$crate" == "$SPECIFIC_CRATE" ]]; then
        crates_to_publish+=("$crate")
        found=true
        break
      fi
    done

    if [[ "$found" != "true" ]]; then
      log_error "Crate $SPECIFIC_CRATE not found in monorepo publish list"
      exit 1
    fi
  else
    crates_to_publish=("${CRATES[@]}")
  fi

  if [[ -n "$VERSION" ]]; then
    update_versions
  fi

  for crate in "${crates_to_publish[@]}"; do
    local crate_dir
    crate_dir=$(get_crate_dir "$crate")
    if [[ ! -f "crates/$crate_dir/Cargo.toml" ]]; then
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

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run completed - no packages were actually published"
  else
    log_success "Publishing completed successfully!"
  fi
}

parse_args "$@"
check_prerequisites
main "$@"