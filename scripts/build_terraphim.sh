#!/bin/bash

# Terraphim Build Script
#
# This script uses the terraphim_build_args library to manage build arguments
# and configurations for the Terraphim AI project.

set -e

# Determine the root directory of the repository
ROOT_DIR=$(git rev-parse --show-toplevel || echo "$PWD")
cd "$ROOT_DIR"

# Load build configuration based on environment
CONFIG_FILE="build_config.toml"

if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Warning: Build configuration file '$CONFIG_FILE' not found, using default build"
    cargo build --release
    echo "Build completed successfully!"
    exit 0
fi

# Check if terraphim-build-args exists
if [ -f "./target/release/terraphim-build-args" ]; then
    # Invoke the build argument manager (Rust tool)
    ./target/release/terraphim-build-args --config "$CONFIG_FILE" \
        --output "cargo"
else
    echo "Warning: terraphim-build-args not found, using default cargo build"
    cargo build --release
fi

echo "Build completed successfully!"
