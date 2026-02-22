# Latency and Throughput Testing Feature Specification

## Overview

Add automated latency and throughput testing capabilities to terraphim-llm-proxy with dynamic model prioritization based on performance metrics.

## Goals

1. **Performance Testing**: Automatically test selected models for latency and throughput
2. **Dynamic Prioritization**: Prioritize high-throughput models for routing decisions
3. **Continuous Monitoring**: Ongoing performance tracking with adaptive routing
4. **Integration**: Seamlessly integrate with existing 3-phase routing system

## Feature Requirements

### 1. Performance Testing Engine

#### 1.1 Test Types
- **Latency Testing**: Measure response time for standardized prompts
- **Throughput Testing**: Measure tokens/second for sustained requests
- **Concurrent Testing**: Measure performance under load
- **Health Testing**: Basic availability and response validation

#### 1.2 Test Methodology
- **Standardized Prompts**: Use consistent test prompts across models
- **Multiple Runs**: Statistical significance with multiple test iterations
- **Configurable Intervals**: Regular testing schedule (hourly/daily/weekly)
- **Test Scenarios**: Different prompt complexities and token lengths

#### 1.3 Metrics Collection
```rust
pub struct PerformanceMetrics {
    pub model_name: String,
    pub provider_name: String,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_tokens_per_sec: f64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub last_tested: SystemTime,
    pub test_count: u32,
}
```

### 2. Performance Database

#### 2.1 Storage
- **In-Memory Cache**: Fast access for routing decisions
- **Persistent Storage**: Historical performance data
- **TTL Management**: Automatic cleanup of old metrics
- **Backup/Restore**: Performance data persistence

#### 2.2 Data Structure
```rust
pub struct PerformanceDatabase {
    metrics: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
    config: PerformanceConfig,
}
```

### 3. Dynamic Prioritization

#### 3.1 Routing Integration
- **Performance Score**: Composite score combining latency and throughput
- **Weight Selection**: Configurable weights for different metrics
- **Threshold Filtering**: Minimum performance requirements
- **Fallback Logic**: Graceful degradation when performance data unavailable

#### 3.2 Score Calculation
```rust
pub fn calculate_performance_score(
    latency_weight: f64,
    throughput_weight: f64,
    reliability_weight: f64,
) -> f64 {
    // Normalize metrics and calculate weighted score
}
```

### 4. Configuration

#### 4.1 Performance Settings
```toml
[performance]
enabled = true
test_interval_minutes = 60
test_timeout_seconds = 30
max_concurrent_tests = 3
min_test_count = 5

[performance.thresholds]
max_latency_ms = 5000
min_throughput_tokens_per_sec = 10
min_success_rate = 0.95

[performance.weights]
latency = 0.4
throughput = 0.4
reliability = 0.2
```

#### 4.2 Model Selection
```toml
[performance.test_models]
providers = ["openrouter", "deepseek"]
models = [
    "openrouter:anthropic/claude-3.5-sonnet",
    "openrouter:openai/gpt-4",
    "deepseek:deepseek-chat"
]
```

### 5. API Endpoints

#### 5.1 Management Endpoints
- `GET /performance/metrics` - Get current performance data
- `POST /performance/test` - Trigger manual performance test
- `GET /performance/history/{model}` - Get historical performance
- `DELETE /performance/metrics/{model}` - Clear performance data

#### 5.2 Monitoring Endpoints
- `GET /health/performance` - Performance testing system health
- `GET /metrics/performance` - Prometheus-compatible metrics

### 6. Integration with Existing Routing

#### 6.1 RouterAgent Enhancement
```rust
impl RouterAgent {
    // New method for performance-aware routing
    pub async fn route_with_performance(
        &self,
        request: &ChatRequest,
        hints: &RoutingHints,
    ) -> Result<RoutingDecision> {
        // Existing 3-phase routing + performance optimization
    }
    
    // Get best performing model for scenario
    fn get_best_performing_model(&self, scenario: &RoutingScenario) -> Option<(String, String)> {
        // Query performance database and select optimal model
    }
}
```

#### 6.2 Routing Decision Enhancement
```rust
pub struct RoutingDecision {
    pub provider: Provider,
    pub model: String,
    pub scenario: RoutingScenario,
    pub performance_score: Option<f64>,  // New field
    pub performance_metrics: Option<PerformanceMetrics>,  // New field
}
```

## Implementation Plan

### Phase 1: Core Testing Engine
1. Implement `PerformanceTester` with basic latency/throughput tests
2. Create `PerformanceMetrics` data structures
3. Add in-memory performance database
4. Basic configuration support

### Phase 2: Integration
1. Enhance `RouterAgent` with performance-aware routing
2. Add performance scoring algorithms
3. Implement periodic testing scheduler
4. Add management API endpoints

### Phase 3: Advanced Features
1. Persistent storage for historical data
2. Advanced test scenarios and prompts
3. Performance trend analysis
4. Alerting for performance degradation

### Phase 4: Monitoring & Observability
1. Prometheus metrics integration
2. Performance dashboards
3. Alerting and notifications
4. Performance optimization recommendations

## Testing Strategy

### Unit Tests
- Performance metrics calculation
- Score computation algorithms
- Configuration validation
- Database operations

### Integration Tests
- End-to-end performance testing
- Router integration with performance data
- API endpoint functionality
- Error handling and fallbacks

### Performance Tests
- Load testing of the testing system itself
- Concurrent test execution
- Memory usage under sustained testing
- Database performance with large datasets

## Error Handling

### Test Failures
- Timeout handling for long-running tests
- Retry logic with exponential backoff
- Graceful degradation when testing fails
- Error reporting and alerting

### Data Issues
- Handling missing or corrupted performance data
- Fallback to default routing when performance unavailable
- Data validation and sanitization
- Automatic cleanup of invalid data

## Security Considerations

### API Security
- Authentication for management endpoints
- Rate limiting on performance testing
- Input validation for test parameters
- Audit logging for performance tests

### Resource Protection
- Limits on concurrent testing
- Resource usage monitoring
- Cost controls for API usage during testing
- Isolation of testing from production traffic

## Monitoring & Observability

### Metrics
- Test execution counts and success rates
- Performance score distributions
- Testing system resource usage
- Routing decision changes due to performance

### Logging
- Structured logging for test execution
- Performance trend logging
- Error and warning logs
- Configuration change logging

### Alerts
- Performance degradation alerts
- Testing system failures
- Unusual routing pattern changes
- Resource exhaustion warnings

## Future Enhancements

### Advanced Analytics
- Machine learning for performance prediction
- Anomaly detection in performance patterns
- Automated performance optimization
- Cost-performance analysis

### Extended Testing
- Multi-region performance testing
- Different time-of-day performance patterns
- Load-dependent performance testing
- A/B testing for routing strategies