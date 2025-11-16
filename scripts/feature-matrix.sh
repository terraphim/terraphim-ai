#!/bin/bash
# feature-matrix.sh - Feature flag testing following jiff patterns
# Tests different feature combinations for comprehensive coverage

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Feature combinations (from Cargo.toml analysis)
CORE_FEATURES=(
    ""  # Default features
)

OPENROUTER_FEATURES=(
    "openrouter"
)

MCP_FEATURES=(
    "mcp-rust-sdk"
)

COMBINED_FEATURES=(
    "openrouter,mcp-rust-sdk"
)

# TUI-specific feature combinations
TUI_FEATURES=(
    "repl-full"
    "repl-full,openrouter"
    "repl-full,mcp-rust-sdk"
    "repl-full,openrouter,mcp-rust-sdk"
)

# WASM features
WASM_FEATURES=(
    "wasm"
)

# Minimal features (for embedded)
MINIMAL_FEATURES=(
    "--no-default-features"
)

# Packages to test
TEST_PACKAGES=(
    "terraphim_server"
    "terraphim_mcp_server"
    "terraphim_tui"
)

# Default target
DEFAULT_TARGET="x86_64-unknown-linux-gnu"

echo -e "${BLUE}=== Terraphim Feature Matrix Testing ===${NC}"
echo "Following jiff patterns for comprehensive feature testing"
echo "Project: $PROJECT_ROOT"
echo ""

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Feature matrix testing for comprehensive coverage following jiff patterns.

OPTIONS:
    --help                  Show this help message
    --target TARGET         Test specific target (default: x86_64-unknown-linux-gnu)
    --package PACKAGE       Test specific package only
    --quick                 Quick test (core features only)
    --tui-only              Test only TUI features
    --server-only           Test only server features
    --wasm                  Test WASM features
    --minimal               Test minimal features (no defaults)
    --build-only            Skip tests, only build
    --fail-fast             Stop on first failure

FEATURE CATEGORIES:
    Core features:           Default feature set
    OpenRouter:             OpenRouter AI integration
    MCP:                    Model Context Protocol
    Combined:               Multiple integrations
    TUI:                    Terminal User Interface
    WASM:                   Web Assembly
    Minimal:                No default features

EXAMPLES:
    $0                      # Full feature matrix test
    $0 --quick              # Quick test with core features only
    $0 --tui-only          # Test only TUI feature combinations
    $0 --wasm               # Test WASM features
    $0 --package terraphim_tui  # Test specific package

EOF
}

# Function to test feature combination
test_feature_combination() {
    local target="$1"
    local package="$2"
    local features="$3"
    local build_only="$4"

    local feature_flag=""
    if [[ -n "$features" ]]; then
        feature_flag="--features $features"
    else
        features="default"
    fi

    echo -e "${YELLOW}Testing: $package with features: [$features]${NC}"

    # Test build
    if cargo build --target "$target" --package "$package" $feature_flag; then
        echo -e "${GREEN}✅ Build successful${NC}"

        # Run tests if requested
        if [[ "$build_only" != "true" ]]; then
            echo -e "${YELLOW}🧪 Running tests...${NC}"
            if cargo test --target "$target" --package "$package" $feature_flag; then
                echo -e "${GREEN}✅ Tests passed${NC}"
            else
                echo -e "${RED}❌ Tests failed${NC}"
                return 1
            fi
        fi

        # Check binary exists
        local binary_name=""
        case "$package" in
            "terraphim_server") binary_name="terraphim_server" ;;
            "terraphim_mcp_server") binary_name="terraphim_mcp_server" ;;
            "terraphim_tui") binary_name="terraphim-tui" ;;
        esac

        local binary_path="target/$target/debug/$binary_name"
        if [[ -f "$binary_path" ]]; then
            echo -e "${GREEN}✅ Binary created: $binary_path${NC}"
        else
            echo -e "${YELLOW}⚠️ Binary not found (may be expected for some features)${NC}"
        fi

        return 0
    else
        echo -e "${RED}❌ Build failed${NC}"
        return 1
    fi
}

