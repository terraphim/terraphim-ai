#!/usr/bin/env bash
# Publish @terraphim/autocomplete to npmjs.org
# Usage: ./scripts/publish-to-npmjs.sh [--dry-run] [--version VERSION] [--tag TAG]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PACKAGE_DIR="$PROJECT_ROOT/terraphim_ai_nodejs"

# Default values
DRY_RUN=false
VERSION=""
TAG="latest"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --tag)
            TAG="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [--dry-run] [--version VERSION] [--tag TAG]"
            echo ""
            echo "Options:"
            echo "  --dry-run     Run npm publish in dry-run mode"
            echo "  --version     Version to publish (e.g., 1.3.1)"
            echo "  --tag         npm tag (default: latest)"
            echo ""
            echo "Environment variables:"
            echo "  NPM_TOKEN     npm authentication token (optional if logged in)"
            echo ""
            echo "Examples:"
            echo "  $0 --dry-run                    # Test without publishing"
            echo "  $0 --version 1.3.1              # Publish version 1.3.1"
            echo "  $0 --version 1.4.0-beta --tag beta  # Publish beta version"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$PACKAGE_DIR"

echo "========================================"
echo "Publishing @terraphim/autocomplete to npmjs.org"
echo "========================================"
echo ""

# Check if npm is available
if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed"
    exit 1
fi

# Check npm authentication
echo "Checking npm authentication..."
if [[ -n "${NPM_TOKEN:-}" ]]; then
    echo "Using NPM_TOKEN environment variable"
    echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" > ~/.npmrc
elif npm whoami &> /dev/null; then
    echo "Using existing npm login: $(npm whoami)"
else
    echo "Error: Not authenticated with npm"
    echo ""
    echo "Please either:"
    echo "  1. Set NPM_TOKEN environment variable"
    echo "  2. Run 'npm login' to authenticate"
    echo ""
    echo "To get an npm token:"
    echo "  - Go to https://www.npmjs.com/settings/tokens"
    echo "  - Create a new 'Automation' token"
    echo "  - Export it: export NPM_TOKEN=npm_xxxxx"
    exit 1
fi

# Override publishConfig to use npmjs.org instead of GitHub Packages
echo ""
echo "Configuring npm for npmjs.org registry..."
npm config set @terraphim:registry https://registry.npmjs.org/

# Update version if specified
if [[ -n "$VERSION" ]]; then
    echo ""
    echo "Updating version to $VERSION..."
    npm version "$VERSION" --no-git-tag-version --allow-same-version
fi

# Show package info
echo ""
echo "Package information:"
echo "  Name: $(node -p "require('./package.json').name")"
echo "  Version: $(node -p "require('./package.json').version")"
echo "  Tag: $TAG"
echo "  Registry: https://registry.npmjs.org/"
echo ""

# Check for native binaries
if ls *.node &> /dev/null; then
    echo "Native binaries found:"
    ls -la *.node
    echo ""
else
    echo "Warning: No native binaries (.node files) found in package directory"
    echo "You may need to build them first with: yarn build"
    echo ""
fi

# Run npm pack to show what will be published
echo "Package contents (dry-run):"
npm pack --dry-run 2>&1 | head -30
echo ""

# Publish
if [[ "$DRY_RUN" == "true" ]]; then
    echo "========================================"
    echo "DRY RUN - Not actually publishing"
    echo "========================================"
    npm publish --dry-run --access public --tag "$TAG" --registry https://registry.npmjs.org/
else
    echo "========================================"
    echo "Publishing to npmjs.org..."
    echo "========================================"
    npm publish --access public --tag "$TAG" --registry https://registry.npmjs.org/

    echo ""
    echo "========================================"
    echo "Published successfully!"
    echo "========================================"
    echo ""
    echo "Install with:"
    echo "  npm install @terraphim/autocomplete@$TAG"
    echo "  bun add @terraphim/autocomplete@$TAG"
    echo ""
    echo "View package:"
    echo "  https://www.npmjs.com/package/@terraphim/autocomplete"
fi

echo ""
echo "Done!"
