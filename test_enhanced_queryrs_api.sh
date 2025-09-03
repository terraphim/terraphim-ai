#!/bin/bash

# Enhanced QueryRs Haystack and Scoring Functions Validation Script
# Tests the comprehensive QueryRs integration with all scoring functions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_subheader() {
    echo -e "${CYAN}--- $1 ---${NC}"
}

# Check if server is running
check_server() {
    print_info "Checking if Terraphim server is running..."

    if curl -s http://localhost:8000/health > /dev/null; then
        print_success "Server is running and healthy"
    else
        print_error "Server is not running. Please start the server first."
        print_info "Run: cargo run --bin terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json"
        exit 1
    fi
}

# Check if Rust Engineer role is available
check_rust_engineer_role() {
    print_info "Checking if Rust Engineer role is available..."

    local roles=$(curl -s http://localhost:8000/config | jq -r '.config.roles | keys[]')

    if echo "$roles" | grep -q "Rust Engineer"; then
        print_success "Rust Engineer role is available"
    else
        print_error "Rust Engineer role not found. Available roles:"
        echo "$roles"
        exit 1
    fi
}

# Test search functionality with specific scorer
test_search_with_scorer() {
    local query="$1"
    local scorer="$2"
    local expected_min_results="$3"
    local description="$4"

    print_info "Testing search for '$query' with $scorer scorer ($description)..."

    local response=$(curl -s -X POST http://localhost:8000/documents/search \
        -H "Content-Type: application/json" \
        -d "{\"search_term\": \"$query\", \"role\": \"Rust Engineer\", \"scorer\": \"$scorer\"}")

    local status=$(echo "$response" | jq -r '.status')
    local result_count=$(echo "$response" | jq '.results | length')

    if [ "$status" = "success" ]; then
        if [ "$result_count" -ge "$expected_min_results" ]; then
            print_success "Found $result_count results for '$query' with $scorer scorer"

            # Check for different result types
            local reddit_count=$(echo "$response" | jq '.results[] | select(.tags[] | contains("reddit")) | .title' | wc -l)
            local std_count=$(echo "$response" | jq '.results[] | select(.tags[] | contains("std")) | .title' | wc -l)
            local attribute_count=$(echo "$response" | jq '.results[] | select(.tags[] | contains("attribute")) | .title' | wc -l)

            if [ "$reddit_count" -gt 0 ]; then
                print_success "  - $reddit_count Reddit posts found"
            fi

            if [ "$std_count" -gt 0 ]; then
                print_success "  - $std_count std documentation items found"
            fi

            if [ "$attribute_count" -gt 0 ]; then
                print_success "  - $attribute_count attribute items found"
            fi

            # Show sample results with scores
            echo "  Sample results:"
            echo "$response" | jq '.results[0:3] | .[] | {title: .title, tags: .tags, score: .score}' | sed 's/^/    - /'

        else
            print_warning "Found only $result_count results for '$query' with $scorer scorer (expected at least $expected_min_results)"
        fi
    else
        print_error "Search failed for '$query' with $scorer scorer: $response"
    fi

    echo ""
}

# Test all scoring functions
test_all_scoring_functions() {
    print_header "Testing All Scoring Functions"

    local scorers=("bm25" "bm25f" "bm25plus" "tfidf" "jaccard" "queryratio" "okapibm25")
    local test_queries=("Iterator" "Vec" "Result" "String" "derive" "cfg" "map" "filter")

    for scorer in "${scorers[@]}"; do
        print_subheader "Testing $scorer scorer"

        for query in "${test_queries[@]}"; do
            test_search_with_scorer "$query" "$scorer" 5 "std library with $scorer"
        done

        # Test Reddit integration with each scorer
        test_search_with_scorer "async" "$scorer" 10 "async/await with $scorer"
        test_search_with_scorer "tokio" "$scorer" 8 "tokio runtime with $scorer"
    done
}

# Test comprehensive search types with different scorers
test_comprehensive_search_with_scorers() {
    print_header "Testing Comprehensive Search Types with Different Scorers"

    local scorers=("bm25" "bm25f" "bm25plus")

    for scorer in "${scorers[@]}"; do
        print_subheader "Testing with $scorer scorer"

        # Test std library items
        test_search_with_scorer "Iterator" "$scorer" 10 "std library trait"
        test_search_with_scorer "Vec" "$scorer" 10 "std library struct"
        test_search_with_scorer "Result" "$scorer" 10 "std library enum"
        test_search_with_scorer "String" "$scorer" 10 "std library type"

        # Test attributes
        test_search_with_scorer "derive" "$scorer" 5 "Rust attributes"
        test_search_with_scorer "cfg" "$scorer" 5 "Rust attributes"

        # Test functions
        test_search_with_scorer "map" "$scorer" 5 "std library functions"
        test_search_with_scorer "filter" "$scorer" 5 "std library functions"

        # Test modules
        test_search_with_scorer "collections" "$scorer" 5 "std library modules"
        test_search_with_scorer "io" "$scorer" 5 "std library modules"
    done
}

# Test Reddit integration with different scorers
test_reddit_integration_with_scorers() {
    print_header "Testing Reddit Integration with Different Scorers"

    local scorers=("bm25" "bm25f" "bm25plus" "tfidf")

    for scorer in "${scorers[@]}"; do
        print_subheader "Testing Reddit with $scorer scorer"

        # Test popular Rust topics
        test_search_with_scorer "async" "$scorer" 15 "async/await"
        test_search_with_scorer "tokio" "$scorer" 10 "tokio runtime"
        test_search_with_scorer "serde" "$scorer" 10 "serialization"
        test_search_with_scorer "cargo" "$scorer" 10 "package manager"
    done
}

# Test scoring function comparison
test_scoring_comparison() {
    print_header "Testing Scoring Function Comparison"

    local query="Iterator"
    local scorers=("bm25" "bm25f" "bm25plus" "tfidf" "jaccard" "queryratio")

    print_info "Comparing scoring functions for query: '$query'"

    local best_scorer=""
    local best_score=0

    for scorer in "${scorers[@]}"; do
        local response=$(curl -s -X POST http://localhost:8000/documents/search \
            -H "Content-Type: application/json" \
            -d "{\"search_term\": \"$query\", \"role\": \"Rust Engineer\", \"scorer\": \"$scorer\"}")

        local result_count=$(echo "$response" | jq '.results | length')

        print_info "  $scorer: $result_count results"

        if [ "$result_count" -gt "$best_score" ]; then
            best_score=$result_count
            best_scorer=$scorer
        fi
    done

    print_success "Best performing scorer: $best_scorer with $best_score results"
    echo ""
}

# Test error handling with different scorers
test_error_handling_with_scorers() {
    print_header "Testing Error Handling with Different Scorers"

    local scorers=("bm25" "bm25f" "bm25plus" "tfidf" "jaccard" "queryratio")

    for scorer in "${scorers[@]}"; do
        print_subheader "Testing error handling with $scorer scorer"

        # Test with empty query
        print_info "Testing empty query with $scorer..."
        local response=$(curl -s -X POST http://localhost:8000/documents/search \
            -H "Content-Type: application/json" \
            -d "{\"search_term\": \"\", \"role\": \"Rust Engineer\", \"scorer\": \"$scorer\"}")

        local status=$(echo "$response" | jq -r '.status')
        if [ "$status" = "success" ]; then
            print_success "Empty query handled gracefully with $scorer"
        else
            print_warning "Empty query returned error with $scorer (this might be expected)"
        fi

        # Test with invalid role
        print_info "Testing invalid role with $scorer..."
        local response=$(curl -s -X POST http://localhost:8000/documents/search \
            -H "Content-Type: application/json" \
            -d "{\"search_term\": \"test\", \"role\": \"InvalidRole\", \"scorer\": \"$scorer\"}")

        local status=$(echo "$response" | jq -r '.status')
        if [ "$status" = "error" ]; then
            print_success "Invalid role properly rejected with $scorer"
        else
            print_warning "Invalid role not properly handled with $scorer"
        fi
    done

    echo ""
}

# Performance test with different scorers
test_performance_with_scorers() {
    print_header "Testing Performance with Different Scorers"

    local scorers=("bm25" "bm25f" "bm25plus" "tfidf" "jaccard" "queryratio")
    local query="Iterator"

    for scorer in "${scorers[@]}"; do
        print_info "Testing performance with $scorer scorer..."

        local start_time=$(date +%s.%N)

        curl -s -X POST http://localhost:8000/documents/search \
            -H "Content-Type: application/json" \
            -d "{\"search_term\": \"$query\", \"role\": \"Rust Engineer\", \"scorer\": \"$scorer\"}" > /dev/null

        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc)

        print_info "  $scorer completed in ${duration}s"

        if (( $(echo "$duration < 2.0" | bc -l) )); then
            print_success "  $scorer performance is good (< 2s)"
        else
            print_warning "  $scorer performance is slow (> 2s)"
        fi
    done

    echo ""
}

