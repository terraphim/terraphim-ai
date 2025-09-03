#!/bin/bash

# Script to populate Atomic Server with markdown documents from docs/src using the terraphim_atomic_client CLI
# This script creates test documents in Atomic Server for testing the new roles

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ATOMIC_SERVER_URL="${ATOMIC_SERVER_URL:-http://localhost:9883}"
DOCS_SRC_PATH="./docs/src"
COLLECTION_NAME="terraphim-test-docs"
CLASS="Article"

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo -e "${RED}❌ jq is required but not installed. Please install jq.${NC}"
    exit 1
fi

# Check if terraphim_atomic_client is installed
if ! command -v terraphim_atomic_client &> /dev/null; then
    echo -e "${RED}❌ terraphim_atomic_client CLI is required but not installed. Please build it first.${NC}"
    exit 1
fi

# Load environment from .env file if it exists
if [ -f "crates/terraphim_atomic_client/.env" ]; then
    source crates/terraphim_atomic_client/.env
fi

export ATOMIC_SERVER_URL
export ATOMIC_SERVER_SECRET

echo -e "${BLUE}🚀 Populating Atomic Server with Terraphim documentation using CLI...${NC}"
echo -e "${BLUE}📡 Atomic Server URL: ${ATOMIC_SERVER_URL}${NC}"
echo -e "${BLUE}📁 Source directory: ${DOCS_SRC_PATH}${NC}"

# Check if Atomic Server is running
echo -e "${BLUE}🔍 Checking if Atomic Server is running...${NC}"
if ! curl -s "${ATOMIC_SERVER_URL}" > /dev/null; then
    echo -e "${RED}❌ Atomic Server is not running at ${ATOMIC_SERVER_URL}${NC}"
    echo -e "${YELLOW}💡 Please start Atomic Server first:${NC}"
    echo -e "${YELLOW}   atomic-server start${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Atomic Server is running${NC}"

# Check if docs/src directory exists
if [ ! -d "$DOCS_SRC_PATH" ]; then
    echo -e "${RED}❌ Source directory ${DOCS_SRC_PATH} does not exist${NC}"
    exit 1
fi

echo -e "${BLUE}📄 Processing markdown files...${NC}"
created_count=0
failed_count=0

# Function to create a document in Atomic Server using CLI
create_document() {
    local file_path="$1"
    local file_name=$(basename "$file_path")
    local file_name_no_ext="${file_name%.*}"
    # Convert to lowercase and replace special characters for valid slug
    local slug=$(echo "$file_name_no_ext" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g' | sed 's/^-\|-$//g')
    # Read file content
    local content=$(cat "$file_path")
    local title=$(echo "$content" | head -n 1 | sed 's/^# //' | sed 's/^## //' | sed 's/^### //')
    # If no title found in first line, use filename
    if [ -z "$title" ] || [ "$title" = "$content" ]; then
        title="$file_name_no_ext"
    fi
    # Use the full content as description
    local description="$content"
    # Use CLI to create resource
    if terraphim_atomic_client create "$slug" "$title" "$description" "$CLASS" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Created document: ${title}${NC}"
        return 0
    else
        echo -e "${RED}❌ Failed to create document: ${title}${NC}"
        return 1
    fi
}

# Process files in docs/src
for file in "$DOCS_SRC_PATH"/*.md; do
    if [ -f "$file" ]; then
        if create_document "$file"; then
            ((created_count++))
        else
            ((failed_count++))
        fi
    fi
done

# Process files in docs/src/kg
if [ -d "$DOCS_SRC_PATH/kg" ]; then
    for file in "$DOCS_SRC_PATH/kg"/*.md; do
        if [ -f "$file" ]; then
            if create_document "$file"; then
                ((created_count++))
            else
                ((failed_count++))
            fi
        fi
    done
fi

# Process files in docs/src/scorers
if [ -d "$DOCS_SRC_PATH/scorers" ]; then
    for file in "$DOCS_SRC_PATH/scorers"/*.md; do
        if [ -f "$file" ]; then
            if create_document "$file"; then
                ((created_count++))
            else
                ((failed_count++))
            fi
        fi
    done
fi

echo -e "${BLUE}📊 Summary:${NC}"
echo -e "${GREEN}✅ Successfully created: ${created_count} documents${NC}"
if [ $failed_count -gt 0 ]; then
    echo -e "${RED}❌ Failed to create: ${failed_count} documents${NC}"
fi

# Wait for indexing (longer)
echo -e "${BLUE}⏳ Waiting for documents to be indexed (10s)...${NC}"
sleep 10

# Test search functionality using CLI (look for a known term)
echo -e "${BLUE}🔍 Testing search functionality using CLI...${NC}"
search_term="Terraphim"
if terraphim_atomic_client search "$search_term" > search_results.json 2>/dev/null; then
    result_count=$(jq '. | length' search_results.json 2>/dev/null || echo "0")
    if [ "$result_count" -gt 0 ]; then
        echo -e "${GREEN}    Found ${result_count} results for '${search_term}'${NC}"
    else
        echo -e "${YELLOW}    No results found for '${search_term}'.${NC}"
        echo -e "${YELLOW}    Possible causes: indexing delay, search endpoint not indexing description, or document creation issue.${NC}"
    fi
    rm -f search_results.json
else
    echo -e "${RED}    Search failed or no results${NC}"
fi

echo -e "${GREEN}🎉 Atomic Server population completed!${NC}"
echo -e "${BLUE}🔧 You can now test the new roles with these configurations:${NC}"
echo -e "${BLUE}   - atomic_title_scorer_config.json${NC}"
echo -e "${BLUE}   - atomic_graph_embeddings_config.json${NC}"
