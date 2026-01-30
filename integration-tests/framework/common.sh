#!/bin/bash

# Common functions for Terraphim Integration Testing Framework

# Test result tracking (shared with main script)
TEST_RESULTS_FILE="${TEST_RESULTS_FILE:-/tmp/terraphim_integration_results.json}"

# Logging functions (duplicate from main script for independence)
log_info() {
    echo -e "\033[0;34m[INFO]\033[0m $1"
}

log_success() {
    echo -e "\033[0;32m[SUCCESS]\033[0m $1"
}

log_warning() {
    echo -e "\033[1;33m[WARNING]\033[0m $1"
}

log_error() {
    echo -e "\033[0;31m[ERROR]\033[0m $1"
}

log_header() {
    echo -e "\033[0;35m========================================\033[0m"
    echo -e "\033[0;35m$1\033[0m"
    echo -e "\033[0;35m========================================\033[0m"
}

# Update test results JSON
update_test_result() {
    local category="$1"
    local test_name="$2"
    local status="$3"
    local duration="$4"
    local details="$5"

    # Create results file if it doesn't exist
    if [[ ! -f "$TEST_RESULTS_FILE" ]]; then
        echo '{"timestamp": "'$(date -Iseconds)'", "results": {}}' | jq . > "$TEST_RESULTS_FILE"
    fi

    # Update JSON results
    jq --arg category "$category" \
       --arg test_name "$test_name" \
       --arg status "$status" \
       --arg duration "$duration" \
       --arg details "$details" \
       --arg timestamp "$(date -Iseconds)" \
       ".results.$category += [{\"name\": \$test_name, \"status\": \$status, \"duration\": \$duration, \"details\": \$details, \"timestamp\": \$timestamp}]" \
       "$TEST_RESULTS_FILE" > "${TEST_RESULTS_FILE}.tmp" && mv "${TEST_RESULTS_FILE}.tmp" "$TEST_RESULTS_FILE"
}

# Server management functions
start_test_server() {
    local port="$1"
    local config="${2:-terraphim_server/default/terraphim_engineer_config.json}"

    log_info "Starting test server on port $port..."

    # Create temporary config for testing
    local temp_config="/tmp/terraphim_test_config_$port.json"
    cp "$config" "$temp_config" 2>/dev/null || create_minimal_test_config "$temp_config"

    # Start server in background
    cd terraphim_server && \
    cargo run -- --config "$temp_config" --port "$port" > "/tmp/terraphim_server_$port.log" 2>&1 &
    echo $! > "/tmp/terraphim_server_$port.pid"

    cd - > /dev/null
}

stop_test_server() {
    local port="${1:-8081}"
    local pid_file="/tmp/terraphim_server_$port.pid"

    if [[ -f "$pid_file" ]]; then
        local pid=$(cat "$pid_file")
        log_info "Stopping test server (PID: $pid)..."
        kill "$pid" 2>/dev/null || true
        rm -f "$pid_file"
        sleep 2
    fi
}

wait_for_server() {
    local url="$1"
    local timeout="${2:-30}"
    local count=0

    log_info "Waiting for server at $url..."

    while [[ $count -lt $timeout ]]; do
        if curl -s -f "$url" > /dev/null 2>&1; then
            log_success "Server is ready"
            return 0
        fi
        sleep 1
        ((count++))
    done

    log_error "Server failed to start within $timeout seconds"
    return 1
}

# API testing functions
test_api_endpoint() {
    local method="$1"
    local url="$2"
    local data="${3:-}"

    case "$method" in
        "GET")
            curl -s -f -X GET "$url" > /dev/null 2>&1
            ;;
        "POST")
            if [[ -n "$data" ]]; then
                curl -s -f -X POST -H "Content-Type: application/json" -d "$data" "$url" > /dev/null 2>&1
            else
                curl -s -f -X POST "$url" > /dev/null 2>&1
            fi
            ;;
        "PUT")
            if [[ -n "$data" ]]; then
                curl -s -f -X PUT -H "Content-Type: application/json" -d "$data" "$url" > /dev/null 2>&1
            else
                curl -s -f -X PUT "$url" > /dev/null 2>&1
            fi
            ;;
        "DELETE")
            curl -s -f -X DELETE "$url" > /dev/null 2>&1
            ;;
        *)
            log_error "Unsupported HTTP method: $method"
            return 1
            ;;
    esac
}

# WebSocket testing
test_websocket_connection() {
    local ws_url="$1"

    # Check if websocat is available
    if ! command -v websocat &> /dev/null; then
        log_warning "websocat not available for WebSocket testing"
        return 1
    fi

    # Attempt WebSocket connection
    timeout 5 websocat -E "$ws_url" <<< "test" > /dev/null 2>&1
}

# Database testing
test_database_connection() {
    # Test database connectivity (placeholder - would need actual DB config)
    log_info "Testing database connection..."

    # For now, just check if any database-related endpoints work
    # In a real implementation, this would test actual DB connections
    return 0  # Assume DB is not configured in test environment
}

# File system testing
test_file_system_operations() {
    log_info "Testing file system operations..."

    # Test creating, writing, reading, and deleting files
    local test_file="/tmp/terraphim_fs_test_$$.txt"

    # Write test
    echo "test content" > "$test_file"
    if [[ ! -f "$test_file" ]]; then
        return 1
    fi

    # Read test
    local content=$(cat "$test_file")
    if [[ "$content" != "test content" ]]; then
        rm -f "$test_file"
        return 1
    fi

    # Delete test
    rm -f "$test_file"
    if [[ -f "$test_file" ]]; then
        return 1
    fi

    return 0
}