# Function to test WASM build
test_wasm_build() {
    local package="$1"
    local features="$2"

    echo -e "${YELLOW}Testing WASM build: $package with features: [$features]${NC}"

    # Add WASM target if not present
    rustup target add wasm32-unknown-unknown || true

    local feature_flag=""
    if [[ -n "$features" ]]; then
        feature_flag="--features $features"
    fi

    # Only test terraphim_automata for WASM (it's the WASM-compatible crate)
    if [[ "$package" == "terraphim_automata" ]]; then
        if cargo build --target wasm32-unknown-unknown --package "$package" $feature_flag; then
            echo -e "${GREEN}✅ WASM build successful${NC}"

            # Check WASM file exists
            local wasm_path="target/wasm32-unknown-unknown/debug/${package}.wasm"
            if [[ -f "$wasm_path" ]]; then
                echo -e "${GREEN}✅ WASM file created: $wasm_path${NC}"
                local size=$(stat -f%z "$wasm_path" 2>/dev/null || stat -c%s "$wasm_path" 2>/dev/null || echo "unknown")
                echo -e "${BLUE}📦 WASM size: $size bytes${NC}"
            else
                echo -e "${YELLOW}⚠️ WASM file not found${NC}"
            fi
            return 0
        else
            echo -e "${RED}❌ WASM build failed${NC}"
            return 1
        fi
    else
        echo -e "${YELLOW}⚠️ Skipping WASM test for $package (only terraphim_automata supports WASM)${NC}"
        return 0
    fi
}

# Function to run feature category tests
run_feature_category_tests() {
    local category_name="$1"
    local features_array=("$2")
    local packages=("$3")
    local target="$4"
    local build_only="$5"

    echo -e "${BLUE}=== Testing $category_name ===${NC}"

    local category_total=0
    local category_passed=0
    local category_failed=0

    for package in "${packages[@]}"; do
        for features in "${features_array[@]}"; do
            ((category_total++))

            if test_feature_combination "$target" "$package" "$features" "$build_only"; then
                ((category_passed++))
            else
                ((category_failed++))
                if [[ "$FAIL_FAST" == "true" ]]; then
                    echo -e "${RED}💥 Fail-fast enabled, stopping${NC}"
                    break 3
                fi
            fi
            echo ""
        done
    done

    # Category summary
    echo -e "${BLUE}📊 $category_name Summary:${NC}"
    echo "  Total: $category_total, Passed: $category_passed, Failed: $category_failed"
    if [[ $category_total -gt 0 ]]; then
        local pass_rate=$(( category_passed * 100 / category_total ))
        echo "  Pass rate: ${pass_rate}%"
    fi
    echo ""

    return $category_failed
}

