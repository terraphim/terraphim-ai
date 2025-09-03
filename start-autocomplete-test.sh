#!/bin/bash

# Terraphim Novel Autocomplete Testing Startup Script
# This script starts all necessary services for testing the Novel editor autocomplete integration

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$SCRIPT_DIR/logs"
PID_DIR="$SCRIPT_DIR/pids"
MCP_SERVER_PORT=${MCP_SERVER_PORT:-8001}
WEB_SERVER_PORT=${WEB_SERVER_PORT:-3000}

# Create directories
mkdir -p "$LOG_DIR" "$PID_DIR"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}🧹 Cleaning up processes...${NC}"

    # Kill all processes in our PID files
    for pid_file in "$PID_DIR"/*.pid; do
        if [[ -f "$pid_file" ]]; then
            pid=$(cat "$pid_file")
            process_name=$(basename "$pid_file" .pid)

            if kill -0 "$pid" 2>/dev/null; then
                echo -e "${YELLOW}   Stopping $process_name (PID: $pid)${NC}"
                kill -TERM "$pid" 2>/dev/null || true

                # Give process time to shut down gracefully
                sleep 2

                # Force kill if still running
                if kill -0 "$pid" 2>/dev/null; then
                    echo -e "${YELLOW}   Force stopping $process_name${NC}"
                    kill -KILL "$pid" 2>/dev/null || true
                fi
            fi

            rm -f "$pid_file"
        fi
    done

    # Additional cleanup for any remaining processes
    pkill -f "terraphim_mcp_server" 2>/dev/null || true
    pkill -f "terraphim_server" 2>/dev/null || true
    pkill -f "yarn.*tauri.*dev" 2>/dev/null || true

    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM EXIT

# Helper functions
log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}❌ ERROR: $1${NC}" >&2
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if port is available
port_available() {
    ! lsof -ti:$1 >/dev/null 2>&1
}

# Wait for service to be ready
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=${3:-30}
    local attempt=1

    log "Waiting for $name to be ready..."

    while [[ $attempt -le $max_attempts ]]; do
        if curl -s --connect-timeout 2 "$url" >/dev/null 2>&1; then
            success "$name is ready!"
            return 0
        fi

        echo -n "."
        sleep 1
        ((attempt++))
    done

    error "$name failed to start after $max_attempts attempts"
    return 1
}

# Start MCP server
start_mcp_server() {
    log "Starting MCP server on port $MCP_SERVER_PORT..."

    if ! port_available $MCP_SERVER_PORT; then
        warning "Port $MCP_SERVER_PORT is already in use"
        local existing_pid=$(lsof -ti:$MCP_SERVER_PORT)
        if [[ -n "$existing_pid" ]]; then
            warning "Killing existing process on port $MCP_SERVER_PORT (PID: $existing_pid)"
            kill -TERM "$existing_pid" 2>/dev/null || true
            sleep 2
        fi
    fi

    cd "$SCRIPT_DIR/crates/terraphim_mcp_server"

    # Start MCP server in background
    RUST_LOG=info cargo run -- \
        --sse \
        --bind "127.0.0.1:$MCP_SERVER_PORT" \
        --verbose \
        > "$LOG_DIR/mcp_server.log" 2>&1 &

    local mcp_pid=$!
    echo $mcp_pid > "$PID_DIR/mcp_server.pid"

    log "MCP server started (PID: $mcp_pid)"

    # Wait for MCP server to be ready
    wait_for_service "http://127.0.0.1:$MCP_SERVER_PORT/health" "MCP server" 30

    cd "$SCRIPT_DIR"
}

# Start Axum server (optional)
start_axum_server() {
    log "Starting Axum server..."

    cd "$SCRIPT_DIR/terraphim_server"

    # Start Axum server in background
    RUST_LOG=info cargo run -- \
        --config default/terraphim_engineer_config.json \
        > "$LOG_DIR/axum_server.log" 2>&1 &

    local axum_pid=$!
    echo $axum_pid > "$PID_DIR/axum_server.pid"

    log "Axum server started (PID: $axum_pid)"

    cd "$SCRIPT_DIR"
}

# Build and prepare desktop app
prepare_desktop_app() {
    log "Preparing desktop application..."

    cd "$SCRIPT_DIR/desktop"

    # Install dependencies if needed
    if [[ ! -d "node_modules" ]] || [[ "package.json" -nt "node_modules" ]]; then
        log "Installing Node.js dependencies..."
        yarn install
    fi

    # Build Tauri app if needed
    if [[ ! -d "src-tauri/target" ]]; then
        log "Building Tauri dependencies (this may take a while)..."
        yarn tauri build --debug
    fi

    cd "$SCRIPT_DIR"
    success "Desktop app prepared"
}

# Start desktop app
start_desktop_app() {
    log "Starting desktop application..."

    cd "$SCRIPT_DIR/desktop"

    # Set environment variables for the app
    export TERRAPHIM_INITIALIZED=true
    export MCP_SERVER_URL="http://127.0.0.1:$MCP_SERVER_PORT"

    # Start Tauri dev server in background
    yarn run tauri dev > "$LOG_DIR/desktop_app.log" 2>&1 &

    local desktop_pid=$!
    echo $desktop_pid > "$PID_DIR/desktop_app.pid"

    log "Desktop app started (PID: $desktop_pid)"

    cd "$SCRIPT_DIR"
}

# Run integration tests
run_integration_tests() {
    log "Running integration tests..."

    cd "$SCRIPT_DIR/desktop"

    if [[ -f "test-novel-autocomplete-integration.js" ]]; then
        node test-novel-autocomplete-integration.js
    else
        warning "Integration test script not found, skipping tests"
    fi

    cd "$SCRIPT_DIR"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."

    local missing_deps=()

    # Check for required commands
    if ! command_exists cargo; then
        missing_deps+=("cargo (Rust)")
    fi

    if ! command_exists node; then
        missing_deps+=("node.js")
    fi

    if ! command_exists yarn; then
        missing_deps+=("yarn")
    fi

    if ! command_exists curl; then
        missing_deps+=("curl")
    fi

    if ! command_exists lsof; then
        missing_deps+=("lsof")
    fi

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        error "Missing required dependencies:"
        for dep in "${missing_deps[@]}"; do
            error "  - $dep"
        done
        exit 1
    fi

    # Check if we're in the right directory
    if [[ ! -d "crates/terraphim_mcp_server" ]] || [[ ! -d "desktop" ]]; then
        error "Please run this script from the terraphim-ai root directory"
        exit 1
    fi

    success "All prerequisites found"
}

# Show usage information
show_usage() {
    cat << EOF
${PURPLE}🚀 Terraphim Novel Autocomplete Testing Startup Script${NC}

Usage: $0 [options]

Options:
    -h, --help              Show this help message
    -m, --mcp-only          Start only MCP server
    -a, --axum-only         Start only Axum server
    -d, --desktop-only      Start only desktop app
    -t, --test-only         Run only integration tests
    -p, --port PORT         Set MCP server port (default: 8001)
    -w, --web-port PORT     Set web server port (default: 3000)
    --no-desktop            Skip desktop app startup
    --no-tests              Skip integration tests
    --verbose               Enable verbose logging

Examples:
    $0                      Start all services
    $0 --mcp-only          Start only MCP server
    $0 --port 8080         Use port 8080 for MCP server
    $0 --no-desktop        Start servers but not desktop app

Services:
    • MCP Server:          http://127.0.0.1:$MCP_SERVER_PORT
    • Desktop App:         Tauri application window
    • Logs:                $LOG_DIR/
    • PIDs:                $PID_DIR/

Controls:
    Ctrl+C                 Stop all services and exit

EOF
}

# Main execution
main() {
    local start_mcp=true
    local start_axum=false
    local start_desktop=true
    local run_tests=true
    local verbose=false

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -m|--mcp-only)
                start_mcp=true
                start_axum=false
                start_desktop=false
                run_tests=false
                ;;
            -a|--axum-only)
                start_mcp=false
                start_axum=true
                start_desktop=false
                run_tests=false
                ;;
            -d|--desktop-only)
                start_mcp=false
                start_axum=false
                start_desktop=true
                run_tests=false
                ;;
            -t|--test-only)
                start_mcp=false
                start_axum=false
                start_desktop=false
                run_tests=true
                ;;
            -p|--port)
                MCP_SERVER_PORT="$2"
                shift
                ;;
            -w|--web-port)
                WEB_SERVER_PORT="$2"
                shift
                ;;
            --no-desktop)
                start_desktop=false
                ;;
            --no-tests)
                run_tests=false
                ;;
            --verbose)
                verbose=true
                export RUST_LOG=debug
                ;;
            *)
                error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
        shift
    done

    # Show banner
    cat << EOF
${PURPLE}
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║  🚀 Terraphim Novel Autocomplete Testing Environment        ║
║                                                              ║
║  Starting services for testing the Novel editor             ║
║  autocomplete integration with knowledge graphs             ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
${NC}

EOF

    # Check prerequisites
    check_prerequisites

    # Show configuration
    info "Configuration:"
    info "  MCP Server Port: $MCP_SERVER_PORT"
    info "  Web Server Port: $WEB_SERVER_PORT"
    info "  Log Directory: $LOG_DIR"
    info "  PID Directory: $PID_DIR"
    info "  Start MCP: $start_mcp"
    info "  Start Axum: $start_axum"
    info "  Start Desktop: $start_desktop"
    info "  Run Tests: $run_tests"
    echo

    # Start services based on configuration
    if [[ "$start_mcp" == "true" ]]; then
        start_mcp_server
        echo
    fi

    if [[ "$start_axum" == "true" ]]; then
        start_axum_server
        echo
    fi

    if [[ "$start_desktop" == "true" ]]; then
        prepare_desktop_app
        start_desktop_app
        echo
    fi

    # Wait a bit for all services to stabilize
    if [[ "$start_mcp" == "true" ]] || [[ "$start_axum" == "true" ]] || [[ "$start_desktop" == "true" ]]; then
        log "Waiting for services to stabilize..."
        sleep 3
    fi

    # Run integration tests
    if [[ "$run_tests" == "true" ]]; then
        run_integration_tests
        echo
    fi

    # Show status and instructions
    cat << EOF
${GREEN}
🎉 Terraphim Novel Autocomplete Testing Environment Ready!
${NC}

${CYAN}📋 Services Status:${NC}
EOF

    if [[ "$start_mcp" == "true" ]]; then
        echo -e "   • ${GREEN}MCP Server:${NC} http://127.0.0.1:$MCP_SERVER_PORT"
    fi

    if [[ "$start_axum" == "true" ]]; then
        echo -e "   • ${GREEN}Axum Server:${NC} Running (check logs)"
    fi

    if [[ "$start_desktop" == "true" ]]; then
        echo -e "   • ${GREEN}Desktop App:${NC} Starting (Tauri window should appear)"
    fi

    cat << EOF

${CYAN}🧪 Testing Instructions:${NC}
   1. Open the Terraphim desktop app (if started)
   2. Navigate to an editor page
   3. Click "Demo" button to insert test content
   4. Type "/" followed by a term (e.g., "/terraphim")
   5. Verify autocomplete suggestions appear
   6. Use ↑↓ arrows to navigate, Tab/Enter to select

${CYAN}📊 Monitoring:${NC}
   • Logs: $LOG_DIR/
   • MCP Server: $LOG_DIR/mcp_server.log
   • Desktop App: $LOG_DIR/desktop_app.log
   • Process PIDs: $PID_DIR/

${CYAN}🔧 Troubleshooting:${NC}
   • Check logs if services don't start
   • Verify ports are available with: lsof -i:$MCP_SERVER_PORT
   • Restart with: ./start-autocomplete-test.sh

${YELLOW}Press Ctrl+C to stop all services and exit${NC}

EOF

    # Keep script running until interrupted
    if [[ "$start_mcp" == "true" ]] || [[ "$start_axum" == "true" ]] || [[ "$start_desktop" == "true" ]]; then
        log "Services running. Press Ctrl+C to stop..."

        # Monitor processes
        while true; do
            local failed_services=()

            # Check MCP server
            if [[ "$start_mcp" == "true" ]] && [[ -f "$PID_DIR/mcp_server.pid" ]]; then
                local mcp_pid=$(cat "$PID_DIR/mcp_server.pid")
                if ! kill -0 "$mcp_pid" 2>/dev/null; then
                    failed_services+=("MCP Server")
                fi
            fi

            # Check desktop app
            if [[ "$start_desktop" == "true" ]] && [[ -f "$PID_DIR/desktop_app.pid" ]]; then
                local desktop_pid=$(cat "$PID_DIR/desktop_app.pid")
                if ! kill -0 "$desktop_pid" 2>/dev/null; then
                    failed_services+=("Desktop App")
                fi
            fi

            # Report failed services
            if [[ ${#failed_services[@]} -gt 0 ]]; then
                for service in "${failed_services[@]}"; do
                    error "$service has stopped unexpectedly"
                done
                break
            fi

            sleep 5
        done
    fi
}

# Run main function with all arguments
main "$@"