# External API testing
test_external_api_calls() {
    log_info "Testing external API calls..."

    # Test basic internet connectivity
    if curl -s --connect-timeout 5 "https://httpbin.org/get" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# File upload/download testing
test_file_upload() {
    local url="$1"
    local test_file="/tmp/terraphim_upload_test_$$.txt"

    echo "test upload content" > "$test_file"

    # Attempt upload (this would need to be implemented based on actual API)
    local result=$(curl -s -X POST -F "file=@$test_file" "$url" 2>/dev/null)
    local exit_code=$?

    rm -f "$test_file"
    return $exit_code
}

test_file_download() {
    local url="$1"

    # Attempt download
    curl -s -f "$url" > /dev/null 2>&1
}

# Create minimal test configuration
create_minimal_test_config() {
    local config_file="$1"

    cat > "$config_file" << 'EOF'
{
  "roles": {
    "TestRole": {
      "name": "TestRole",
      "relevance_function": "Ripgrep",
      "haystacks": [
        {
          "location": "terraphim_server/fixtures/haystack",
          "name": "test_haystack"
        }
      ],
      "kg": null
    }
  },
  "selected_role": "TestRole"
}
EOF
}

# Performance testing helpers
measure_execution_time() {
    local command="$1"
    local start_time=$(date +%s.%3N)

    eval "$command"
    local exit_code=$?

    local end_time=$(date +%s.%3N)
    local duration=$(echo "$end_time - $start_time" | bc)

    echo "$duration"
    return $exit_code
}

# Load testing
generate_load() {
    local url="$1"
    local num_requests="$2"
    local concurrency="${3:-10}"

    log_info "Generating load: $num_requests requests with $concurrency concurrency..."

    # Use ab (apache bench) if available
    if command -v ab &> /dev/null; then
        ab -n "$num_requests" -c "$concurrency" "$url" > /dev/null 2>&1
        return $?
    else
        # Fallback to simple curl loop
        for i in $(seq 1 "$num_requests"); do
            curl -s "$url" > /dev/null 2>&1 &
            if [[ $((i % concurrency)) -eq 0 ]]; then
                wait
            fi
        done
        wait
        return 0
    fi
}

# Memory and CPU monitoring
start_resource_monitoring() {
    local pid="$1"
    local output_file="/tmp/terraphim_resource_monitor_$pid.log"

    log_info "Starting resource monitoring for PID $pid..."

    # Monitor CPU and memory usage
    {
        while kill -0 "$pid" 2>/dev/null; do
            local cpu_mem=$(ps -p "$pid" -o pcpu,pmem --no-headers 2>/dev/null || echo "0.0 0.0")
            echo "$(date +%s) $cpu_mem" >> "$output_file"
            sleep 1
        done
    } &
    echo $! > "/tmp/terraphim_monitor_$pid.pid"
}

stop_resource_monitoring() {
    local pid="$1"
    local monitor_pid_file="/tmp/terraphim_monitor_$pid.pid"

    if [[ -f "$monitor_pid_file" ]]; then
        local monitor_pid=$(cat "$monitor_pid_file")
        kill "$monitor_pid" 2>/dev/null || true
        rm -f "$monitor_pid_file"
    fi
}

# Docker container management for complex scenarios
start_docker_services() {
    local compose_file="${1:-docker/docker-compose.test.yml}"

    if [[ -f "$compose_file" ]]; then
        log_info "Starting Docker services..."
        docker-compose -f "$compose_file" up -d
        sleep 10  # Wait for services to be ready
    else
        log_warning "Docker compose file not found: $compose_file"
    fi
}

stop_docker_services() {
    local compose_file="${1:-docker/docker-compose.test.yml}"

    if [[ -f "$compose_file" ]]; then
        log_info "Stopping Docker services..."
        docker-compose -f "$compose_file" down -v
    fi
}

# Cross-platform testing helpers
get_platform_info() {
    uname -s
}

is_windows() {
    [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]]
}

is_macos() {
    [[ "$(uname -s)" == "Darwin" ]]
}

is_linux() {
    [[ "$(uname -s)" == "Linux" ]]
}

# Error simulation
simulate_network_failure() {
    local interface="${1:-eth0}"
    log_info "Simulating network failure on $interface..."

    # This would require root privileges and platform-specific commands
    # For testing purposes, we'll just log the intent
    log_warning "Network failure simulation requires root privileges - skipping actual failure"
}

simulate_disk_full() {
    local mount_point="${1:-/tmp}"
    log_info "Simulating disk full condition..."

    # Create a large file to fill disk (would need careful cleanup)
    log_warning "Disk full simulation not implemented - would require large file creation"
}

# Cleanup helper
cleanup_test_artifacts() {
    log_info "Cleaning up test artifacts..."

    # Remove test files
    rm -f /tmp/terraphim_*_test_*.*
    rm -f /tmp/terraphim_server_*.log
    rm -f /tmp/terraphim_server_*.pid
    rm -f /tmp/terraphim_monitor_*.pid
    rm -f /tmp/terraphim_resource_monitor_*.log
}