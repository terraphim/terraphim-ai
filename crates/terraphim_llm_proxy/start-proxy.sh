#!/bin/bash
set -a
source .env
set +a
./target/release/terraphim-llm-proxy --config config.multi-provider.live.toml
