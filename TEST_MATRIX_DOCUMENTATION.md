# Comprehensive Scoring Function x Haystack Test Matrix

## Overview

This document describes the comprehensive test matrix framework that validates every combination of scoring functions and haystacks in the Terraphim system.

## What Was Created

### 🎯 Test Matrix Framework

**Location**: `crates/terraphim_agent/tests/scoring_haystack_matrix_tests.rs`

A comprehensive testing framework that systematically tests every scoring function with every haystack type to ensure compatibility, performance, and correctness.

### 📊 Test Coverage

#### Scoring Functions (5 types)
- **TerraphimGraph** (`terraphim-graph`) - Knowledge graph-based ranking
- **TitleScorer** (`title-scorer`) - Document title-based ranking (default)
- **BM25** (`bm25`) - Standard Okapi BM25 ranking
- **BM25F** (`bm25f`) - Fielded BM25 with weighted document fields
- **BM25Plus** (`bm25plus`) - Enhanced BM25 with additional parameters

#### Haystack Types (6 types)
- **Ripgrep** - Local filesystem search
- **Atomic** - Atomic server integration
- **QueryRs** - Rust documentation search
- **ClickUp** - ClickUp API integration
- **MCP** - Model Context Protocol servers
- **Perplexity** - AI-powered web search

#### Query Scorers (10 algorithms)
For advanced TitleScorer combinations:
- **Levenshtein** - Edit distance similarity
- **Jaro** - Character transposition similarity
- **JaroWinkler** - Enhanced Jaro with prefix bonus
- **BM25/BM25F/BM25Plus** - BM25 variants within TitleScorer
- **TFIDF** - Term frequency-inverse document frequency
- **Jaccard** - Set-based similarity
- **QueryRatio** - Query term coverage ratio
- **OkapiBM25** - Original Okapi BM25 implementation

### 🧪 Test Categories

#### 1. Basic Matrix Test (`test_complete_scoring_haystack_matrix`)
- Tests all 30 basic combinations (5 scoring functions × 6 haystacks)
- Validates configuration generation and basic search functionality
- Expected success rate: 40%+ (remote services may fail)

#### 2. Priority Combinations Test (`test_priority_combinations`)
- Tests critical combinations that should always work
- Focuses on local/reliable combinations
- Expected success rate: 80%+

#### 3. Performance Comparison Test (`test_scoring_function_performance_comparison`)
- Benchmarks performance across different scoring functions
- Measures response time and results per second
- Uses Ripgrep (most reliable) for consistent testing

#### 4. Extended Matrix Test (`test_extended_matrix_with_query_scorers`)
- Tests 90+ combinations including query scorer variations
- Comprehensive validation of TitleScorer with all query algorithms
- Expected success rate: 30%+ (more combinations = more potential failures)

#### 5. Title Scorer Combinations Test (`test_title_scorer_query_combinations`)
- Focused testing of TitleScorer with specific query scoring algorithms
- Validates algorithm compatibility and performance
- Expected success rate: 50%+

### 🚀 Test Execution

#### Manual Execution
```bash
# Run individual test categories
cargo test -p terraphim_agent test_complete_scoring_haystack_matrix -- --nocapture
cargo test -p terraphim_agent test_priority_combinations -- --nocapture
cargo test -p terraphim_agent test_scoring_function_performance_comparison -- --nocapture
cargo test -p terraphim_agent test_extended_matrix_with_query_scorers -- --nocapture
cargo test -p terraphim_agent test_title_scorer_query_combinations -- --nocapture
```

#### Automated Execution Script
**Location**: `run_test_matrix.sh`

```bash
# Run all tests
./run_test_matrix.sh

# Run specific categories
./run_test_matrix.sh basic
./run_test_matrix.sh priority
./run_test_matrix.sh performance
./run_test_matrix.sh extended
./run_test_matrix.sh title-scorer
```

### 📈 Test Results Structure

Each test combination generates a `MatrixTestResult` with:
- **Configuration Success**: Whether test config was created successfully
- **Search Success**: Whether search operation completed successfully
- **Response Time**: How long the search took
- **Result Count**: Number of results returned
- **Performance Score**: Results per second metric
- **Error Details**: Specific error messages for failures

### 📋 Comprehensive Reporting

The test matrix generates detailed reports including:
- **Overall success rate** across all combinations
- **Per-scoring-function** success rates
- **Per-haystack** success rates
- **Performance rankings** (fastest combinations first)
- **Detailed failure analysis** with specific error messages

## Current Findings

### ✅ Successful Framework Creation
- Test matrix framework compiles and runs successfully
- Configuration generation works for all combinations
- Comprehensive reporting provides detailed analysis
- Performance tracking captures timing and throughput metrics

### ⚠️ Identified CLI Limitation
**Issue Discovered**: The TUI command line interface doesn't currently support a `--config` parameter.

**Error**: `error: unexpected argument '--config' found`

**Impact**:
- All test combinations currently fail due to CLI limitation
- Test framework correctly identifies this as a systematic issue
- Need to either:
  1. Add `--config` parameter support to TUI CLI, or
  2. Modify test approach to use environment variables or default config loading

### 🎯 Next Steps

1. **Add CLI Config Support**: Implement `--config` parameter in TUI CLI
2. **Alternative Test Approach**: Create tests that use environment-based configuration
3. **Integration Testing**: Once CLI is fixed, run full matrix validation
4. **Performance Baselines**: Establish performance benchmarks for each combination
5. **CI Integration**: Add matrix tests to continuous integration pipeline

## Architecture Benefits

### 🔧 Modular Design
- Each scoring function and haystack type is modeled as an enum
- Configuration generation is parameterized and extensible
- Test execution engine supports different test patterns

### 📊 Comprehensive Coverage
- Tests **30 basic combinations** (5 scoring × 6 haystacks)
- Tests **90+ extended combinations** (including query scorers)
- Validates **configuration generation**, **search execution**, and **performance**

### 🎯 Quality Assurance
- **Systematic validation** ensures no combination is missed
- **Performance tracking** identifies optimization opportunities
- **Error analysis** provides actionable debugging information
- **Success rate metrics** track system reliability

### 🚀 Extensible Framework
- Easy to add new scoring functions
- Easy to add new haystack types
- Easy to add new query scorer algorithms
- Easy to add new test patterns

## Usage Examples

### Testing a New Scoring Function
```rust
// Add to ScoringFunction enum
enum ScoringFunction {
    // ... existing functions
    NewScorer,
}

// Add configuration mapping
fn as_config_str(&self) -> &'static str {
    match self {
        // ... existing mappings
        Self::NewScorer => "new-scorer",
    }
}
```

### Testing a New Haystack Type
```rust
// Add to HaystackType enum
enum HaystackType {
    // ... existing types
    NewHaystack,
}

// Add configuration details
fn default_location(&self) -> &'static str {
    match self {
        // ... existing locations
        Self::NewHaystack => "https://api.newhaystack.com",
    }
}
```

## Conclusion

The comprehensive test matrix provides:
- **Complete validation** of all scoring function × haystack combinations
- **Performance benchmarking** across the entire system
- **Quality assurance** for configuration compatibility
- **Systematic approach** to testing that scales with new features
- **Detailed reporting** for debugging and optimization

This framework ensures that any changes to scoring functions or haystack implementations are thoroughly validated across the entire system, maintaining reliability and performance standards.
