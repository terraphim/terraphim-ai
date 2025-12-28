#!/bin/bash
#
# Generate Knowledge Graph from Note Titles
#
# Scans markdown files in a notes directory and generates KG entries
# from titles for semantic term expansion.
#
# Usage:
#   ./scripts/generate-notes-kg.sh [--source DIR] [--output DIR] [--filter PATTERN]
#
# Examples:
#   ./scripts/generate-notes-kg.sh
#   ./scripts/generate-notes-kg.sh --source ~/notes --output docs/src/kg/notes_kg
#   ./scripts/generate-notes-kg.sh --filter "*rust*"
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
SOURCE_DIR="/Users/alex/synced/expanded_docs"
OUTPUT_DIR="docs/src/kg/rust_notes_kg"
FILTER_PATTERN="*rust*.md"
MAX_ENTRIES=100

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --source|-s)
            SOURCE_DIR="$2"
            shift 2
            ;;
        --output|-o)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --filter|-f)
            FILTER_PATTERN="$2"
            shift 2
            ;;
        --max|-m)
            MAX_ENTRIES="$2"
            shift 2
            ;;
        --all|-a)
            FILTER_PATTERN="*.md"
            shift
            ;;
        --help|-h)
            cat << 'EOF'
Generate Knowledge Graph from Note Titles

Usage:
  ./scripts/generate-notes-kg.sh [OPTIONS]

Options:
  --source, -s DIR      Source directory containing markdown notes
                        (default: /Users/alex/synced/expanded_docs)
  --output, -o DIR      Output directory for KG files
                        (default: docs/src/kg/rust_notes_kg)
  --filter, -f PATTERN  Glob pattern to filter files
                        (default: *rust*.md)
  --all, -a             Process all markdown files (no filter)
  --max, -m N           Maximum number of KG entries to generate
                        (default: 100)
  --help, -h            Show this help message

Examples:
  # Generate Rust-specific KG
  ./scripts/generate-notes-kg.sh

  # Generate from all notes
  ./scripts/generate-notes-kg.sh --all --output docs/src/kg/all_notes_kg

  # Custom source directory
  ./scripts/generate-notes-kg.sh --source ~/my-notes --filter "*.md"
EOF
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Validate source directory
if [ ! -d "$SOURCE_DIR" ]; then
    echo -e "${RED}Error: Source directory not found: $SOURCE_DIR${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}Generating Knowledge Graph from Note Titles${NC}"
echo -e "Source: ${GREEN}$SOURCE_DIR${NC}"
echo -e "Filter: ${GREEN}$FILTER_PATTERN${NC}"
echo -e "Output: ${GREEN}$OUTPUT_DIR${NC}"
echo ""

# Function to normalize title to filename
normalize_title() {
    echo "$1" | \
        tr '[:upper:]' '[:lower:]' | \
        sed 's/[^a-z0-9]/_/g' | \
        sed 's/__*/_/g' | \
        sed 's/^_//;s/_$//'
}

# Function to extract key terms from title
extract_terms() {
    local title="$1"
    # Remove common words and extract meaningful terms
    echo "$title" | \
        tr '[:upper:]' '[:lower:]' | \
        sed 's/[^a-z0-9 ]/ /g' | \
        tr ' ' '\n' | \
        grep -v -E '^(a|an|the|and|or|but|in|on|at|to|for|of|with|by|from|is|are|was|were|be|been|being|have|has|had|do|does|did|will|would|could|should|may|might|must|shall|can|this|that|these|those|it|its|i|you|we|they|he|she|me|my|your|our|their|his|her)$' | \
        grep -E '^.{3,}$' | \
        sort -u | \
        head -5 | \
        tr '\n' ', ' | \
        sed 's/, $//'
}

# Count files
FILE_COUNT=$(find "$SOURCE_DIR" -maxdepth 1 -name "$FILTER_PATTERN" -type f 2>/dev/null | wc -l | tr -d ' ')
echo -e "Found ${GREEN}$FILE_COUNT${NC} matching files"
echo ""

# Process files
PROCESSED=0
GENERATED=0

for file in "$SOURCE_DIR"/$FILTER_PATTERN; do
    [ -f "$file" ] || continue

    ((PROCESSED++))

    # Extract title (first line starting with #)
    TITLE=$(grep -m1 '^# ' "$file" 2>/dev/null | sed 's/^# //')

    if [ -z "$TITLE" ]; then
        # Fallback to filename
        TITLE=$(basename "$file" .md | tr '-' ' ' | tr '_' ' ')
    fi

    # Skip if title is too short
    if [ ${#TITLE} -lt 5 ]; then
        continue
    fi

    # Generate normalized filename
    NORM_NAME=$(normalize_title "$TITLE")
    OUTPUT_FILE="$OUTPUT_DIR/${NORM_NAME}.md"

    # Skip if already exists
    if [ -f "$OUTPUT_FILE" ]; then
        continue
    fi

    # Extract key terms for synonyms
    TERMS=$(extract_terms "$TITLE")

    # Get source URL if present
    SOURCE_URL=$(grep -m1 '^\*\*Source URL\*\*:' "$file" 2>/dev/null | sed 's/^\*\*Source URL\*\*: //')

    # Generate KG entry
    cat > "$OUTPUT_FILE" << EOF
# $TITLE

Knowledge graph entry auto-generated from personal notes.

EOF

    if [ -n "$SOURCE_URL" ]; then
        echo "source:: $SOURCE_URL" >> "$OUTPUT_FILE"
    fi

    if [ -n "$TERMS" ]; then
        echo "synonyms:: $TERMS" >> "$OUTPUT_FILE"
    fi

    ((GENERATED++))

    # Progress indicator
    if [ $((GENERATED % 10)) -eq 0 ]; then
        echo -e "Generated ${GREEN}$GENERATED${NC} KG entries..."
    fi

    # Stop at max entries
    if [ $GENERATED -ge $MAX_ENTRIES ]; then
        echo -e "${YELLOW}Reached maximum entries ($MAX_ENTRIES)${NC}"
        break
    fi
done

echo ""
echo -e "${GREEN}Complete!${NC}"
echo -e "Processed: ${BLUE}$PROCESSED${NC} files"
echo -e "Generated: ${GREEN}$GENERATED${NC} KG entries"
echo -e "Output: ${GREEN}$OUTPUT_DIR${NC}"
echo ""

# List generated files
if [ $GENERATED -gt 0 ]; then
    echo -e "${BLUE}Sample generated entries:${NC}"
    ls -1 "$OUTPUT_DIR"/*.md 2>/dev/null | head -5
fi