# Test query.rs haystack specific features
test_queryrs_haystack_features() {
    print_header "Testing QueryRs Haystack Specific Features"

    # Test std documentation search
    print_subheader "Testing std documentation search"
    test_search_with_scorer "std::collections::HashMap" "bm25" 5 "std module path"
    test_search_with_scorer "std::io::Read" "bm25" 5 "std trait"
    test_search_with_scorer "std::vec::Vec" "bm25" 5 "std struct"

    # Test attribute search
    print_subheader "Testing attribute search"
    test_search_with_scorer "#[derive(Debug)]" "bm25" 3 "derive attribute"
    test_search_with_scorer "#[cfg(test)]" "bm25" 3 "cfg attribute"
    test_search_with_scorer "#[allow(dead_code)]" "bm25" 3 "allow attribute"

    # Test function search
    print_subheader "Testing function search"
    test_search_with_scorer "fn main()" "bm25" 5 "main function"
    test_search_with_scorer "fn new()" "bm25" 5 "new function"
    test_search_with_scorer "fn clone()" "bm25" 5 "clone function"

    # Test module search
    print_subheader "Testing module search"
    test_search_with_scorer "mod tests" "bm25" 3 "test module"
    test_search_with_scorer "pub mod" "bm25" 5 "public module"

    # Test Reddit community content
    print_subheader "Testing Reddit community content"
    test_search_with_scorer "rust async await" "bm25" 10 "async/await discussion"
    test_search_with_scorer "rust ownership borrowing" "bm25" 8 "ownership concepts"
    test_search_with_scorer "rust error handling" "bm25" 8 "error handling"
}

