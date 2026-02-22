# Priority-Based Routing Test Specification

## Overview

This document defines the comprehensive test specification for priority-based routing functionality in terraphim-llm-proxy. The priority system allows fast/expensive routing rules to override general "think" matcher based on priority values.

## Test Categories

### 1. Unit Tests

#### 1.1 Priority Type Tests
- **Priority Creation**: Test creating Priority with valid values (0-100)
- **Priority Bounds**: Test that values outside 0-100 range are clamped
- **Priority Comparison**: Test ordering and comparison operations
- **Priority Classification**: Test is_high(), is_medium(), is_low() methods
- **Priority Constants**: Test predefined constants (HIGH, MEDIUM, LOW, etc.)
- **Priority Serialization**: Test serde serialization/deserialization

#### 1.2 RoutingRule Tests
- **Rule Creation**: Test creating routing rules with all parameters
- **Rule Defaults**: Test default priority and values
- **Rule Validation**: Test rule field validation
- **Rule Serialization**: Test serde serialization/deserialization
- **Rule Updates**: Test touch() method for timestamp updates

#### 1.3 PatternMatch Tests
- **Pattern Creation**: Test creating pattern matches with priority
- **Weighted Score**: Test weighted score calculation (score * priority_factor)
- **Pattern Serialization**: Test serde serialization/deserialization
- **Simple Pattern**: Test simple pattern creation with defaults

#### 1.4 RoutingDecision Tests
- **Decision Creation**: Test creating routing decisions with priority
- **Decision with Rule**: Test decisions with associated rule IDs
- **Decision Defaults**: Test default decision creation
- **Decision Serialization**: Test serde serialization/deserialization

#### 1.5 RoleGraphClient Tests
- **Priority Parsing**: Test parsing priority:: directives from markdown
- **Rule Generation**: Test auto-generation of routing rules from markdown
- **Priority Mapping**: Test concept to priority mapping
- **Priority Query**: Test query_routing_priority() method
- **Rule Retrieval**: Test get_routing_rules() and get_enabled_routing_rules()

### 2. Integration Tests

#### 2.1 Markdown Configuration Tests
- **Priority Directive Parsing**: Test parsing `priority:: 90` from markdown files
- **Default Priority**: Test default priority when no directive specified
- **Invalid Priority**: Test handling of invalid priority values
- **Multiple Files**: Test priority parsing across multiple taxonomy files
- **File Updates**: Test dynamic reloading of updated priority values

#### 2.2 Router Integration Tests
- **Priority-Based Routing**: Test router using priority for decision making
- **High Priority Override**: Test high priority rules overriding lower priority ones
- **Weighted Scoring**: Test routing based on weighted scores
- **Fallback Behavior**: Test fallback when no priority rules match
- **Scenario Integration**: Test priority integration with existing scenarios

#### 2.3 Multi-Phase Routing Tests
- **Phase 1 Priority**: Test priority-based pattern matching in Phase 1
- **Phase 2 Session**: Test session-aware priority routing
- **Cost Integration**: Test priority routing with cost optimization
- **Performance Integration**: Test priority routing with performance optimization
- **Fallback Chain**: Test complete fallback chain with priority

### 3. Performance Tests

#### 3.1 Priority Lookup Performance
- **Rule Retrieval**: Test performance of retrieving enabled rules
- **Priority Sorting**: Test performance of sorting rules by priority
- **Pattern Matching**: Test performance of priority-aware pattern matching
- **Large Rule Sets**: Test performance with 1000+ routing rules

#### 3.2 Memory Usage Tests
- **Rule Storage**: Test memory usage of storing priority rules
- **Priority Mapping**: Test memory usage of concept to priority mapping
- **Pattern Cache**: Test memory usage of pattern matching cache

### 4. End-to-End Tests

#### 4.1 Request Routing Tests
- **High Priority Request**: Test urgent request routing to high-priority rules
- **Medium Priority Request**: Test standard request routing to medium-priority rules
- **Low Priority Request**: Test background request routing to low-priority rules
- **No Match Request**: Test fallback routing when no patterns match

#### 4.2 Real-World Scenarios
- **Urgent Code Generation**: Test urgent coding tasks using high-priority routing
- **Background Processing**: Test background indexing using low-priority routing
- **Interactive Development**: Test interactive coding using medium-priority routing
- **Mixed Workload**: Test system handling mixed priority requests

### 5. Edge Cases and Error Handling

#### 5.1 Invalid Input Tests
- **Negative Priority**: Test handling of negative priority values
- **Overflow Priority**: Test handling of priority values > 100
- **Empty Patterns**: Test routing with empty pattern strings
- **Missing Providers**: Test routing to non-existent providers
- **Invalid Models**: Test routing to non-existent models

#### 5.2 Conflict Resolution Tests
- **Equal Priority**: Test behavior when multiple rules have same priority
- **Priority Ties**: Test tie-breaking with same priority and score
- **Conflicting Rules**: Test handling of conflicting routing rules
- **Rule Overlap**: Test behavior when patterns overlap

#### 5.3 System State Tests
- **Empty Rule Set**: Test routing with no priority rules configured
- **All Disabled**: Test routing when all rules are disabled
- **Missing Taxonomy**: Test routing when taxonomy files are missing
- **Corrupted Files**: Test routing with corrupted markdown files