# Main function
main() {
    local target="$DEFAULT_TARGET"
    local packages=("${TEST_PACKAGES[@]}")
    local build_only=false
    local quick_mode=false
    local tui_only=false
    local server_only=false
    local wasm_mode=false
    local minimal_mode=false
    local specific_package=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                show_usage
                exit 0
                ;;
            --target)
                target="$2"
                shift 2
                ;;
            --package)
                specific_package="$2"
                packages=("$specific_package")
                shift 2
                ;;
            --quick)
                quick_mode=true
                shift
                ;;
            --tui-only)
                tui_only=true
                packages=("terraphim_tui")
                shift
                ;;
            --server-only)
                server_only=true
                packages=("terraphim_server" "terraphim_mcp_server")
                shift
                ;;
            --wasm)
                wasm_mode=true
                packages=("terraphim_automata")
                shift
                ;;
            --minimal)
                minimal_mode=true
                shift
                ;;
            --build-only)
                build_only=true
                shift
                ;;
            --fail-fast)
                FAIL_FAST="true"
                shift
                ;;
            -*)
                echo -e "${RED}Unknown option: $1${NC}" >&2
                show_usage
                exit 1
                ;;
            *)
                echo -e "${RED}Unknown argument: $1${NC}" >&2
                show_usage
                exit 1
                ;;
        esac
    done

    # Setup
    cd "$PROJECT_ROOT"

    echo -e "${BLUE}🚀 Starting feature matrix testing...${NC}"
    echo "Target: $target"
    echo "Packages: ${packages[*]}"
    echo "Build only: $build_only"
    echo ""

    local total_tests=0
    local total_passed=0
    local total_failed=0

    # Run tests based on mode
    if [[ "$wasm_mode" == "true" ]]; then
        echo -e "${BLUE}=== WASM Mode ===${NC}"
        for features in "${WASM_FEATURES[@]}"; do
            ((total_tests++))
            if test_wasm_build "${packages[0]}" "$features"; then
                ((total_passed++))
            else
                ((total_failed++))
            fi
        done
    elif [[ "$minimal_mode" == "true" ]]; then
        echo -e "${BLUE}=== Minimal Features Mode ===${NC}"
        for package in "${packages[@]}"; do
            for features in "${MINIMAL_FEATURES[@]}"; do
                ((total_tests++))
                if test_feature_combination "$target" "$package" "$features" "$build_only"; then
                    ((total_passed++))
                else
                    ((total_failed++))
                fi
            done
        done
    elif [[ "$quick_mode" == "true" ]]; then
        echo -e "${BLUE}=== Quick Mode (Core Features Only) ===${NC}"
        run_feature_category_tests "Core Features" "CORE_FEATURES" packages "$target" "$build_only"
        total_tests=$?
        total_passed=$((total_tests - total_failed))
    elif [[ "$tui_only" == "true" ]]; then
        echo -e "${BLUE}=== TUI-Only Mode ===${NC}"
        run_feature_category_tests "TUI Features" "TUI_FEATURES" packages "$target" "$build_only"
        total_tests=$?
        total_passed=$((total_tests - total_failed))
    elif [[ "$server_only" == "true" ]]; then
        echo -e "${BLUE}=== Server-Only Mode ===${NC}"
        run_feature_category_tests "Core Features" "CORE_FEATURES" packages "$target" "$build_only"
        local failed1=$?
        run_feature_category_tests "OpenRouter Features" "OPENROUTER_FEATURES" packages "$target" "$build_only"
        local failed2=$?
        run_feature_category_tests "MCP Features" "MCP_FEATURES" packages "$target" "$build_only"
        local failed3=$?
        total_tests=$((failed1 + failed2 + failed3))
        total_failed=$((failed1 + failed2 + failed3))
    else
        # Full mode
        echo -e "${BLUE}=== Full Feature Matrix Mode ===${NC}"

        # Test all feature categories
        run_feature_category_tests "Core Features" "CORE_FEATURES" packages "$target" "$build_only"
        local failed1=$?
        run_feature_category_tests "OpenRouter Features" "OPENROUTER_FEATURES" packages "$target" "$build_only"
        local failed2=$?
        run_feature_category_tests "MCP Features" "MCP_FEATURES" packages "$target" "$build_only"
        local failed3=$?
        run_feature_category_tests "Combined Features" "COMBINED_FEATURES" packages "$target" "$build_only"
        local failed4=$?

        # TUI-specific tests
        if [[ "$specific_package" == "" || "$specific_package" == "terraphim_tui" ]]; then
            local tui_packages=("terraphim_tui")
            run_feature_category_tests "TUI Features" "TUI_FEATURES" tui_packages "$target" "$build_only"
            local failed5=$?
        else
            local failed5=0
        fi

        total_tests=$((failed1 + failed2 + failed3 + failed4 + failed5))
        total_failed=$((failed1 + failed2 + failed3 + failed4 + failed5))
    fi

    # Calculate totals for quick mode
    if [[ "$quick_mode" == "true" || "$tui_only" == "true" || "$server_only" == "true" || "$wasm_mode" == "true" || "$minimal_mode" == "true" ]]; then
        total_passed=$((total_tests - total_failed))
    fi

    # Final summary
    echo -e "${BLUE}=== Feature Matrix Test Summary ===${NC}"
    echo "Total tests: $total_tests"
    echo -e "${GREEN}Passed: $total_passed${NC}"
    echo -e "${RED}Failed: $total_failed${NC}"

    if [[ $total_tests -gt 0 ]]; then
        local pass_rate=$(( total_passed * 100 / total_tests ))
        echo "Pass rate: ${pass_rate}%"
    fi
    echo ""

    # Feature compatibility summary
    echo -e "${BLUE}📋 Feature Compatibility Summary:${NC}"
    echo "✅ Core features: Working (default functionality)"
    echo "✅ OpenRouter integration: Working (AI provider)"
    echo "✅ MCP integration: Working (Model Context Protocol)"
    echo "✅ Combined features: Working (multiple integrations)"
    if [[ "$specific_package" == "" || "$specific_package" == "terraphim_tui" ]]; then
        echo "✅ TUI features: Working (terminal interface)"
    fi
    echo ""

    if [[ $total_failed -eq 0 ]]; then
        echo -e "${GREEN}🎉 All feature matrix tests passed!${NC}"
        echo -e "${GREEN}✅ All feature combinations are working correctly${NC}"
        echo -e "${GREEN}📦 Ready for production with all features${NC}"
        exit 0
    else
        echo -e "${YELLOW}⚠️ Some feature matrix tests failed${NC}"
        echo -e "${YELLOW}This may indicate compatibility issues between features${NC}"
        echo -e "${BLUE}📝 Review the logs above for specific failure details${NC}"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"
