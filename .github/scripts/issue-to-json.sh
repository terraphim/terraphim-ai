#!/bin/bash
# Convert markdown issue file to JSON for Gitea API
# Usage: ./issue-to-json.sh path/to/issue.md

set -e

ISSUE_FILE="$1"

if [ -z "$ISSUE_FILE" ]; then
    echo "Usage: $0 <path/to/issue.md>"
    exit 1
fi

if [ ! -f "$ISSUE_FILE" ]; then
    echo "Error: File not found: $ISSUE_FILE"
    exit 1
fi

# Extract YAML frontmatter
frontmatter=$(sed -n '/^---$/,/^---$/p' "$ISSUE_FILE" | sed '1d;$d')

# Extract title
title=$(echo "$frontmatter" | grep "^title:" | sed 's/title: *//; s/^"//; s/"$//')

# Extract labels (handle array format)
labels_raw=$(echo "$frontmatter" | grep "^labels:" | sed 's/labels: *//')
labels=$(echo "$labels_raw" | sed 's/\[//; s/\]//; s/, */,/g; s/"//g')

# Extract body (content after second ---)
body=$(sed '1,/^---$/d' "$ISSUE_FILE" | sed '1{/^---$/d}')

# Escape body for JSON
body_escaped=$(echo "$body" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()), end="")' 2>/dev/null || echo "$body" | sed 's/\\/\\\\/g; s/"/\\"/g; s/$/\\n/; $s/\\n$//')

# Build JSON
cat << EOF
{
  "title": "$title",
  "body": $body_escaped,
  "labels": [$(echo "$labels" | awk -F',' '{for(i=1;i<=NF;i++) printf "\"%s\"%s", $i, (i<NF?", ":"")}')]
}
EOF
