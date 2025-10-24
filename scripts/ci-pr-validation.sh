#!/bin/bash

# CI PR Validation Script
# Full PR validation with detailed reporting
# Usage: ./scripts/ci-pr-validation.sh [pr-number]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

PR_NUMBER="${1:-}"
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
GENERATE_REPORT="${GENERATE_REPORT:-true}"

echo -e "${BLUE}🔍 CI PR Validation${NC}"
echo "==================="
echo "Full PR validation with detailed reporting"
if [[ -n "$PR_NUMBER" ]]; then
    echo "PR Number: $PR_NUMBER"
fi
echo "Target: $TARGET"
echo "Generate Report: $GENERATE_REPORT"
echo ""

# Test results tracking
SCRIPTS_PASSED=0
SCRIPTS_FAILED=0
FAILED_SCRIPTS=()
TIMINGS=()
START_TIME=$(date +%s)

# Report file
REPORT_FILE="/tmp/ci-pr-validation-report-$(date +%Y%m%d-%H%M%S).md"

# Function to run script and track results with detailed reporting
run_script_with_report() {
    local script_name="$1"
    local script_path="$2"
    local description="$3"
    local category="$4"

    echo -e "\n${BLUE}🔄 Running: ${script_name}${NC}"
    echo "Category: $category"
    echo "Description: $description"
    echo "Script: $script_path"
    echo "Start time: $(date)"

    local script_start_time=$(date +%s)
    local script_log_file="/tmp/${script_name// /_}-${script_start_time}.log"

    # Run script and capture output
    if bash "$script_path" > "$script_log_file" 2>&1; then
        local script_end_time=$(date +%s)
        local script_duration=$((script_end_time - script_start_time))
        echo -e "${GREEN}  ✅ PASSED (${script_duration}s)${NC}"
        SCRIPTS_PASSED=$((SCRIPTS_PASSED + 1))
        TIMINGS+=("$script_name:$script_duration")
        return 0
    else
        local script_end_time=$(date +%s)
        local script_duration=$((script_end_time - script_start_time))
        echo -e "${RED}  ❌ FAILED (${script_duration}s)${NC}"
        echo -e "${YELLOW}  Log file: $script_log_file${NC}"
        SCRIPTS_FAILED=$((SCRIPTS_FAILED + 1))
        FAILED_SCRIPTS+=("$script_name:$script_log_file")
        TIMINGS+=("$script_name:$script_duration")

        # Show last few lines of output
        echo -e "${YELLOW}  Last 10 lines of output:${NC}"
        tail -10 "$script_log_file" | sed 's/^/    /'
        return 1
    fi
}

