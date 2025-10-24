---
name: search
description: Search for files and content using ripgrep
usage: "search <pattern> [path] [--type <type>] [--case-sensitive]"
category: File Operations
version: "1.0.0"
risk_level: Low
execution_mode: Local
permissions:
  - read
aliases:
  - find
  - grep
parameters:
  - name: pattern
    type: string
    required: true
    description: Search pattern (supports regex)
  - name: path
    type: string
    required: false
    default_value: "."
    description: Directory to search in
  - name: type
    type: string
    required: false
    description: File type filter (rs, py, js, md, etc.)
  - name: case_sensitive
    type: boolean
    required: false
    default_value: false
    description: Case-sensitive search
timeout: 30
---

# Search Command

Searches for files and content using ripgrep with pattern matching and filtering.

## Examples

```bash
# Basic search
search "TODO"

# Search in specific directory
search "async" src/

# Case-sensitive search
search "Error" --case-sensitive

# File type filter
search "test" --type rs
```

## Parameters

- **pattern**: Required. The search pattern (supports regular expressions)
- **path**: Optional. Directory to search in (default: current directory)
- **type**: Optional. File extension to filter by (e.g., rs, py, js)
- **case_sensitive**: Optional. Enable case-sensitive matching (default: false)

## Notes

This command uses ripgrep for fast searching and supports all ripgrep features including:
- Regular expression patterns
- File type filtering
- Binary file exclusion
- Line numbers and context