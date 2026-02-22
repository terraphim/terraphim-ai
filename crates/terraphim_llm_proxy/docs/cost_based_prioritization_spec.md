# Cost-Based Prioritization Feature Specification

## Overview

Add cost-aware routing capabilities to terraphim-llm-proxy with OpenRouter pricing data integration and intelligent model prioritization based on cost-effectiveness.

## Goals

1. **Pricing Data Integration**: Fetch and maintain OpenRouter pricing data for all models
2. **Cost-Optimized Routing**: Prioritize models based on cost-performance ratio
3. **Budget Management**: Implement cost controls and spending limits
4. **Transparent Cost Tracking**: Provide detailed cost analytics and reporting

## Feature Requirements

### 1. OpenRouter Pricing Integration

#### 1.1 Pricing Data Fetching
- **API Integration**: Connect to OpenRouter pricing API
- **Model Coverage**: Support all OpenRouter models
- **Regular Updates**: Automatic pricing data refresh
- **Fallback Caching**: Local cache for pricing data

#### 1.2 Pricing Data Structure
```rust
pub struct ModelPricing {
    pub model_id: String,
    pub model_name: String,
    pub provider: String,
    pub input_cost_per_1k_tokens: f64,
    pub output_cost_per_1k_tokens: f64,
    pub context_window: usize,
    pub supports_images: bool,
    pub supports_tools: bool,
    pub last_updated: SystemTime,
}

pub struct PricingDatabase {
    models: Arc<RwLock<HashMap<String, ModelPricing>>>,
    last_updated: Arc<RwLock<SystemTime>>,
}
```

#### 1.3 OpenRouter API Integration
```rust
pub struct OpenRouterPricingClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterPricingClient {
    pub async fn fetch_pricing(&self) -> Result<Vec<ModelPricing>>;
    pub async fn refresh_pricing(&self) -> Result<()>;
    pub fn get_model_pricing(&self, model_id: &str) -> Option<ModelPricing>;
}
```

### 2. Cost Calculation Engine

#### 2.1 Request Cost Estimation
```rust
pub struct CostEstimate {
    pub model_id: String,
    pub input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
}

pub struct CostCalculator {
    pricing_db: Arc<PricingDatabase>,
    config: CostConfig,
}

impl CostCalculator {
    pub fn estimate_request_cost(&self, request: &ChatRequest, model: &str) -> Result<CostEstimate>;
    pub fn calculate_actual_cost(&self, usage: &TokenUsage, model: &str) -> Result<f64>;
    pub fn get_cheapest_model(&self, constraints: &ModelConstraints) -> Option<String>;
}
```

#### 2.2 Token Usage Estimation
- **Input Analysis**: Count input tokens from request
- **Output Prediction**: Estimate output tokens based on request type
- **Context Window**: Validate against model context limits
- **Historical Data**: Use historical token usage for better estimates

### 3. Cost-Optimized Routing

#### 3.1 Routing Decision Enhancement
```rust
pub struct CostAwareRoutingDecision {
    pub provider: Provider,
    pub model: String,
    pub scenario: RoutingScenario,
    pub cost_estimate: CostEstimate,
    pub cost_effectiveness_score: f64,
    pub budget_impact: BudgetImpact,
}

pub struct BudgetImpact {
    pub request_cost: f64,
    pub daily_usage: f64,
    pub monthly_usage: f64,
    pub budget_remaining: f64,
    pub within_budget: bool,
}
```

#### 3.2 Cost-Effectiveness Scoring
```rust
pub fn calculate_cost_effectiveness_score(
    performance_score: f64,
    cost_per_1k_tokens: f64,
    quality_weight: f64,
    cost_weight: f64,
) -> f64 {
    // Balance quality vs cost for optimal selection
}
```

#### 3.3 Budget Management
```rust
pub struct BudgetManager {
    daily_limit: f64,
    monthly_limit: f64,
    current_usage: Arc<RwLock<BudgetUsage>>,
}

pub struct BudgetUsage {
    pub daily_spend: f64,
    pub monthly_spend: f64,
    pub request_count: u64,
    pub last_reset: SystemTime,
}
```

### 4. Configuration

#### 4.1 Cost Settings
```toml
[cost]
enabled = true
pricing_update_interval_hours = 24
budget_daily_limit = 10.0
budget_monthly_limit = 300.0
cost_tracking_enabled = true

[cost.weights]
quality = 0.6
cost = 0.4

[cost.thresholds]
max_cost_per_request = 1.0
max_cost_per_1k_tokens = 0.10
min_cost_effectiveness_score = 0.5
```

#### 4.2 Provider-Specific Settings
```toml
[cost.providers.openrouter]
enabled = true
pricing_api_url = "https://openrouter.ai/api/v1/pricing"
update_interval_hours = 12

[cost.providers.deepseek]
enabled = true
input_cost_per_1k_tokens = 0.14
output_cost_per_1k_tokens = 0.28
context_window = 64000
```

### 5. API Endpoints

#### 5.1 Pricing Endpoints
- `GET /pricing/models` - Get all model pricing
- `GET /pricing/models/{model_id}` - Get specific model pricing
- `POST /pricing/refresh` - Force pricing data refresh
- `GET /pricing/last-updated` - Get last pricing update time

#### 5.2 Cost Tracking Endpoints
- `GET /cost/usage` - Get current cost usage
- `GET /cost/estimates` - Get cost estimates for models
- `GET /cost/budget` - Get budget status
- `GET /cost/history` - Get cost history