# Function to generate detailed report
generate_report() {
    local end_time=$(date +%s)
    local total_duration=$((end_time - START_TIME))
    local total_minutes=$((total_duration / 60))
    local total_seconds=$((total_duration % 60))

    cat > "$REPORT_FILE" << EOF
# CI PR Validation Report

**Generated:** $(date)
**Total Duration:** ${total_minutes}m ${total_seconds}s
**Target Platform:** $TARGET
**PR Number:** ${PR_NUMBER:-"N/A"}

## Summary

| Metric | Count |
|--------|-------|
| Total Scripts | $((SCRIPTS_PASSED + SCRIPTS_FAILED)) |
| ✅ Passed | $SCRIPTS_PASSED |
| ❌ Failed | $SCRIPTS_FAILED |
| Success Rate | $(echo "scale=1; $SCRIPTS_PASSED * 100 / ($SCRIPTS_PASSED + $SCRIPTS_FAILED)" | bc 2>/dev/null || echo "N/A")% |

## Results by Category

EOF

    if [ $SCRIPTS_FAILED -eq 0 ]; then
        cat >> "$REPORT_FILE" << EOF
### 🎉 All Checks Passed!

The codebase is ready for merge and deployment.

EOF
    else
        cat >> "$REPORT_FILE" << EOF
### ❌ Failed Checks

The following checks failed and need to be addressed:

EOF
        for failed_script in "${FAILED_SCRIPTS[@]}"; do
            local name=$(echo "$failed_script" | cut -d':' -f1)
            local log_file=$(echo "$failed_script" | cut -d':' -f2)
            cat >> "$REPORT_FILE" << EOF
- **$name** - [View Log]($log_file)

EOF
        done
    fi

    cat >> "$REPORT_FILE" << EOF
## Performance Metrics

EOF
    for timing in "${TIMINGS[@]}"; do
        local name=$(echo "$timing" | cut -d':' -f1)
        local duration=$(echo "$timing" | cut -d':' -f2)
        local minutes=$((duration / 60))
        local seconds=$((duration % 60))
        cat >> "$REPORT_FILE" << EOF
- **$name:** ${minutes}m ${seconds}s
EOF
    done

    cat >> "$REPORT_FILE" << EOF

## Environment Information

- **OS:** $(uname -s -r)
- **Node.js:** $(node --version 2>/dev/null || echo "Not installed")
- **Rust:** $(rustc --version 2>/dev/null || echo "Not installed")
- **Cargo:** $(cargo --version 2>/dev/null || echo "Not installed")
- **Yarn:** $(yarn --version 2>/dev/null || echo "Not installed")

## Recommendations

EOF

    if [ $SCRIPTS_FAILED -eq 0 ]; then
        cat >> "$REPORT_FILE" << EOF
✅ **Ready to Merge!** All checks are passing.

- Commit your changes
- Push to your branch
- Create or update your PR
- The CI pipeline should pass successfully

EOF
    else
        cat >> "$REPORT_FILE" << EOF
❌ **Action Required:** Fix the failed checks above.

1. Review the failure logs
2. Address the issues
3. Re-run this validation script
4. Ensure all tests pass before merging

**Quick Commands:**
\`\`\`bash
# Re-run failed script
./scripts/ci-quick-check.sh

# Re-run full validation
./scripts/ci-pr-validation.sh

# Run individual scripts
./scripts/ci-check-format.sh
./scripts/ci-check-frontend.sh
./scripts/ci-check-rust.sh $TARGET
./scripts/ci-check-tests.sh
\`\`\`

EOF
    fi

    cat >> "$REPORT_FILE" << EOF
---
*Report generated by CI PR Validation Script*
EOF

    echo -e "${CYAN}📄 Detailed report generated: $REPORT_FILE${NC}"
}

# Check PR information if PR number provided
if [[ -n "$PR_NUMBER" ]]; then
    echo -e "${BLUE}🔍 Fetching PR information...${NC}"
    if command -v gh &> /dev/null; then
        if gh pr view "$PR_NUMBER" --json title,headRefName,baseRefName > /dev/null 2>&1; then
            echo -e "${GREEN}✅ PR #$PR_NUMBER found${NC}"
            PR_INFO=$(gh pr view "$PR_NUMBER" --json title,headRefName,baseRefName)
            PR_TITLE=$(echo "$PR_INFO" | jq -r '.title')
            PR_HEAD=$(echo "$PR_INFO" | jq -r '.headRefName')
            PR_BASE=$(echo "$PR_INFO" | jq -r '.baseRefName')
            echo "  Title: $PR_TITLE"
            echo "  Branch: $PR_HEAD → $PR_BASE"
        else
            echo -e "${YELLOW}⚠️  PR #$PR_NUMBER not found or accessible${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  gh CLI not found, skipping PR info fetch${NC}"
    fi
fi

# Check if all scripts exist
echo -e "${BLUE}🔍 Checking script availability...${NC}"
REQUIRED_SCRIPTS=(
    "ci-check-format.sh"
    "ci-check-frontend.sh"
    "ci-check-rust.sh"
    "ci-check-tests.sh"
    "ci-check-desktop.sh"
)

for script in "${REQUIRED_SCRIPTS[@]}"; do
    if [[ ! -f "$SCRIPT_DIR/$script" ]]; then
        echo -e "${RED}❌ Required script not found: $SCRIPT_DIR/$script${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✅ All required scripts found${NC}"

# Run scripts with detailed reporting
echo -e "\n${BLUE}📋 Running PR Validation Scripts${NC}"
echo "===================================="

# 1. Code Quality Checks
run_script_with_report "Format Check" \
    "$SCRIPT_DIR/ci-check-format.sh" \
    "Code formatting and linting checks" \
    "Code Quality"

# 2. Frontend Validation
run_script_with_report "Frontend Check" \
    "$SCRIPT_DIR/ci-check-frontend.sh" \
    "Frontend build and test validation" \
    "Frontend"

# 3. Rust Build Validation
run_script_with_report "Rust Build Check" \
    "$SCRIPT_DIR/ci-check-rust.sh $TARGET" \
    "Rust build and cross-compilation validation" \
    "Backend"

# 4. Test Suite Validation
run_script_with_report "Test Suite Check" \
    "$SCRIPT_DIR/ci-check-tests.sh" \
    "Unit, integration, and documentation tests" \
    "Testing"

# 5. Desktop Application Validation
run_script_with_report "Desktop Test Check" \
    "$SCRIPT_DIR/ci-check-desktop.sh" \
    "Desktop application E2E tests" \
    "Testing"

# Generate report
if [[ "$GENERATE_REPORT" == "true" ]]; then
    generate_report
fi

# Calculate total time
END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))
TOTAL_MINUTES=$((TOTAL_DURATION / 60))
TOTAL_SECONDS=$((TOTAL_DURATION % 60))

echo -e "\n${BLUE}📊 PR Validation Results${NC}"
echo "========================"
echo "Total time: ${TOTAL_MINUTES}m ${TOTAL_SECONDS}s"

TOTAL_SCRIPTS=$((SCRIPTS_PASSED + SCRIPTS_FAILED))
echo "Total scripts: $TOTAL_SCRIPTS"
echo -e "${GREEN}Passed: $SCRIPTS_PASSED${NC}"
echo -e "${RED}Failed: $SCRIPTS_FAILED${NC}"

if [ $SCRIPTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 PR VALIDATION PASSED!${NC}"
    echo ""
    echo "✅ Code quality: PASSED"
    echo "✅ Frontend: PASSED"
    echo "✅ Backend build: PASSED"
    echo "✅ Test suite: PASSED"
    echo "✅ Desktop tests: PASSED"
    echo ""
    echo "🚀 PR is ready for merge!"
    if [[ -f "$REPORT_FILE" ]]; then
        echo "📄 Detailed report: $REPORT_FILE"
    fi
    exit 0
else
    echo -e "\n${RED}❌ PR VALIDATION FAILED!${NC}"
    echo ""
    echo "Failed scripts:"
    for failed_script in "${FAILED_SCRIPTS[@]}"; do
        local name=$(echo "$failed_script" | cut -d':' -f1)
        local log_file=$(echo "$failed_script" | cut -d':' -f2)
        echo -e "${RED}  - $name (log: $log_file)${NC}"
    done
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Review the failure logs above"
    echo "2. Fix the issues"
    echo "3. Re-run: $0 $PR_NUMBER"
    if [[ -f "$REPORT_FILE" ]]; then
        echo "📄 Detailed report: $REPORT_FILE"
    fi
    exit 1
fi
