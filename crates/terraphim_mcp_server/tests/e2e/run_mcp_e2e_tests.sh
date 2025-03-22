#!/bin/bash
set -e

# Parse command-line arguments
DEBUG=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug)
            DEBUG=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--debug]"
            exit 1
            ;;
    esac
done

# Define directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/../../../../" &> /dev/null && pwd)"
LOG_DIR="$WORKSPACE_DIR/logs"
BINARY_PATH="$WORKSPACE_DIR/target/release/terraphim_mcp_server"
VENV_DIR="$SCRIPT_DIR/.venv"
FIXTURES_DIR="$WORKSPACE_DIR/terraphim_server/fixtures"
CONFIG_PATH="$FIXTURES_DIR/test_config.json"

# Print header
echo "====================================================="
echo "Terraphim MCP Server End-to-End Test Runner"
echo "====================================================="
if [ "$DEBUG" = true ]; then
    echo "Debug mode: enabled"
fi

# Check if Python 3 is installed
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is required but not found."
    exit 1
fi

# Check if fixtures directory exists
if [ ! -d "$FIXTURES_DIR" ]; then
    echo "Error: Fixtures directory not found at $FIXTURES_DIR"
    exit 1
fi

# Check if config file exists
if [ ! -f "$CONFIG_PATH" ]; then
    echo "Error: Config file not found at $CONFIG_PATH"
    exit 1
fi

echo "Using fixtures from: $FIXTURES_DIR"
echo "Using config from: $CONFIG_PATH"

# Check if uv is installed
if ! command -v uv &> /dev/null; then
    echo "Installing uv Python package manager..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    # Add uv to PATH for this session
    export PATH="$HOME/.cargo/bin:$PATH"
    
    # Verify installation
    if ! command -v uv &> /dev/null; then
        echo "Error: Failed to install uv. Please install it manually: https://github.com/astral-sh/uv"
        exit 1
    fi
fi

# Create logs directory
mkdir -p $LOG_DIR
echo "Created logs directory: $LOG_DIR"

# Setup Python virtual environment with uv
echo -e "\nSetting up Python virtual environment..."
if [ ! -d "$VENV_DIR" ]; then
    uv venv $VENV_DIR
    echo "Created virtual environment at $VENV_DIR"
else
    echo "Using existing virtual environment at $VENV_DIR"
fi

# Determine appropriate activation script based on OS
if [[ "$OSTYPE" == "darwin"* ]] || [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # macOS or Linux
    ACTIVATE_SCRIPT="$VENV_DIR/bin/activate"
else
    # Windows
    ACTIVATE_SCRIPT="$VENV_DIR/Scripts/activate"
fi

# Activate virtual environment
if [ -f "$ACTIVATE_SCRIPT" ]; then
    source "$ACTIVATE_SCRIPT"
    echo "Activated virtual environment"
else
    echo "Error: Virtual environment activation script not found at $ACTIVATE_SCRIPT"
    exit 1
fi

# Verify virtual environment is active
if [ -z "$VIRTUAL_ENV" ]; then
    echo "Error: Failed to activate virtual environment"
    exit 1
fi

# Install MCP SDK using uv
echo -e "\nInstalling MCP Python SDK..."
uv pip install -U 'mcp[cli]'
echo "MCP SDK installed successfully"

# Move to workspace directory before building
cd $WORKSPACE_DIR

# Build the release binary
echo -e "\nBuilding release binary..."
cargo build --release -p terraphim_mcp_server
echo "Build complete!"

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Build may have failed."
    exit 1
fi

echo -e "\nBuilt binary: $BINARY_PATH"

# Set environment variables
export TERRAPHIM_LOG_DIR="$LOG_DIR"
export RUST_LOG="info"
export RUST_BACKTRACE=1
export TERRAPHIM_CONFIG="$CONFIG_PATH"
export TERRAPHIM_FIXTURES_DIR="$FIXTURES_DIR"

# Make config file readable
chmod +r "$CONFIG_PATH"

# Print environment info
echo -e "\nTest environment:"
echo "TERRAPHIM_LOG_DIR=$TERRAPHIM_LOG_DIR"
echo "RUST_LOG=$RUST_LOG"
echo "TERRAPHIM_CONFIG=$TERRAPHIM_CONFIG"
echo "TERRAPHIM_FIXTURES_DIR=$TERRAPHIM_FIXTURES_DIR"

# Go back to the script directory
cd $SCRIPT_DIR

# Run the Python test script
echo -e "\nRunning end-to-end tests..."

DEBUG_FLAG=""
if [ "$DEBUG" = true ]; then
    DEBUG_FLAG="--debug"
fi

# Check which Python command to use
if python --version >/dev/null 2>&1; then
    PYTHON_CMD="python"
else
    PYTHON_CMD="python3"
fi

$PYTHON_CMD mcp_e2e_test.py --binary "$BINARY_PATH" $DEBUG_FLAG --expect-data

# Get the exit code
TEST_RESULT=$?

# Check the test result
if [ $TEST_RESULT -eq 0 ]; then
    echo -e "\n✅ All tests passed successfully!"
else
    echo -e "\n❌ Some tests failed. Check the output above for details."
fi

# Print log locations
echo -e "\nLog files are available at: $LOG_DIR"

# Deactivate virtual environment
if type deactivate >/dev/null 2>&1; then
    deactivate
    echo "Deactivated virtual environment"
else 
    echo "Note: Could not deactivate virtual environment (function not available)"
fi

exit $TEST_RESULT 