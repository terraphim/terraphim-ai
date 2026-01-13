#!/bin/bash
# Claude Log Analyzer - Automated Feature Demonstration
# Uses real sessions from ~/.claude/projects

set -e  # Exit on error

# Color codes for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Timing function
start_time=$(date +%s)
step_start_time=$start_time

print_header() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_step() {
    local current_time=$(date +%s)
    local elapsed=$((current_time - step_start_time))
    step_start_time=$current_time

    echo ""
    echo -e "${BLUE}─────────────────────────────────────────────────────${NC}"
    echo -e "${YELLOW}$1${NC}"
    if [ $elapsed -gt 0 ]; then
        echo -e "${GREEN}Previous step completed in ${elapsed}s${NC}"
    fi
    echo -e "${BLUE}─────────────────────────────────────────────────────${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_info() {
    echo -e "${CYAN}ℹ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Main demo
print_header "Claude Log Analyzer - Feature Demonstration"

# Check prerequisites
if [ ! -f "./target/release/cla" ]; then
    print_info "Building release binary..."
    cargo build --release --features terraphim
fi

# Create examples directory
mkdir -p examples
print_success "Created examples/ directory for outputs"

# Check if session directory exists
CLAUDE_DIR="${HOME}/.claude/projects"
if [ ! -d "$CLAUDE_DIR" ]; then
    print_error "Session directory not found: $CLAUDE_DIR"
    exit 1
fi

SESSION_COUNT=$(find "$CLAUDE_DIR" -type f -name "session.jsonl" | wc -l | tr -d ' ')
print_info "Found $SESSION_COUNT Claude sessions to analyze"
echo ""

# Part 1: Basic Tool Analysis
print_step "1. Basic Tool Analysis"
print_info "Analyzing tool usage patterns..."
./target/release/cla tools | head -50 | tee examples/output-basic.txt
echo ""

# Part 2: Tool Filtering
print_step "2. Tool-Specific Analysis"
print_info "Filtering by specific tool (e.g., 'Bash')..."
./target/release/cla tools --tool Bash | head -30 | tee examples/output-tool-filter.txt
echo ""

# Part 3: Export Formats
print_step "3. Export Formats (JSON, CSV, Markdown)"
print_info "Exporting tool analysis in multiple formats..."
./target/release/cla tools --format json -o examples/tools.json
print_success "Exported JSON: examples/tools.json"

./target/release/cla tools --format csv -o examples/tools.csv
print_success "Exported CSV: examples/tools.csv"

./target/release/cla tools --format markdown -o examples/tools.md
print_success "Exported Markdown: examples/tools.md"
echo ""

# Part 4: Agent Correlation
print_step "4. Agent-Tool Correlation Matrix"
print_info "Analyzing which agents use which tools..."
./target/release/cla tools --show-correlation | tee examples/output-correlation.txt
echo ""

# Part 5: Tool Chains
print_step "5. Tool Chain Detection"
print_info "Identifying common tool sequences and patterns..."
./target/release/cla tools --show-chains | head -100 | tee examples/output-chains.txt
echo ""

# Part 6: Knowledge Graph Search
print_step "6. Knowledge Graph Search"
print_info "Searching for deployment-related tool usage..."
./target/release/cla tools --kg-search "deploy OR build OR test" | head -100 | tee examples/output-kg-search.txt
echo ""

# Part 7: Sorting Options
print_step "7. Different Sorting Methods"
print_info "Sorting by most recent usage..."
./target/release/cla tools --sort-by recent --min-usage 5 | head -30 | tee examples/output-sort-recent.txt
echo ""

# Part 8: Self-Analysis
print_step "8. Self-Analysis (This Project's Development)"
PROJECT_DIR="${HOME}/.claude/projects/-Users-alex-projects-zestic-ai-claude-log-analyzer"
if [ -d "$PROJECT_DIR" ]; then
    print_info "Analyzing how this project was built..."
    ./target/release/cla tools "$PROJECT_DIR" --show-chains | head -50 | tee examples/output-self-analysis.txt
else
    print_info "Project session directory not found, skipping self-analysis"
fi
echo ""

# Part 9: Session Summary
print_step "9. Overall Session Summary"
print_info "Generating summary statistics..."
./target/release/cla summary | tee examples/output-summary.txt
echo ""

# Part 10: Timeline Visualization
print_step "10. Timeline Visualization"
print_info "Generating HTML timeline..."
./target/release/cla timeline --output examples/timeline.html
if [ -f "examples/timeline.html" ]; then
    print_success "Timeline saved to examples/timeline.html"
    print_info "Open examples/timeline.html in your browser to view interactive timeline"
fi
echo ""

# Calculate total time
end_time=$(date +%s)
total_time=$((end_time - start_time))

# Summary
print_header "Demo Complete!"
echo -e "${GREEN}Total execution time: ${total_time}s${NC}"
echo ""
echo -e "${CYAN}Outputs saved to examples/${NC}"
ls -lh examples/ | grep -v "^total" | awk '{print "  " $9 " (" $5 ")"}'
echo ""
echo -e "${YELLOW}Suggested next steps:${NC}"
echo "  1. View the correlation matrix: cat examples/output-correlation.txt"
echo "  2. Explore tool chains: cat examples/output-chains.txt"
echo "  3. Open HTML timeline: open examples/timeline.html"
echo "  4. Analyze JSON output: jq . examples/tools.json"
echo ""
print_success "All features demonstrated successfully!"
