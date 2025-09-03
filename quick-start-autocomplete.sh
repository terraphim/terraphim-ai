#!/bin/bash

# Quick Start Script for Terraphim Novel Autocomplete Testing
# Common usage scenarios made easy

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

show_menu() {
    cat << EOF
${PURPLE}
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║  ⚡ Quick Start - Terraphim Novel Autocomplete Testing      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
${NC}

${BLUE}Choose a testing scenario:${NC}

${GREEN}1)${NC} Full Testing Environment
   🚀 Start MCP server + Desktop app + Run tests
   ${YELLOW}Perfect for comprehensive testing${NC}

${GREEN}2)${NC} MCP Server Only
   🔧 Start just the MCP server for API testing
   ${YELLOW}Good for backend development${NC}

${GREEN}3)${NC} Desktop Development
   💻 Start MCP server + Desktop app (no tests)
   ${YELLOW}Ideal for UI development${NC}

${GREEN}4)${NC} Run Tests Only
   🧪 Run integration tests against existing services
   ${YELLOW}Quick validation of running services${NC}

${GREEN}5)${NC} Status Check
   📊 Check what services are currently running
   ${YELLOW}See what's already running${NC}

${GREEN}6)${NC} Stop All Services
   🛑 Stop all running Terraphim services
   ${YELLOW}Clean shutdown of everything${NC}

${GREEN}7)${NC} Custom Configuration
   ⚙️  Advanced options and custom ports
   ${YELLOW}For custom setups${NC}

${GREEN}q)${NC} Quit

EOF
}

# Execute choice
execute_choice() {
    local choice=$1

    case $choice in
        1)
            echo -e "${GREEN}🚀 Starting Full Testing Environment...${NC}"
            exec "$SCRIPT_DIR/start-autocomplete-test.sh"
            ;;
        2)
            echo -e "${GREEN}🔧 Starting MCP Server Only...${NC}"
            exec "$SCRIPT_DIR/start-autocomplete-test.sh" --mcp-only
            ;;
        3)
            echo -e "${GREEN}💻 Starting Desktop Development Environment...${NC}"
            exec "$SCRIPT_DIR/start-autocomplete-test.sh" --no-tests
            ;;
        4)
            echo -e "${GREEN}🧪 Running Integration Tests...${NC}"
            exec "$SCRIPT_DIR/start-autocomplete-test.sh" --test-only
            ;;
        5)
            echo -e "${GREEN}📊 Checking Service Status...${NC}"
            exec "$SCRIPT_DIR/stop-autocomplete-test.sh" --status
            ;;
        6)
            echo -e "${GREEN}🛑 Stopping All Services...${NC}"
            exec "$SCRIPT_DIR/stop-autocomplete-test.sh"
            ;;
        7)
            show_custom_options
            ;;
        q|Q)
            echo -e "${GREEN}👋 Goodbye!${NC}"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}❓ Invalid choice. Please try again.${NC}"
            ;;
    esac
}

# Custom configuration menu
show_custom_options() {
    echo
    echo -e "${BLUE}⚙️  Custom Configuration Options:${NC}"
    echo

    read -p "MCP Server Port (default 8001): " mcp_port
    mcp_port=${mcp_port:-8001}

    read -p "Enable verbose logging? (y/N): " verbose
    verbose=${verbose:-n}

    echo
    echo -e "${BLUE}Select services to start:${NC}"
    read -p "Start MCP Server? (Y/n): " start_mcp
    start_mcp=${start_mcp:-y}

    read -p "Start Desktop App? (Y/n): " start_desktop
    start_desktop=${start_desktop:-y}

    read -p "Run Integration Tests? (Y/n): " run_tests
    run_tests=${run_tests:-y}

    # Build command
    local cmd="$SCRIPT_DIR/start-autocomplete-test.sh"
    local args=()

    if [[ "$mcp_port" != "8001" ]]; then
        args+=("--port" "$mcp_port")
    fi

    if [[ "$verbose" =~ ^[Yy] ]]; then
        args+=("--verbose")
    fi

    if [[ "$start_desktop" =~ ^[Nn] ]]; then
        args+=("--no-desktop")
    fi

    if [[ "$run_tests" =~ ^[Nn] ]]; then
        args+=("--no-tests")
    fi

    if [[ "$start_mcp" =~ ^[Nn] ]]; then
        # If MCP disabled, we need specific modes
        if [[ "$start_desktop" =~ ^[Yy] ]]; then
            args=("--desktop-only")
        elif [[ "$run_tests" =~ ^[Yy] ]]; then
            args=("--test-only")
        else
            echo -e "${YELLOW}⚠️  No services selected!${NC}"
            return
        fi
    fi

    echo
    echo -e "${GREEN}🚀 Starting with custom configuration...${NC}"
    echo -e "${BLUE}Command: $cmd ${args[*]}${NC}"
    echo

    exec "$cmd" "${args[@]}"
}

# Main execution
main() {
    # Check if we're in the right directory
    if [[ ! -f "$SCRIPT_DIR/start-autocomplete-test.sh" ]]; then
        echo -e "${YELLOW}❌ Error: start-autocomplete-test.sh not found${NC}"
        echo -e "${YELLOW}Please run this script from the terraphim-ai root directory${NC}"
        exit 1
    fi

    # Handle direct command line arguments
    if [[ $# -gt 0 ]]; then
        case $1 in
            1|full) execute_choice 1 ;;
            2|mcp) execute_choice 2 ;;
            3|dev) execute_choice 3 ;;
            4|test) execute_choice 4 ;;
            5|status) execute_choice 5 ;;
            6|stop) execute_choice 6 ;;
            7|custom) execute_choice 7 ;;
            --help|-h)
                echo -e "${BLUE}Quick Start Usage:${NC}"
                echo "  $0              Show interactive menu"
                echo "  $0 full         Full testing environment"
                echo "  $0 mcp          MCP server only"
                echo "  $0 dev          Development environment"
                echo "  $0 test         Run tests only"
                echo "  $0 status       Check service status"
                echo "  $0 stop         Stop all services"
                echo "  $0 custom       Custom configuration"
                exit 0
                ;;
            *)
                echo -e "${YELLOW}❓ Unknown option: $1${NC}"
                echo "Use '$0 --help' for usage information"
                exit 1
                ;;
        esac
        return
    fi

    # Interactive mode
    while true; do
        show_menu
        read -p "Enter your choice: " choice
        echo

        execute_choice "$choice"

        if [[ "$choice" != "5" ]]; then
            # Don't loop back for status check
            break
        fi

        echo
        read -p "Press Enter to continue..."
        clear
    done
}

# Run main with all arguments
main "$@"