## Test Data

### Sample Markdown Files

#### High Priority Rule
```markdown
# Urgent Code Generation

priority:: 95
route:: openai,gpt-4o

Handles urgent code generation requests that require fast response times.

synonyms:: urgent coding, fast code, emergency programming, critical development
```

#### Medium Priority Rule
```markdown
# Standard Development

priority:: 50
route:: anthropic,claude-3-sonnet

Standard software development tasks with balanced cost and performance.

synonyms:: development, coding, programming, software engineering
```

#### Low Priority Rule
```markdown
# Background Processing

priority:: 15
route:: ollama,qwen2.5-coder:latest

Background tasks that can use slower, cost-effective models.

synonyms:: background, batch, offline, processing
```

### Test Requests

#### Urgent Request
```json
{
  "model": "claude-3-5-sonnet",
  "messages": [
    {"role": "user", "content": "URGENT: Fix critical production bug now!"}
  ]
}
```

#### Standard Request
```json
{
  "model": "claude-3-5-sonnet",
  "messages": [
    {"role": "user", "content": "Help me implement a new feature"}
  ]
}
```

#### Background Request
```json
{
  "model": "claude-3-5-haiku",
  "messages": [
    {"role": "user", "content": "Index this codebase for search"}
  ]
}
```

## Test Automation

### Unit Test Framework
- **Rust**: Use built-in `#[test]` attribute and `cargo test`
- **Test Organization**: Group tests by module and functionality
- **Mock Data**: Use realistic test data in `tests/` directory
- **Property Testing**: Use `proptest` for property-based testing

### Integration Test Framework
- **Test Server**: Spin up actual proxy server for integration tests
- **Database Setup**: Use test databases for cost and performance data
- **File System**: Use temporary directories for taxonomy files
- **HTTP Clients**: Use `reqwest` for HTTP request testing

### Performance Test Framework
- **Criterion**: Use `criterion` for benchmarking
- **Memory Profiling**: Use `valgrind` or `heaptrack` for memory analysis
- **Load Testing**: Use `hey` or `wrk` for load testing
- **Metrics Collection**: Use built-in metrics for performance analysis

### End-to-End Test Framework
- **Docker Compose**: Use containers for complete system testing
- **Test Scenarios**: Define realistic user scenarios
- **Assertion Helpers**: Create custom assertion helpers
- **Test Data Management**: Manage test data lifecycle

## Success Criteria

### Functional Requirements
- [ ] All priority values (0-100) are handled correctly
- [ ] High priority rules override lower priority rules
- [ ] Weighted scoring is calculated correctly
- [ ] Fallback behavior works as expected
- [ ] Markdown parsing is robust and error-tolerant

### Performance Requirements
- [ ] Priority lookup completes within 1ms for 1000 rules
- [ ] Memory usage scales linearly with rule count
- [ ] Pattern matching maintains sub-millisecond performance
- [ ] System handles 1000+ concurrent requests with priority routing

### Reliability Requirements
- [ ] System gracefully handles invalid priority values
- [ ] No memory leaks in priority rule management
- [ ] System recovers from corrupted taxonomy files
- [ ] All error conditions are properly logged

### Maintainability Requirements
- [ ] Test coverage exceeds 90% for priority routing code
- [ ] All tests are documented and self-explanatory
- [ ] Test data is versioned and maintained
- [ ] Performance benchmarks are tracked over time

## Test Execution

### Running Tests
```bash
# Unit tests
cargo test -p terraphim_types
cargo test -p terraphim-llm-proxy

# Integration tests
cargo test --test integration_tests

# Performance benchmarks
cargo bench

# End-to-end tests
docker-compose -f test-compose.yml up --abort-on-container-exit
```

### Test Reports
- **Unit Test Coverage**: Use `cargo tarpaulin` for coverage reports
- **Performance Reports**: Use `criterion` HTML reports
- **Integration Results**: Use JUnit XML format for CI integration
- **E2E Summaries**: Generate HTML reports with screenshots

## Continuous Integration

### CI Pipeline
1. **Unit Tests**: Run on every commit
2. **Integration Tests**: Run on every pull request
3. **Performance Tests**: Run nightly
4. **E2E Tests**: Run before releases

### Quality Gates
- All tests must pass
- Coverage must exceed 90%
- Performance must not regress > 5%
- No new security vulnerabilities

### Monitoring
- Test execution time trends
- Coverage percentage changes
- Performance benchmark trends
- Flaky test detection

## Documentation

### Test Documentation
- Each test file has header documentation
- Complex test logic has inline comments
- Test data has explanatory documentation
- Known limitations are documented

### API Documentation
- Priority routing API is documented
- Configuration options are explained
- Error conditions are described
- Usage examples are provided

## Future Enhancements

### Test Enhancements
- **Property Testing**: Expand property-based test coverage
- **Fuzz Testing**: Add fuzz testing for markdown parsing
- **Chaos Testing**: Test system behavior under failure conditions
- **Contract Testing**: Add API contract tests

### Tooling Improvements
- **Test Data Generation**: Automated test data generation
- **Performance Visualization**: Better performance visualization
- **Test Parallelization**: Improve test execution parallelization
- **Debug Tools**: Better debugging tools for test failures