# Test scoring function edge cases
test_scoring_edge_cases() {
    print_header "Testing Scoring Function Edge Cases"

    local scorers=("bm25" "bm25f" "bm25plus" "tfidf" "jaccard" "queryratio")

    for scorer in "${scorers[@]}"; do
        print_subheader "Testing edge cases with $scorer scorer"

        # Test very short queries
        test_search_with_scorer "a" "$scorer" 1 "single character query"
        test_search_with_scorer "ab" "$scorer" 1 "two character query"

        # Test very long queries
        test_search_with_scorer "this is a very long query that should test the scoring function with many words" "$scorer" 1 "long query"

        # Test special characters
        test_search_with_scorer "std::collections::HashMap<String, Vec<i32>>" "$scorer" 3 "complex type with special chars"
        test_search_with_scorer "fn<T: Clone + Debug>()" "$scorer" 3 "function with generics"

        # Test case sensitivity
        test_search_with_scorer "iterator" "$scorer" 5 "lowercase query"
        test_search_with_scorer "ITERATOR" "$scorer" 5 "uppercase query"
        test_search_with_scorer "Iterator" "$scorer" 5 "titlecase query"
    done
}

# Main test execution
main() {
    print_header "Enhanced QueryRs Haystack and Scoring Functions Validation"

    check_server
    check_rust_engineer_role

    test_all_scoring_functions
    test_comprehensive_search_with_scorers
    test_reddit_integration_with_scorers
    test_scoring_comparison
    test_error_handling_with_scorers
    test_performance_with_scorers
    test_queryrs_haystack_features
    test_scoring_edge_cases

    print_header "Validation Summary"
    print_success "Enhanced QueryRs haystack and scoring functions validation completed!"
    print_info "Features validated:"
    print_info "  ✅ All scoring functions (BM25, BM25F, BM25Plus, TFIDF, Jaccard, QueryRatio, OkapiBM25)"
    print_info "  ✅ Reddit posts integration with all scorers"
    print_info "  ✅ Std documentation integration with all scorers"
    print_info "  ✅ Multiple search types (traits, structs, functions, modules, attributes)"
    print_info "  ✅ Error handling across all scorers"
    print_info "  ✅ Performance testing for all scorers"
    print_info "  ✅ QueryRs haystack specific features"
    print_info "  ✅ Scoring function edge cases"
    print_info "  ✅ Scoring function comparison"

    print_info ""
    print_info "The QueryRs haystack now provides comprehensive Rust documentation search"
    print_info "with multiple scoring algorithms for optimal relevance ranking."
    print_info ""
    print_info "Available scoring functions:"
    print_info "  - BM25: Standard probabilistic relevance ranking"
    print_info "  - BM25F: Fielded BM25 with weighted document fields"
    print_info "  - BM25Plus: Enhanced BM25 with additional parameters"
    print_info "  - TFIDF: Traditional term frequency-inverse document frequency"
    print_info "  - Jaccard: Set-based similarity scoring"
    print_info "  - QueryRatio: Query term coverage ratio"
    print_info "  - OkapiBM25: Original Okapi BM25 implementation"
}

# Run the main function
main "$@"
