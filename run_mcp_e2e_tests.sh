#!/usr/bin/env bash
set -euo pipefail

# ANSI color codes for output formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="${SCRIPT_DIR}"
BINARY_PATH="${WORKSPACE_DIR}/target/release/terraphim_mcp_server"
FIXTURES_DIR="${SCRIPT_DIR}/terraphim_server/fixtures"

# Default values
DEBUG_MODE=false
EXPECT_DATA=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
    --debug)
      DEBUG_MODE=true
      shift
      ;;
    --expect-data)
      EXPECT_DATA=true
      shift
      ;;
    *)
      echo -e "${RED}Unknown option: $key${RESET}"
      echo "Usage: $0 [--debug] [--expect-data]"
      exit 1
      ;;
  esac
done

echo -e "${BLUE}${BOLD}Terraphim MCP Server E2E Test Runner${RESET}"
echo -e "${BLUE}Script directory: ${SCRIPT_DIR}${RESET}"
echo -e "${BLUE}Workspace directory: ${WORKSPACE_DIR}${RESET}"
echo -e "${BLUE}Fixtures directory: ${FIXTURES_DIR}${RESET}"

# Ensure fixtures directory exists
if [[ ! -d "${FIXTURES_DIR}/haystack" ]]; then
    echo -e "${YELLOW}Creating fixtures directory...${RESET}"
    mkdir -p "${FIXTURES_DIR}/haystack"
fi

# Count the number of fixture files
FIXTURE_COUNT=$(find "${FIXTURES_DIR}/haystack" -type f | wc -l | tr -d ' ')
echo -e "${BLUE}Found ${FIXTURE_COUNT} test document(s) in fixtures directory${RESET}"

# Setup Python environment with uv
VENV_DIR="${SCRIPT_DIR}/.venv"

# Check if uv is installed, and install it if needed
if ! command -v uv &> /dev/null; then
    echo -e "${YELLOW}uv package manager not found, installing...${RESET}"

    if command -v curl &> /dev/null; then
        curl -LsSf https://astral.sh/uv/install.sh | sh
    elif command -v wget &> /dev/null; then
        wget -qO- https://astral.sh/uv/install.sh | sh
    else
        echo -e "${RED}Error: Neither curl nor wget is available to install uv${RESET}"
        exit 1
    fi

    # Update PATH to include uv
    if [[ -f "${HOME}/.cargo/env" ]]; then
        source "${HOME}/.cargo/env"
    else
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi

    echo -e "${GREEN}uv package manager installed${RESET}"
fi

# Check if Python 3 is installed
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}Error: Python 3 is required but not found${RESET}"
    exit 1
fi

# Create logs directory if it doesn't exist
LOGS_DIR="${SCRIPT_DIR}/logs"
mkdir -p "${LOGS_DIR}"

# Clean up previous run
echo -e "${YELLOW}Cleaning up previous run...${RESET}"
rm -rf /tmp/sled/db
pkill -f terraphim_mcp_server || true

# Make sure fixtures directory exists
mkdir -p "${FIXTURES_DIR}/haystack"

# Set environment variables for the test
export TERRAPHIM_LOG_DIR="${LOGS_DIR}"
export TERRAPHIM_FIXTURES_DIR="${FIXTURES_DIR}"
export TERRAPHIM_CONFIG="${FIXTURES_DIR}/test_config.json"
export RUST_LOG=info
export RUST_BACKTRACE=1

# Print environment variables
echo -e "${BLUE}Environment variables:${RESET}"
echo -e "${BLUE}TERRAPHIM_LOG_DIR=${TERRAPHIM_LOG_DIR}${RESET}"
echo -e "${BLUE}TERRAPHIM_FIXTURES_DIR=${TERRAPHIM_FIXTURES_DIR}${RESET}"
echo -e "${BLUE}TERRAPHIM_CONFIG=${TERRAPHIM_CONFIG}${RESET}"
echo -e "${BLUE}RUST_LOG=${RUST_LOG}${RESET}"

# Setup virtual environment if it doesn't exist
if [[ ! -d "${VENV_DIR}" ]]; then
    echo -e "${YELLOW}Creating Python virtual environment...${RESET}"
    uv venv "${VENV_DIR}"
    echo -e "${GREEN}Virtual environment created at: ${VENV_DIR}${RESET}"
else
    echo -e "${GREEN}Using existing virtual environment at: ${VENV_DIR}${RESET}"
fi

# Function to activate the virtual environment
activate_venv() {
    if [[ -f "${VENV_DIR}/bin/activate" ]]; then
        source "${VENV_DIR}/bin/activate"
    elif [[ -f "${VENV_DIR}/Scripts/activate" ]]; then
        source "${VENV_DIR}/Scripts/activate"
    else
        echo -e "${RED}Error: Could not find activation script in virtual environment${RESET}"
        exit 1
    fi
}

# Activate virtual environment
activate_venv

# Install MCP Python SDK in the virtual environment
echo -e "${YELLOW}Installing MCP Python SDK...${RESET}"
uv pip install -U 'mcp[cli,asyncio]'
echo -e "${GREEN}MCP Python SDK installed${RESET}"

# Build Terraphim MCP Server binary
echo -e "${YELLOW}Building Terraphim MCP Server release binary...${RESET}"
(cd "${WORKSPACE_DIR}" && cargo build --release -p terraphim_mcp_server)
echo -e "${GREEN}Binary built at: ${BINARY_PATH}${RESET}"

# Ensure the binary is executable
chmod +x "${BINARY_PATH}"

# Run the tests
echo -e "${YELLOW}Running end-to-end tests...${RESET}"

PYTHON_TEST_ARGS=("--binary" "${BINARY_PATH}")

if [[ "${DEBUG_MODE}" == "true" ]]; then
    PYTHON_TEST_ARGS+=("--debug")
    echo -e "${YELLOW}Debug mode enabled${RESET}"
fi

if [[ "${EXPECT_DATA}" == "true" ]]; then
    PYTHON_TEST_ARGS+=("--expect-data")
    echo -e "${YELLOW}Expecting data to be available (strict mode)${RESET}"
fi

python3 -u "${SCRIPT_DIR}/mcp_e2e_test.py" "${PYTHON_TEST_ARGS[@]}"
TEST_EXIT_CODE=$?

# Check test result
if [[ ${TEST_EXIT_CODE} -eq 0 ]]; then
    echo -e "${GREEN}${BOLD}✅ All tests passed!${RESET}"
else
    echo -e "${RED}${BOLD}❌ Tests failed with exit code: ${TEST_EXIT_CODE}${RESET}"
fi

# Deactivate virtual environment
deactivate 2>/dev/null || true

exit ${TEST_EXIT_CODE}
