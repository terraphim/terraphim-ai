#!/bin/bash

# Terraphim Novel Autocomplete Testing Stop Script
# This script stops all services started by start-autocomplete-test.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_DIR="$SCRIPT_DIR/pids"

# Helper functions
log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Stop function
stop_services() {
    log "Stopping Terraphim autocomplete testing services..."

    local stopped_count=0

    # Stop services from PID files
    if [[ -d "$PID_DIR" ]]; then
        for pid_file in "$PID_DIR"/*.pid; do
            if [[ -f "$pid_file" ]]; then
                local pid=$(cat "$pid_file")
                local process_name=$(basename "$pid_file" .pid)

                if kill -0 "$pid" 2>/dev/null; then
                    log "Stopping $process_name (PID: $pid)"
                    kill -TERM "$pid" 2>/dev/null || true

                    # Give process time to shut down gracefully
                    sleep 2

                    # Force kill if still running
                    if kill -0 "$pid" 2>/dev/null; then
                        warning "Force stopping $process_name"
                        kill -KILL "$pid" 2>/dev/null || true
                    fi

                    ((stopped_count++))
                else
                    log "$process_name was not running"
                fi

                rm -f "$pid_file"
            fi
        done
    fi

    # Additional cleanup for any remaining processes
    local additional_killed=0

    # Kill MCP server processes
    if pkill -f "terraphim_mcp_server" 2>/dev/null; then
        log "Stopped additional MCP server processes"
        ((additional_killed++))
    fi

    # Kill Axum server processes
    if pkill -f "terraphim_server" 2>/dev/null; then
        log "Stopped additional Axum server processes"
        ((additional_killed++))
    fi

    # Kill Tauri dev processes
    if pkill -f "yarn.*tauri.*dev" 2>/dev/null; then
        log "Stopped additional Tauri dev processes"
        ((additional_killed++))
    fi

    # Clean up PID directory
    if [[ -d "$PID_DIR" ]]; then
        rmdir "$PID_DIR" 2>/dev/null || true
    fi

    # Summary
    local total_stopped=$((stopped_count + additional_killed))

    if [[ $total_stopped -gt 0 ]]; then
        success "Stopped $total_stopped process(es)"
    else
        log "No services were running"
    fi

    success "All Terraphim autocomplete testing services stopped"
}

# Check for running processes
check_status() {
    echo -e "${BLUE}📊 Checking service status...${NC}"

    local running_processes=()

    # Check common ports
    local ports=(8001 3000 8000 8080)
    for port in "${ports[@]}"; do
        if lsof -ti:$port >/dev/null 2>&1; then
            local pid=$(lsof -ti:$port)
            running_processes+=("Port $port (PID: $pid)")
        fi
    done

    # Check specific processes
    if pgrep -f "terraphim_mcp_server" >/dev/null 2>&1; then
        local pid=$(pgrep -f "terraphim_mcp_server")
        running_processes+=("MCP Server (PID: $pid)")
    fi

    if pgrep -f "terraphim_server" >/dev/null 2>&1; then
        local pid=$(pgrep -f "terraphim_server")
        running_processes+=("Axum Server (PID: $pid)")
    fi

    if pgrep -f "yarn.*tauri.*dev" >/dev/null 2>&1; then
        local pid=$(pgrep -f "yarn.*tauri.*dev")
        running_processes+=("Tauri Dev (PID: $pid)")
    fi

    if [[ ${#running_processes[@]} -gt 0 ]]; then
        warning "Found running processes:"
        for process in "${running_processes[@]}"; do
            echo -e "   • $process"
        done
        return 1
    else
        success "No services are running"
        return 0
    fi
}

# Show usage
show_usage() {
    cat << EOF
${BLUE}🛑 Terraphim Novel Autocomplete Testing Stop Script${NC}

Usage: $0 [options]

Options:
    -h, --help      Show this help message
    -s, --status    Check status of running services
    -f, --force     Force kill all processes (use if normal stop fails)

Examples:
    $0              Stop all services gracefully
    $0 --status     Check what services are running
    $0 --force      Force stop all services

EOF
}

# Force stop (more aggressive)
force_stop() {
    log "Force stopping all Terraphim services..."

    # Kill by process name patterns
    pkill -9 -f "terraphim_mcp_server" 2>/dev/null || true
    pkill -9 -f "terraphim_server" 2>/dev/null || true
    pkill -9 -f "yarn.*tauri.*dev" 2>/dev/null || true
    pkill -9 -f "cargo.*run.*terraphim" 2>/dev/null || true

    # Kill by ports
    local ports=(8001 3000 8000 8080)
    for port in "${ports[@]}"; do
        if lsof -ti:$port >/dev/null 2>&1; then
            local pid=$(lsof -ti:$port)
            kill -9 "$pid" 2>/dev/null || true
            log "Force killed process on port $port (PID: $pid)"
        fi
    done

    # Clean up PID files
    rm -rf "$PID_DIR" 2>/dev/null || true

    success "Force stop completed"
}

# Main execution
main() {
    local force_mode=false
    local status_only=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -s|--status)
                status_only=true
                ;;
            -f|--force)
                force_mode=true
                ;;
            *)
                echo -e "${RED}❌ Unknown option: $1${NC}"
                show_usage
                exit 1
                ;;
        esac
        shift
    done

    # Show banner
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║  🛑 Terraphim Novel Autocomplete Testing Stop Script        ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    if [[ "$status_only" == "true" ]]; then
        check_status
        exit $?
    elif [[ "$force_mode" == "true" ]]; then
        force_stop
    else
        stop_services

        # Verify everything stopped
        sleep 1
        if ! check_status >/dev/null 2>&1; then
            warning "Some processes may still be running. Use --force if needed."
            check_status
        fi
    fi
}

# Run main with all arguments
main "$@"
