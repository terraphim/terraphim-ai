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
    echo "Error: Build configuration file '$CONFIG_FILE' not found!"
    exit 1
fi

# Invoke the build argument manager (Rust tool)
./target/release/terraphim-build-args --config "$CONFIG_FILE" \
    --output "cargo"

echo "Build completed successfully!"
