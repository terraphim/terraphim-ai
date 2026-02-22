#!/bin/bash

# Retrieve API keys from 1Password
export OPENROUTER_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/openrouter-api-key")
export ANTHROPIC_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/anthropic-api-key")
export CEREBRAS_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/cerebras-api-key")
export GROQ_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/groq-api-key")

# Additional keys needed for config
export DEEPSEEK_API_KEY="REDACTED_DEEPSEEK_KEY"  # Correct DeepSeek API key
export OPENAI_API_KEY="dummy-key-for-now"    # Not using OpenAI directly

echo "Starting Terraphim LLM Proxy with real API keys..."
echo "OpenRouter API Key: ${OPENROUTER_API_KEY:0:10}..."
echo "Anthropic API Key: ${ANTHROPIC_API_KEY:0:10}..."
echo "Cerebras API Key: ${CEREBRAS_API_KEY:0:10}..."
echo "Groq API Key: ${GROQ_API_KEY:0:10}..."

RUST_LOG=debug cargo run --release