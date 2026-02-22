#!/bin/bash

# Performance Testing for 4-Scenario Routing
# Tests response times and routing performance for each scenario

PROXY_URL="http://localhost:3456"
API_KEY="sk-or-v1-d3a7b4c6a7b9e2f1a9c5e7f2b8d0c3e5"
RESULTS_FILE="performance_results_$(date +%Y%m%d_%H%M%S).csv"

echo "🚀 Performance Testing 4-Scenario Routing"
echo "=========================================="
echo ""

# Initialize results file
echo "Scenario,RequestID,StatusCode,ResponseTime(ms),TokensUsed,RoutingDecision" > "$RESULTS_FILE"

# Function to test scenario performance
test_scenario_performance() {
    local scenario_name="$1"
    local test_prompt="$2"
    local expected_route="$3"
    local iterations="${4:-3}"

    echo "📊 Testing $scenario_name Performance ($iterations iterations)"
    echo "   Prompt: '$test_prompt'"
    echo "   Expected Route: $expected_route"
    echo ""

    for i in $(seq 1 $iterations); do
        echo "   Test $i/$iterations:"

        # Record start time
        start_time=$(date +%s%3N)

        # Make request
        response=$(curl -s -w "%{http_code},%{time_total}" \
            -X POST "$PROXY_URL/v1/chat/completions" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $API_KEY" \
            -d "{
                \"model\": \"auto\",
                \"messages\": [{\"role\": \"user\", \"content\": \"$test_prompt\"}],
                \"max_tokens\": 20
            }" 2>/dev/null)

        # Parse response
        status_code=$(echo "$response" | tail -c 100 | grep -o '^[0-9]*' | tail -1)
        response_time=$(echo "$response" | tail -c 100 | grep -o '[0-9]*\.[0-9]*$' | tail -1)
        response_time_ms=$(echo "$response_time * 1000" | bc -l | cut -d. -f1)

        # Extract JSON part (everything before the status_code,time_total)
        json_response=$(echo "$response" | sed 's/,[0-9]*,[0-9]*\.[0-9]*$//')

        # Check for token usage
        tokens_used="0"
        if echo "$json_response" | grep -q '"usage"'; then
            tokens_used=$(echo "$json_response" | jq -r '.usage.total_tokens // 0' 2>/dev/null || echo "0")
        fi

        # Determine routing decision based on response
        routing_decision="unknown"
        if echo "$json_response" | grep -q "claude-sonnet-4.5"; then
            routing_decision="fast_expensive"
        elif echo "$json_response" | grep -q "deepseek-v3.1-terminus"; then
            routing_decision="intelligent"
        elif echo "$json_response" | grep -q "claude-3.5-sonnet"; then
            routing_decision="balanced"
        elif echo "$json_response" | grep -q "deepseek-chat"; then
            routing_decision="slow_cheap"
        fi

        # Record results
        request_id="${scenario_name}_$(date +%s)_${i}"
        echo "$scenario_name,$request_id,$status_code,$response_time_ms,$tokens_used,$routing_decision" >> "$RESULTS_FILE"

        echo "     Status: $status_code, Time: ${response_time_ms}ms, Tokens: $tokens_used, Route: $routing_decision"

        # Check if response was successful
        if [[ "$status_code" == "200" ]]; then
            echo "     ✅ Success"
        elif [[ "$status_code" == "401" ]]; then
            echo "     ⚠️  Auth error (expected for demo keys)"
        elif [[ "$status_code" == "500" ]]; then
            echo "     ⚠️  Server error"
        else
            echo "     ❌ Error: $status_code"
        fi

        sleep 0.5  # Brief pause between requests
    done
    echo ""
}

echo "🔍 Checking proxy health..."
health_response=$(curl -s "$PROXY_URL/health" 2>/dev/null)
if echo "$health_response" | grep -q "healthy"; then
    echo "✅ Proxy is healthy and ready"
else
    echo "❌ Proxy is not responding - exiting"
    exit 1
fi
echo ""

# Test each scenario
echo "🎯 Starting Performance Tests..."
echo ""

# Test 1: Fast & Expensive Routing
test_scenario_performance "Fast_Expensive" \
    "critical production issue needs urgent premium tier resolution with maximum speed" \
    "fast_expensive" \
    3

# Test 2: Intelligent Routing
test_scenario_performance "Intelligent" \
    "I need to think through this architecture design step by step and plan the implementation carefully" \
    "intelligent" \
    3

# Test 3: Balanced Routing
test_scenario_performance "Balanced" \
    "Help me understand this regular code pattern in a practical sensible way" \
    "balanced" \
    3

# Test 4: Slow & Cheap Routing
test_scenario_performance "Slow_Cheap" \
    "Use the cheapest budget-friendly approach for this background batch processing" \
    "slow_cheap" \
    3

echo "📈 Performance Test Results Summary"
echo "================================="
echo ""
echo "Results saved to: $RESULTS_FILE"
echo ""

# Display summary statistics
echo "Scenario Performance Summary:"
echo "----------------------------"
awk -F',' 'NR>1 {
    scenario[$1]++;
    total_time[$1] += $4;
    count[$1]++
    if($4>max_time[$1] || max_time[$1]=="") max_time[$1]=$4;
    if($4<min_time[$1] || min_time[$1]=="") min_time[$1]=$4;
} END {
    for(s in scenario) {
        avg_time = total_time[s]/count[s];
        printf "%-15s: %2d tests, Avg: %4dms, Min: %4dms, Max: %4dms\n",
               s, scenario[s], avg_time, min_time[s], max_time[s];
    }
}' "$RESULTS_FILE"

echo ""
echo "🔍 Routing Decision Analysis:"
echo "---------------------------"
awk -F',' 'NR>1 {
    routes[$6]++;
    total++
} END {
    for(r in routes) {
        printf "%-15s: %2d requests (%.1f%%)\n", r, routes[r], (routes[r]/total)*100;
    }
}' "$RESULTS_FILE"

echo ""
echo "📊 Response Time Distribution:"
echo "-----------------------------"
awk -F',' 'NR>1 {
    if($4<500) fast++;
    else if($4<1000) medium++;
    else slow++;
    total++
} END {
    printf "Fast (<500ms):   %2d (%.1f%%)\n", fast, (fast/total)*100;
    printf "Medium (500-1s): %2d (%.1f%%)\n", medium, (medium/total)*100;
    printf "Slow (>1s):     %2d (%.1f%%)\n", slow, (slow/total)*100;
}' "$RESULTS_FILE"

echo ""
echo "🎯 Performance Testing Completed!"
echo ""
echo "📋 To analyze detailed results:"
echo "   cat $RESULTS_FILE"
echo "   or import into spreadsheet for further analysis"
echo ""
echo "💡 Performance Tips:"
echo "   - Fast & Expensive should have lowest latency"
echo "   - Intelligent may have higher latency due to complex routing"
echo "   - Balanced should provide consistent performance"
echo "   - Slow & Cheap may have higher latency but lowest cost"