#### 5.3 Analytics Endpoints
- `GET /analytics/cost-by-model` - Cost breakdown by model
- `GET /analytics/cost-by-scenario` - Cost by routing scenario
- `GET /analytics/cost-trends` - Cost trends over time
- `GET /analytics/savings` - Cost savings from optimization

### 6. Integration with Existing Routing

#### 6.1 RouterAgent Enhancement
```rust
impl RouterAgent {
    // New method for cost-aware routing
    pub async fn route_with_cost_optimization(
        &self,
        request: &ChatRequest,
        hints: &RoutingHints,
    ) -> Result<CostAwareRoutingDecision> {
        // Existing routing + cost optimization
    }
    
    // Get cheapest viable model for scenario
    fn get_cheapest_viable_model(&self, scenario: &RoutingScenario) -> Option<(String, String)> {
        // Find cheapest model that meets requirements
    }
    
    // Check if request is within budget
    fn is_within_budget(&self, cost_estimate: &CostEstimate) -> bool {
        // Budget validation logic
    }
}
```

#### 6.2 Multi-Objective Optimization
```rust
pub struct RoutingOptimizer {
    performance_weight: f64,
    cost_weight: f64,
    reliability_weight: f64,
}

impl RoutingOptimizer {
    pub fn optimize_routing(
        &self,
        candidates: Vec<RoutingCandidate>,
        request: &ChatRequest,
    ) -> Result<CostAwareRoutingDecision> {
        // Multi-objective optimization for best overall choice
    }
}
```

### 7. Cost Tracking and Analytics

#### 7.1 Usage Tracking
```rust
pub struct CostTracker {
    usage_db: Arc<RwLock<CostUsageDatabase>>,
    real_time_tracker: Arc<Mutex<RealTimeCostTracker>>,
}

pub struct CostUsageRecord {
    pub timestamp: SystemTime,
    pub model_id: String,
    pub scenario: RoutingScenario,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
    pub session_id: Option<String>,
}
```

#### 7.2 Analytics Engine
```rust
pub struct CostAnalytics {
    cost_tracker: Arc<CostTracker>,
    pricing_db: Arc<PricingDatabase>,
}

impl CostAnalytics {
    pub fn get_cost_by_model(&self, period: TimePeriod) -> Vec<ModelCostBreakdown>;
    pub fn get_cost_trends(&self, period: TimePeriod) -> Vec<CostTrend>;
    pub fn calculate_savings(&self, period: TimePeriod) -> CostSavingsReport;
    pub fn get_budget_forecast(&self) -> BudgetForecast;
}
```

## Implementation Plan

### Phase 1: Pricing Data Integration
1. Implement OpenRouter pricing API client
2. Create pricing database with caching
3. Add pricing data structures and validation
4. Basic configuration support

### Phase 2: Cost Calculation
1. Implement cost estimation engine
2. Add token counting and usage prediction
3. Create cost-effectiveness scoring
4. Budget management system

### Phase 3: Routing Integration
1. Enhance RouterAgent with cost-aware routing
2. Implement multi-objective optimization
3. Add budget constraints to routing decisions
4. Cost-based fallback logic

### Phase 4: Analytics and Monitoring
1. Cost tracking and usage analytics
2. Real-time cost monitoring
3. Budget alerts and notifications
4. Cost optimization recommendations

## Testing Strategy

### Unit Tests
- Pricing data parsing and validation
- Cost calculation accuracy
- Budget management logic
- Scoring algorithm correctness

### Integration Tests
- OpenRouter API integration
- End-to-end cost-aware routing
- Budget constraint enforcement
- Analytics data accuracy

### Performance Tests
- Cost calculation performance under load
- Pricing database query performance
- Memory usage with large pricing datasets
- Routing decision latency impact

## Error Handling

### API Failures
- OpenRouter API timeout handling
- Pricing data fallback mechanisms
- Retry logic with exponential backoff
- Graceful degradation with stale pricing data

### Data Issues
- Invalid pricing data handling
- Missing model pricing fallbacks
- Currency conversion error handling
- Data validation and sanitization

### Budget Issues
- Budget exceeded handling
- Cost estimation errors
- Currency fluctuation impacts
- Emergency budget overrides

## Security Considerations

### API Security
- OpenRouter API key protection
- Rate limiting on pricing API calls
- Input validation for cost parameters
- Audit logging for cost-related operations

### Data Protection
- Sensitive cost data encryption
- Access control for budget information
- Cost data retention policies
- Compliance with financial regulations

## Monitoring & Observability

### Metrics
- Cost estimation accuracy
- Budget utilization rates
- Pricing data freshness
- Cost optimization effectiveness

### Logging
- Cost calculation logs
- Budget enforcement actions
- Pricing update events
- Cost optimization decisions

### Alerts
- Budget threshold warnings
- Unusual cost spikes
- Pricing data staleness
- Cost calculation errors

## Future Enhancements

### Advanced Optimization
- Machine learning for cost prediction
- Dynamic pricing adaptation
- Market-based cost optimization
- Multi-provider cost arbitrage

### Extended Features
- Cost allocation by user/team
- Project-based budgeting
- Cost forecasting and planning
- Automated cost optimization

### Integration
- Accounting system integration
- Billing and invoicing
- Cost reporting dashboards
- Financial analytics integration