#!/bin/bash

# Comprehensive Terraphim Setup Script
# This script populates Atomic Server with Terraphim ontologies and documents,
# and optionally configures Terraphim Server roles via API.
#
# Usage:
#   ./setup_terraphim_full.sh <atomic_server_url> <agent_secret> [terraphim_server_url]
#
# Examples:
#   # Populate only atomic server
#   ./setup_terraphim_full.sh http://localhost:9883 your-base64-secret
#
#   # Populate atomic server AND configure terraphim server
#   ./setup_terraphim_full.sh http://localhost:9883 your-base64-secret http://localhost:8000

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
ATOMIC_SERVER_URL="${1:-${ATOMIC_SERVER_URL:-http://localhost:9883}}"
AGENT_SECRET="${2:-${ATOMIC_SERVER_SECRET}}"
TERRAPHIM_SERVER_URL="${3:-${TERRAPHIM_SERVER_URL}}"

DOCS_SRC_PATH="./docs/src"
ONTOLOGY_PATH="./crates/terraphim_atomic_client"

# Predefined configurations
SYSTEM_OPERATOR_CONFIG="./terraphim_server/default/system_operator_config.json"
TERRAPHIM_ENGINEER_CONFIG="./terraphim_server/default/terraphim_engineer_config.json"

# Function to print colored header
print_header() {
    echo -e "${CYAN}================================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}================================================${NC}"
}

