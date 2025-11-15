#!/bin/bash
# run_all_tests.sh - Master test runner for all Terraphim components

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create results directory
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="test_results_$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}   Terraphim AI Complete Functional Test Suite${NC}"
echo -e "${BLUE}================================================${NC}"
echo "Started at: $(date)"
echo "Results will be saved to: $RESULTS_DIR"
echo ""

# Check if binaries exist
echo -e "${YELLOW}Checking binaries...${NC}"
if [ ! -f "./target/release/terraphim-agent" ]; then
    echo -e "${RED}Error: TUI binary not found. Please build first.${NC}"
    exit 1
fi
if [ ! -f "./target/release/terraphim_server" ]; then
    echo -e "${RED}Error: Server binary not found. Please build first.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Binaries found${NC}"
echo ""

# Make test scripts executable
chmod +x tests/functional/*.sh

# Track overall results
TOTAL_PASS=0
TOTAL_FAIL=0

# Function to run a test and collect results
run_test() {
    local test_name="$1"
    local test_script="$2"
    local log_file="$RESULTS_DIR/${test_name}.log"

    echo -e "${BLUE}Running $test_name tests...${NC}"

    if $test_script 2>&1 | tee "$log_file"; then
        echo -e "${GREEN}✓ $test_name tests completed successfully${NC}"
        # Extract pass/fail counts from log
        passes=$(grep -c "✓ PASS" "$log_file" || echo 0)
        fails=$(grep -c "✗ FAIL" "$log_file" || echo 0)
        TOTAL_PASS=$((TOTAL_PASS + passes))
        TOTAL_FAIL=$((TOTAL_FAIL + fails))
    else
        echo -e "${RED}✗ $test_name tests failed${NC}"
        fails=$(grep -c "✗ FAIL" "$log_file" || echo 1)
        TOTAL_FAIL=$((TOTAL_FAIL + fails))
    fi
    echo ""
}

# 1. Test TUI REPL
run_test "TUI_REPL" "tests/functional/test_tui_repl.sh"

# 2. Test Server API
run_test "Server_API" "tests/functional/test_server_api.sh"

# 3. Desktop App tests would go here (requires UI automation)
# run_test "Desktop_App" "tests/functional/test_desktop_app.sh"

# Generate combined summary
SUMMARY_FILE="$RESULTS_DIR/summary.txt"
{
    echo "==================================="
    echo "    COMPLETE TEST SUITE SUMMARY"
    echo "==================================="
    echo "Timestamp: $TIMESTAMP"
    echo "Total Tests Run: $((TOTAL_PASS + TOTAL_FAIL))"
    echo "Passed: $TOTAL_PASS"
    echo "Failed: $TOTAL_FAIL"
    if [ $TOTAL_FAIL -eq 0 ]; then
        echo "Pass Rate: 100%"
        echo "Status: ALL TESTS PASSED ✅"
    else
        echo "Pass Rate: $(( TOTAL_PASS * 100 / (TOTAL_PASS + TOTAL_FAIL) ))%"
        echo "Status: SOME TESTS FAILED ❌"
    fi
    echo ""
    echo "Detailed Results:"
    echo "-----------------"
    for log in "$RESULTS_DIR"/*.log; do
        if [ -f "$log" ]; then
            basename=$(basename "$log" .log)
            echo ""
            echo "$basename:"
            grep "Test Summary" -A 5 "$log" | tail -n 4
        fi
    done
} | tee "$SUMMARY_FILE"

echo ""
echo -e "${BLUE}==================================${NC}"
echo -e "${BLUE}         TEST SUITE COMPLETE${NC}"
echo -e "${BLUE}==================================${NC}"
echo "Completed at: $(date)"
echo "Full results saved to: $RESULTS_DIR"
echo "Summary available at: $SUMMARY_FILE"

# Generate HTML report
HTML_REPORT="$RESULTS_DIR/report.html"
cat > "$HTML_REPORT" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Terraphim AI Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .pass { color: green; font-weight: bold; }
        .fail { color: red; font-weight: bold; }
        .header { background-color: #f0f0f0; padding: 10px; }
        .summary { background-color: #e0e0e0; padding: 15px; margin: 20px 0; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4CAF50; color: white; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Terraphim AI Functional Test Report</h1>
EOF

echo "<p>Generated: $(date)</p>" >> "$HTML_REPORT"
echo "<p>Version: v1.0.1</p>" >> "$HTML_REPORT"
echo "</div>" >> "$HTML_REPORT"
echo "<div class='summary'>" >> "$HTML_REPORT"
echo "<h2>Summary</h2>" >> "$HTML_REPORT"
echo "<p>Total Tests: $((TOTAL_PASS + TOTAL_FAIL))</p>" >> "$HTML_REPORT"
echo "<p class='pass'>Passed: $TOTAL_PASS</p>" >> "$HTML_REPORT"
echo "<p class='fail'>Failed: $TOTAL_FAIL</p>" >> "$HTML_REPORT"
echo "<p>Pass Rate: $(( TOTAL_PASS * 100 / (TOTAL_PASS + TOTAL_FAIL) ))%</p>" >> "$HTML_REPORT"
echo "</div>" >> "$HTML_REPORT"
echo "</body></html>" >> "$HTML_REPORT"

echo "HTML report generated: $HTML_REPORT"

# Exit with appropriate status
if [ $TOTAL_FAIL -eq 0 ]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED!${NC}"
    exit 1
fi
