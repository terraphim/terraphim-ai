#!/bin/bash
# Ultimate Bug Scanner - Claude Code Hook
# Runs on every file save for UBS-supported languages (JS/TS, Python, C/C++, Rust, Go, Java, Ruby)

if [[ "$FILE_PATH" =~ \.(js|jsx|ts|tsx|mjs|cjs|py|pyw|pyi|c|cc|cpp|cxx|h|hh|hpp|hxx|rs|go|java|rb)$ ]]; then
  echo "🔬 Running bug scanner..."
  if ! command -v ubs >/dev/null 2>&1; then
    echo "⚠️  'ubs' not found in PATH; install it before using this hook." >&2
    exit 0
  fi
  ubs "${PROJECT_DIR}" --ci 2>&1 | head -50
fi