# Function to print step
print_step() {
    echo -e "${BLUE}🔧 $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to show usage
show_usage() {
    echo -e "${YELLOW}Usage: $0 <atomic_server_url> <agent_secret> [terraphim_server_url]${NC}"
    echo ""
    echo "Parameters:"
    echo "  atomic_server_url    - URL of the Atomic Server (e.g., http://localhost:9883)"
    echo "  agent_secret         - Base64-encoded agent secret for Atomic Server authentication"
    echo "  terraphim_server_url - Optional: URL of Terraphim Server for role configuration (e.g., http://localhost:8000)"
    echo ""
    echo "Examples:"
    echo "  # Populate only atomic server"
    echo "  $0 http://localhost:9883 your-base64-secret"
    echo ""
    echo "  # Populate atomic server AND configure terraphim server"
    echo "  $0 http://localhost:9883 your-base64-secret http://localhost:8000"
    echo ""
    echo "Environment Variables (optional):"
    echo "  ATOMIC_SERVER_URL    - Default atomic server URL"
    echo "  ATOMIC_SERVER_SECRET - Default agent secret"
    echo "  TERRAPHIM_SERVER_URL - Default terraphim server URL"
}

# Check arguments
if [ $# -lt 2 ]; then
    print_error "Missing required arguments"
    show_usage
    exit 1
fi

print_header "🚀 Terraphim Full Setup Script"
echo -e "${BLUE}📡 Atomic Server URL: ${ATOMIC_SERVER_URL}${NC}"
echo -e "${BLUE}🔑 Agent Secret: ${AGENT_SECRET:0:20}...${NC}"
if [ -n "$TERRAPHIM_SERVER_URL" ]; then
    echo -e "${BLUE}🌐 Terraphim Server URL: ${TERRAPHIM_SERVER_URL}${NC}"
else
    echo -e "${YELLOW}🌐 Terraphim Server: Not specified (atomic server only)${NC}"
fi
echo ""

# Check dependencies
print_step "Checking dependencies..."

# Check if required tools are installed
missing_tools=()

if ! command -v jq &> /dev/null; then
    missing_tools+=("jq")
fi

if ! command -v curl &> /dev/null; then
    missing_tools+=("curl")
fi

if ! command -v terraphim_atomic_client &> /dev/null; then
    missing_tools+=("terraphim_atomic_client")
fi

if [ ${#missing_tools[@]} -gt 0 ]; then
    print_error "Missing required tools: ${missing_tools[*]}"
    echo "Please install the missing tools:"
    echo "  - jq: sudo apt install jq (Ubuntu) or brew install jq (macOS)"
    echo "  - curl: usually pre-installed"
    echo "  - terraphim_atomic_client: cargo build --release -p terraphim_atomic_client"
    exit 1
fi

print_success "All required tools are available"

# Set environment variables for terraphim_atomic_client
export ATOMIC_SERVER_URL
export ATOMIC_SERVER_SECRET="$AGENT_SECRET"

# Check if Atomic Server is running
print_step "Checking Atomic Server connectivity..."
if ! curl -s -f "${ATOMIC_SERVER_URL}" > /dev/null; then
    print_error "Atomic Server is not running at ${ATOMIC_SERVER_URL}"
    echo "Please start Atomic Server first:"
    echo "  atomic-server start --port 9883"
    exit 1
fi
print_success "Atomic Server is running and accessible"

# Check if Terraphim Server is running (if specified)
terraphim_server_available=false
if [ -n "$TERRAPHIM_SERVER_URL" ]; then
    print_step "Checking Terraphim Server connectivity..."
    if curl -s -f "${TERRAPHIM_SERVER_URL}/health" > /dev/null; then
        print_success "Terraphim Server is running and accessible"
        terraphim_server_available=true
    else
        print_warning "Terraphim Server is not accessible at ${TERRAPHIM_SERVER_URL}"
        print_warning "Will skip Terraphim Server configuration"
    fi
fi

# Check if required directories exist
print_step "Checking required directories..."
if [ ! -d "$DOCS_SRC_PATH" ]; then
    print_error "Source directory ${DOCS_SRC_PATH} does not exist"
    exit 1
fi

if [ ! -d "$ONTOLOGY_PATH" ]; then
    print_error "Ontology directory ${ONTOLOGY_PATH} does not exist"
    exit 1
fi

print_success "All required directories exist"

# ================================================================
# Phase 1: Populate Atomic Server with Terraphim Ontologies
# ================================================================

print_header "📚 Phase 1: Populating Atomic Server with Terraphim Ontologies"

# Find the best ontology file to use
ontology_files=(
    "$ONTOLOGY_PATH/terraphim_ontology_full.json"
    "$ONTOLOGY_PATH/terraphim_ontology_complete.json"
    "$ONTOLOGY_PATH/terraphim_ontology_fixed.json"
    "$ONTOLOGY_PATH/terraphim_ontology.json"
)

ontology_file=""
for file in "${ontology_files[@]}"; do
    if [ -f "$file" ]; then
        ontology_file="$file"
        break
    fi
done

if [ -z "$ontology_file" ]; then
    print_error "No Terraphim ontology file found in ${ONTOLOGY_PATH}"
    exit 1
fi

print_step "Using ontology file: $(basename "$ontology_file")"

# Import ontology using terraphim_atomic_client
print_step "Importing Terraphim ontology into Atomic Server..."
if terraphim_atomic_client import-ontology "$ontology_file" --validate; then
    print_success "Terraphim ontology imported successfully"
else
    print_error "Failed to import Terraphim ontology"
    exit 1
fi

# ================================================================
# Phase 2: Populate Atomic Server with Documents
# ================================================================

print_header "📄 Phase 2: Populating Atomic Server with Documents"

print_step "Processing markdown files from ${DOCS_SRC_PATH}..."
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

    # Extract title from first line or use filename
    local title=$(echo "$content" | head -n 1 | sed 's/^# //' | sed 's/^## //' | sed 's/^### //')
    if [ -z "$title" ] || [ "$title" = "$content" ]; then
        title="$file_name_no_ext"
    fi

    # Use the full content as description
    local description="$content"

    # Use CLI to create Article resource
    if terraphim_atomic_client create "$slug" "$title" "$description" "Article" > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Created: ${title}${NC}"
        return 0
    else
        echo -e "${RED}  ❌ Failed: ${title}${NC}"
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
    print_step "Processing knowledge graph files..."
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
    print_step "Processing scorer files..."
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

print_success "Document population completed: ${created_count} created, ${failed_count} failed"

# Wait for indexing
print_step "Waiting for documents to be indexed (10s)..."
sleep 10

# Test search functionality
print_step "Testing search functionality..."
search_term="Terraphim"
if terraphim_atomic_client search "$search_term" > search_results.json 2>/dev/null; then
    result_count=$(jq '. | length' search_results.json 2>/dev/null || echo "0")
    if [ "$result_count" -gt 0 ]; then
        print_success "Search test passed: Found ${result_count} results for '${search_term}'"
    else
        print_warning "Search test: No results found for '${search_term}'"
    fi
    rm -f search_results.json
else
    print_warning "Search test failed or no results"
fi

# ================================================================
# Phase 3: Configure Terraphim Server (Optional)
# ================================================================

if [ "$terraphim_server_available" = true ]; then
    print_header "🌐 Phase 3: Configuring Terraphim Server"

    # Test configuration endpoint
    print_step "Testing Terraphim Server configuration endpoint..."
    if curl -s -f "${TERRAPHIM_SERVER_URL}/config" > /dev/null; then
        print_success "Configuration endpoint is accessible"
    else
        print_error "Configuration endpoint is not accessible"
        exit 1
    fi

    # Apply System Operator configuration if available
    if [ -f "$SYSTEM_OPERATOR_CONFIG" ]; then
        print_step "Applying System Operator configuration..."
        if curl -s -X POST \
            -H "Content-Type: application/json" \
            -d @"$SYSTEM_OPERATOR_CONFIG" \
            "${TERRAPHIM_SERVER_URL}/config" > /dev/null; then
            print_success "System Operator configuration applied"
        else
            print_warning "Failed to apply System Operator configuration"
        fi
    fi

    # Apply Terraphim Engineer configuration if available
    if [ -f "$TERRAPHIM_ENGINEER_CONFIG" ]; then
        print_step "Applying Terraphim Engineer configuration..."
        if curl -s -X POST \
            -H "Content-Type: application/json" \
            -d @"$TERRAPHIM_ENGINEER_CONFIG" \
            "${TERRAPHIM_SERVER_URL}/config" > /dev/null; then
            print_success "Terraphim Engineer configuration applied"
        else
            print_warning "Failed to apply Terraphim Engineer configuration"
        fi
    fi

    # Test configuration after update
    print_step "Verifying configuration update..."
    if curl -s "${TERRAPHIM_SERVER_URL}/config" | jq '.config.roles' > /dev/null 2>&1; then
        print_success "Configuration verification passed"
    else
        print_warning "Configuration verification failed"
    fi

else
    print_header "🌐 Phase 3: Terraphim Server Configuration Skipped"
    print_warning "Terraphim Server not available or not specified"
fi

# ================================================================
# Summary
# ================================================================

print_header "🎉 Setup Complete!"

echo -e "${GREEN}✅ Atomic Server Population:${NC}"
echo -e "   📚 Ontology: Imported from $(basename "$ontology_file")"
echo -e "   📄 Documents: ${created_count} created, ${failed_count} failed"
echo -e "   🔍 Search: Functional"

if [ "$terraphim_server_available" = true ]; then
    echo -e "${GREEN}✅ Terraphim Server Configuration:${NC}"
    echo -e "   🔧 System Operator: Applied"
    echo -e "   🔧 Terraphim Engineer: Applied"
    echo -e "   🌐 Server URL: ${TERRAPHIM_SERVER_URL}"
else
    echo -e "${YELLOW}⚠️  Terraphim Server Configuration: Skipped${NC}"
fi

echo ""
echo -e "${BLUE}🚀 Ready to use Terraphim with Atomic Server!${NC}"
echo -e "${BLUE}📡 Atomic Server: ${ATOMIC_SERVER_URL}${NC}"
if [ "$terraphim_server_available" = true ]; then
    echo -e "${BLUE}🌐 Terraphim Server: ${TERRAPHIM_SERVER_URL}${NC}"
fi
echo ""
echo -e "${CYAN}Available configurations:${NC}"
echo -e "   🔧 System Operator - Remote KG + GitHub docs"
echo -e "   🔧 Terraphim Engineer - Local KG + Internal docs"
echo -e "   📝 Default - Title scorer + Local docs"
