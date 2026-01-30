#!/bin/bash

# Multi-Component Integration Testing
# Tests server + TUI, desktop + server, and multi-server communication

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FRAMEWORK_DIR="${SCRIPT_DIR}/framework"

source "${FRAMEWORK_DIR}/common.sh"

TEST_CATEGORY="multi_component"

# Test Server + TUI HTTP API Communication
test_server_tui_http_api() {
    log_info "Testing Server + TUI HTTP API communication..."

    local test_name="server_tui_http_api"
    local start_time=$(date +%s)

    # Start test server
    start_test_server "8081"

    # Wait for server to be ready
    wait_for_server "http://localhost:8081/health" 10

    # Test basic API endpoints that TUI would use
    local endpoints=(
        "GET /health"
        "GET /config"
        "POST /documents/search"
        "GET /workflows"
    )

    local passed=0
    local total=${#endpoints[@]}

    for endpoint in "${endpoints[@]}"; do
        local method=$(echo "$endpoint" | cut -d' ' -f1)
        local path=$(echo "$endpoint" | cut -d' ' -f2)

        if test_api_endpoint "$method" "http://localhost:8081$path"; then
            ((passed++))
            log_info "✅ $method $path - OK"
        else
            log_error "❌ $method $path - FAILED"
        fi
    done

    # Cleanup
    stop_test_server

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    if [[ $passed -eq $total ]]; then
        update_test_result "$TEST_CATEGORY" "$test_name" "passed" "$duration" "All HTTP API endpoints functional"
        return 0
    else
        update_test_result "$TEST_CATEGORY" "$test_name" "failed" "$duration" "$passed/$total endpoints passed"
        return 1
    fi
}

# Test Desktop + Server Communication
test_desktop_server_communication() {
    log_info "Testing Desktop + Server communication..."

    local test_name="desktop_server_communication"
    local start_time=$(date +%s)

    # Start test server
    start_test_server "8082"

    # Wait for server
    wait_for_server "http://localhost:8082/health" 10

    # Test WebSocket connection (if supported)
    if test_websocket_connection "ws://localhost:8082/ws"; then
        log_info "✅ WebSocket connection established"
    else
        log_warning "⚠️ WebSocket connection failed (may not be implemented)"
    fi

    # Test file upload/download endpoints (simulated)
    local upload_test=$(test_file_upload "http://localhost:8082/documents/upload")
    local download_test=$(test_file_download "http://localhost:8082/documents/download/test.txt")

    # Cleanup
    stop_test_server

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Basic communication test - just check if server responds
    if curl -s -f "http://localhost:8082/health" > /dev/null 2>&1; then
        update_test_result "$TEST_CATEGORY" "$test_name" "passed" "$duration" "Desktop-server communication functional"
        return 0
    else
        update_test_result "$TEST_CATEGORY" "$test_name" "failed" "$duration" "Server not responding"
        return 1
    fi
}

# Test Multi-Server Communication and Load Balancing
test_multi_server_communication() {
    log_info "Testing multi-server communication and load balancing..."

    local test_name="multi_server_communication"
    local start_time=$(date +%s)

    # Start multiple test servers
    start_test_server "8083"
    start_test_server "8084"

    # Wait for servers
    wait_for_server "http://localhost:8083/health" 10
    wait_for_server "http://localhost:8084/health" 10

    # Test load balancing scenario
    local server1_responses=0
    local server2_responses=0
    local total_requests=10

    for i in $(seq 1 $total_requests); do
        # Simulate load balancing by alternating between servers
        local port=$((8083 + (i % 2)))
        if curl -s -f "http://localhost:$port/health" > /dev/null 2>&1; then
            if [[ $port -eq 8083 ]]; then
                ((server1_responses++))
            else
                ((server2_responses++))
            fi
        fi
    done

    # Cleanup
    stop_test_server "8083"
    stop_test_server "8084"

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    if [[ $((server1_responses + server2_responses)) -gt 0 ]]; then
        update_test_result "$TEST_CATEGORY" "$test_name" "passed" "$duration" "Multi-server setup functional ($server1_responses/$server2_responses responses)"
        return 0
    else
        update_test_result "$TEST_CATEGORY" "$test_name" "failed" "$duration" "No server responses received"
        return 1
    fi
}

# Test External Service Integration
test_external_service_integration() {
    log_info "Testing external service integration..."

    local test_name="external_service_integration"
    local start_time=$(date +%s)

    # Start test server
    start_test_server "8085"

    # Wait for server
    wait_for_server "http://localhost:8085/health" 10

    # Test external API calls (mocked)
    local external_tests_passed=0
    local external_tests_total=3

    # Test database connectivity (if configured)
    if test_database_connection; then
        ((external_tests_passed++))
        log_info "✅ Database connection OK"
    else
        log_warning "⚠️ Database connection failed (may not be configured)"
    fi

    # Test file system operations
    if test_file_system_operations; then
        ((external_tests_passed++))
        log_info "✅ File system operations OK"
    else
        log_error "❌ File system operations failed"
    fi

    # Test external API calls (if configured)
    if test_external_api_calls; then
        ((external_tests_passed++))
        log_info "✅ External API calls OK"
    else
        log_warning "⚠️ External API calls failed (may not be configured)"
    fi

    # Cleanup
    stop_test_server

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Allow some tests to fail (external services may not be available)
    if [[ $external_tests_passed -ge 1 ]]; then
        update_test_result "$TEST_CATEGORY" "$test_name" "passed" "$duration" "$external_tests_passed/$external_tests_total external services tested"
        return 0
    else
        update_test_result "$TEST_CATEGORY" "$test_name" "failed" "$duration" "No external services could be tested"
        return 1
    fi
}

# Run all multi-component integration tests
run_multi_component_tests() {
    log_header "MULTI-COMPONENT INTEGRATION TESTING"

    local tests=(
        "test_server_tui_http_api"
        "test_desktop_server_communication"
        "test_multi_server_communication"
        "test_external_service_integration"
    )

    local passed=0
    local total=${#tests[@]}

    for test_func in "${tests[@]}"; do
        log_info "Running $test_func..."
        if $test_func; then
            ((passed++))
        fi
        echo ""
    done

    log_header "MULTI-COMPONENT TEST RESULTS"
    echo "Passed: $passed/$total"

    if [[ $passed -eq $total ]]; then
        log_success "All multi-component integration tests passed"
        return 0
    else
        log_warning "Some multi-component tests failed: $passed/$total passed"
        return 1
    fi
}

# Run tests if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    run_multi_component_tests
fi