# API Reference Snippets

**Generated:** 2026-04-29T05:50:59Z
**Agent:** documentation-generator (Ferrox)

## terraphim_types

### Types
```rust
crates/terraphim_types/src/mcp_tool.rs:28:pub struct McpToolEntry {
crates/terraphim_types/src/review.rs:11:pub enum FindingSeverity {
crates/terraphim_types/src/review.rs:22:pub enum FindingCategory {
crates/terraphim_types/src/review.rs:33:pub struct ReviewFinding {
crates/terraphim_types/src/review.rs:52:pub struct ReviewAgentOutput {
crates/terraphim_types/src/capability.rs:12:pub enum Capability {
crates/terraphim_types/src/capability.rs:59:pub enum ProviderType {
crates/terraphim_types/src/capability.rs:81:pub enum CostLevel {
crates/terraphim_types/src/capability.rs:93:pub enum Latency {
crates/terraphim_types/src/capability.rs:104:pub struct Provider {
crates/terraphim_types/src/capability.rs:189:pub struct ProcessId(pub u64);
crates/terraphim_types/src/procedure.rs:32:pub struct ProcedureStep {
crates/terraphim_types/src/procedure.rs:51:pub struct ProcedureConfidence {
crates/terraphim_types/src/procedure.rs:114:pub struct CapturedProcedure {
crates/terraphim_types/src/persona.rs:46:pub struct PersonaDefinition {
crates/terraphim_types/src/persona.rs:79:pub struct CharacteristicDef {
crates/terraphim_types/src/persona.rs:91:pub struct SfiaSkillDef {
crates/terraphim_types/src/persona.rs:200:pub enum PersonaLoadError {
crates/terraphim_types/src/shared_learning.rs:23:pub enum TrustLevel {
crates/terraphim_types/src/shared_learning.rs:95:pub enum TrustLevelError {
crates/terraphim_types/src/shared_learning.rs:103:pub enum LearningCategory {
crates/terraphim_types/src/shared_learning.rs:129:pub trait LearningStore: Send + Sync {
crates/terraphim_types/src/shared_learning.rs:149:pub struct InMemoryLearningStore {
crates/terraphim_types/src/shared_learning.rs:284:pub struct QualityMetrics {
crates/terraphim_types/src/shared_learning.rs:341:pub enum SuggestionStatus {
crates/terraphim_types/src/shared_learning.rs:374:pub enum LearningSource {
crates/terraphim_types/src/shared_learning.rs:407:pub struct SharedLearning {
crates/terraphim_types/src/shared_learning.rs:664:pub enum StoreError {
crates/terraphim_types/src/lib.rs:164:pub struct RoleName {
crates/terraphim_types/src/lib.rs:255:pub struct NormalizedTermValue(String);
```

### Functions
```rust
crates/terraphim_types/src/mcp_tool.rs:63:    pub fn new(name: &str, description: &str, server_name: &str) -> Self {
crates/terraphim_types/src/mcp_tool.rs:75:    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
crates/terraphim_types/src/mcp_tool.rs:81:    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
crates/terraphim_types/src/mcp_tool.rs:87:    pub fn search_text(&self) -> String {
crates/terraphim_types/src/review.rs:62:pub fn deduplicate_findings(findings: Vec<ReviewFinding>) -> Vec<ReviewFinding> {
crates/terraphim_types/src/capability.rs:39:    pub fn all() -> Vec<Capability> {
crates/terraphim_types/src/capability.rs:125:    pub fn new(
crates/terraphim_types/src/capability.rs:143:    pub fn with_cost(mut self, cost: CostLevel) -> Self {
crates/terraphim_types/src/capability.rs:149:    pub fn with_latency(mut self, latency: Latency) -> Self {
crates/terraphim_types/src/capability.rs:155:    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
crates/terraphim_types/src/capability.rs:161:    pub fn has_capability(&self, capability: &Capability) -> bool {
crates/terraphim_types/src/capability.rs:166:    pub fn has_all_capabilities(&self, capabilities: &[Capability]) -> bool {
crates/terraphim_types/src/capability.rs:171:    pub fn matches_keywords(&self, text: &str) -> bool {
crates/terraphim_types/src/capability.rs:179:    pub fn capability_names(&self) -> Vec<String> {
crates/terraphim_types/src/capability.rs:193:    pub fn new() -> Self {
crates/terraphim_types/src/procedure.rs:62:    pub fn new() -> Self {
crates/terraphim_types/src/procedure.rs:71:    pub fn record_success(&mut self) {
crates/terraphim_types/src/procedure.rs:77:    pub fn record_failure(&mut self) {
crates/terraphim_types/src/procedure.rs:96:    pub fn total_executions(&self) -> u32 {
crates/terraphim_types/src/procedure.rs:101:    pub fn is_high_confidence(&self) -> bool {
crates/terraphim_types/src/procedure.rs:140:    pub fn new(id: String, title: String, description: String) -> Self {
crates/terraphim_types/src/procedure.rs:157:    pub fn add_step(&mut self, step: ProcedureStep) {
crates/terraphim_types/src/procedure.rs:163:    pub fn add_steps(&mut self, steps: Vec<ProcedureStep>) {
crates/terraphim_types/src/procedure.rs:169:    pub fn with_source_session(mut self, session_id: String) -> Self {
crates/terraphim_types/src/procedure.rs:175:    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
crates/terraphim_types/src/procedure.rs:181:    pub fn with_confidence(mut self, confidence: ProcedureConfidence) -> Self {
crates/terraphim_types/src/procedure.rs:192:    pub fn record_success(&mut self) {
crates/terraphim_types/src/procedure.rs:198:    pub fn record_failure(&mut self) {
crates/terraphim_types/src/procedure.rs:204:    pub fn step_count(&self) -> usize {
crates/terraphim_types/src/procedure.rs:209:    pub fn is_empty(&self) -> bool {
```

## terraphim_config

### Types
```rust
crates/terraphim_config/src/lib.rs:31:pub type Result<T> = std::result::Result<T, TerraphimConfigError>;
crates/terraphim_config/src/lib.rs:38:pub enum TerraphimConfigError {
crates/terraphim_config/src/lib.rs:201:pub struct Role {
crates/terraphim_config/src/lib.rs:290:pub enum ServiceType {
crates/terraphim_config/src/lib.rs:320:pub struct Haystack {
crates/terraphim_config/src/lib.rs:435:pub struct KnowledgeGraph {
crates/terraphim_config/src/lib.rs:454:pub struct KnowledgeGraphLocal {
crates/terraphim_config/src/lib.rs:477:pub struct ConfigBuilder {
crates/terraphim_config/src/lib.rs:826:pub enum ConfigId {
crates/terraphim_config/src/lib.rs:838:pub struct Config {
crates/terraphim_config/src/lib.rs:952:pub struct ConfigState {
crates/terraphim_config/src/llm_router.rs:10:pub struct LlmRouterConfig {
crates/terraphim_config/src/llm_router.rs:56:pub enum RouterMode {
crates/terraphim_config/src/llm_router.rs:69:pub enum RouterStrategy {
```

### Functions
```rust
crates/terraphim_config/src/lib.rs:85:pub fn expand_path(path: &str) -> PathBuf {
crates/terraphim_config/src/lib.rs:248:    pub fn new(name: impl Into<RoleName>) -> Self {
crates/terraphim_config/src/lib.rs:272:    pub fn has_llm_config(&self) -> bool {
crates/terraphim_config/src/lib.rs:277:    pub fn get_llm_model(&self) -> Option<&str> {
crates/terraphim_config/src/lib.rs:389:    pub fn new(location: String, service: ServiceType, read_only: bool) -> Self {
crates/terraphim_config/src/lib.rs:401:    pub fn with_atomic_secret(mut self, secret: Option<String>) -> Self {
crates/terraphim_config/src/lib.rs:410:    pub fn with_extra_parameters(
crates/terraphim_config/src/lib.rs:419:    pub fn with_extra_parameter(mut self, key: String, value: String) -> Self {
crates/terraphim_config/src/lib.rs:425:    pub fn get_extra_parameters(&self) -> &std::collections::HashMap<String, String> {
crates/terraphim_config/src/lib.rs:446:    pub fn is_set(&self) -> bool {
crates/terraphim_config/src/lib.rs:486:    pub fn new() -> Self {
crates/terraphim_config/src/lib.rs:493:    pub fn new_with_id(id: ConfigId) -> Self {
crates/terraphim_config/src/lib.rs:508:    pub fn build_default_embedded(mut self) -> Self {
crates/terraphim_config/src/lib.rs:597:    pub fn get_default_data_path(&self) -> PathBuf {
crates/terraphim_config/src/lib.rs:600:    pub fn build_default_server(mut self) -> Self {
crates/terraphim_config/src/lib.rs:694:    pub fn build_default_desktop(mut self) -> Self {
crates/terraphim_config/src/lib.rs:761:    pub fn from_config(
crates/terraphim_config/src/lib.rs:774:    pub fn global_shortcut(mut self, global_shortcut: &str) -> Self {
crates/terraphim_config/src/lib.rs:780:    pub fn add_role(mut self, role_name: &str, role: Role) -> Self {
crates/terraphim_config/src/lib.rs:792:    pub fn default_role(mut self, default_role: &str) -> Result<Self> {
crates/terraphim_config/src/lib.rs:807:    pub fn build(self) -> Result<Config> {
crates/terraphim_config/src/lib.rs:866:    pub fn load_from_json_file(path: &str) -> Result<Self> {
crates/terraphim_config/src/lib.rs:964:    pub async fn new(config: &mut Config) -> Result<Self> {
crates/terraphim_config/src/lib.rs:1072:    pub async fn get_default_role(&self) -> RoleName {
crates/terraphim_config/src/lib.rs:1077:    pub async fn get_selected_role(&self) -> RoleName {
crates/terraphim_config/src/lib.rs:1083:    pub async fn get_role(&self, role: &RoleName) -> Option<Role> {
crates/terraphim_config/src/lib.rs:1089:    pub async fn add_to_roles(&mut self, document: &Document) -> OpendalResult<()> {
crates/terraphim_config/src/lib.rs:1101:    pub async fn search_indexed_documents(
```

## terraphim_service

### Types
```rust
crates/terraphim_service/src/queue_based_rate_limiter.rs:20:pub struct QueueBasedTokenBucketLimiter {
crates/terraphim_service/src/queue_based_rate_limiter.rs:230:pub struct QueueBasedRateLimiterManager {
crates/terraphim_service/src/openrouter.rs:11:pub enum OpenRouterError {
crates/terraphim_service/src/openrouter.rs:36:pub type Result<T> = std::result::Result<T, OpenRouterError>;
crates/terraphim_service/src/openrouter.rs:44:pub struct OpenRouterService {
crates/terraphim_service/src/openrouter.rs:395:pub struct OpenRouterService;
crates/terraphim_service/src/conversation_service.rs:14:pub struct ConversationFilter {
crates/terraphim_service/src/conversation_service.rs:31:pub struct ConversationStatistics {
crates/terraphim_service/src/conversation_service.rs:40:pub struct ConversationService {
crates/terraphim_service/src/error.rs:11:pub trait TerraphimError: std::error::Error + Send + Sync + 'static {
crates/terraphim_service/src/error.rs:28:pub enum ErrorCategory {
crates/terraphim_service/src/error.rs:47:pub enum CommonError {
crates/terraphim_service/src/error.rs:261:pub type TerraphimResult<T> = Result<T, CommonError>;
crates/terraphim_service/src/summarization_queue.rs:15:pub struct TaskId(pub Uuid);
crates/terraphim_service/src/summarization_queue.rs:37:pub enum Priority {
crates/terraphim_service/src/summarization_queue.rs:51:pub enum TaskStatus {
crates/terraphim_service/src/summarization_queue.rs:101:pub struct SummarizationTask {
crates/terraphim_service/src/summarization_queue.rs:188:pub struct QueueConfig {
crates/terraphim_service/src/summarization_queue.rs:239:pub struct RateLimitConfig {
crates/terraphim_service/src/summarization_queue.rs:250:pub enum QueueCommand {
crates/terraphim_service/src/summarization_queue.rs:267:pub struct QueueStats {
crates/terraphim_service/src/summarization_queue.rs:292:pub struct RateLimiterStatus {
crates/terraphim_service/src/summarization_queue.rs:305:pub enum SubmitResult {
crates/terraphim_service/src/summarization_queue.rs:321:pub struct SummarizationQueue {
crates/terraphim_service/src/rate_limiter.rs:12:pub struct TokenBucketLimiter {
crates/terraphim_service/src/rate_limiter.rs:155:pub struct RateLimiterManager {
crates/terraphim_service/src/llm_proxy.rs:14:pub enum LlmProxyError {
crates/terraphim_service/src/llm_proxy.rs:31:pub type Result<T> = std::result::Result<T, LlmProxyError>;
crates/terraphim_service/src/llm_proxy.rs:35:pub struct ProxyConfig {
crates/terraphim_service/src/llm_proxy.rs:86:pub struct LlmProxyClient {
```

### Functions
```rust
crates/terraphim_service/src/queue_based_rate_limiter.rs:29:    pub fn new(config: &RateLimitConfig) -> Self {
crates/terraphim_service/src/queue_based_rate_limiter.rs:42:    pub async fn try_acquire(&self, tokens_needed: f64) -> bool {
crates/terraphim_service/src/queue_based_rate_limiter.rs:62:    pub async fn acquire(&self, tokens_needed: f64) -> Result<(), crate::ServiceError> {
crates/terraphim_service/src/queue_based_rate_limiter.rs:90:    pub async fn get_status(&self) -> RateLimiterStatus {
crates/terraphim_service/src/queue_based_rate_limiter.rs:236:    pub fn new(configs: HashMap<String, RateLimitConfig>) -> Self {
crates/terraphim_service/src/queue_based_rate_limiter.rs:249:    pub async fn try_acquire(&self, provider: &str, tokens_needed: f64) -> bool {
crates/terraphim_service/src/queue_based_rate_limiter.rs:260:    pub async fn acquire(
crates/terraphim_service/src/queue_based_rate_limiter.rs:275:    pub async fn get_all_status(&self) -> HashMap<String, RateLimiterStatus> {
crates/terraphim_service/src/queue_based_rate_limiter.rs:287:    pub async fn add_limiter(&self, provider: String, config: RateLimitConfig) {
crates/terraphim_service/src/queue_based_rate_limiter.rs:293:    pub async fn remove_limiter(&self, provider: &str) {
crates/terraphim_service/src/conversation_service.rs:46:    pub fn new() -> Self {
crates/terraphim_service/src/conversation_service.rs:54:    pub fn with_persistence(persistence: Arc<Mutex<dyn ConversationPersistence>>) -> Self {
crates/terraphim_service/src/conversation_service.rs:59:    pub async fn create_conversation(&self, title: String, role: RoleName) -> Result<Conversation> {
crates/terraphim_service/src/conversation_service.rs:74:    pub async fn get_conversation(&self, id: &ConversationId) -> Result<Conversation> {
crates/terraphim_service/src/conversation_service.rs:82:    pub async fn update_conversation(&self, conversation: Conversation) -> Result<Conversation> {
crates/terraphim_service/src/conversation_service.rs:95:    pub async fn delete_conversation(&self, id: &ConversationId) -> Result<()> {
crates/terraphim_service/src/conversation_service.rs:103:    pub async fn list_conversations(
crates/terraphim_service/src/conversation_service.rs:148:    pub async fn search_conversations(&self, query: &str) -> Result<Vec<ConversationSummary>> {
crates/terraphim_service/src/conversation_service.rs:163:    pub async fn export_conversation(&self, id: &ConversationId) -> Result<String> {
crates/terraphim_service/src/conversation_service.rs:172:    pub async fn import_conversation(&self, json_data: &str) -> Result<Conversation> {
crates/terraphim_service/src/conversation_service.rs:189:    pub async fn get_statistics(&self) -> Result<ConversationStatistics> {
crates/terraphim_service/src/error.rs:116:    pub fn network(message: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:123:    pub fn network_with_source(
crates/terraphim_service/src/error.rs:133:    pub fn config(message: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:140:    pub fn config_field(message: impl Into<String>, field: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:147:    pub fn validation(message: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:154:    pub fn validation_field(message: impl Into<String>, field: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:161:    pub fn auth(message: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:167:    pub fn storage(message: impl Into<String>) -> Self {
crates/terraphim_service/src/error.rs:174:    pub fn storage_with_source(
```

## terraphim_orchestrator

### Types
```rust
crates/terraphim_orchestrator/src/metrics_persistence.rs:14:pub struct PersistedAgentMetrics {
crates/terraphim_orchestrator/src/metrics_persistence.rs:39:pub struct MetricsPersistenceConfig {
crates/terraphim_orchestrator/src/metrics_persistence.rs:57:pub trait MetricsPersistence: Send + Sync {
crates/terraphim_orchestrator/src/metrics_persistence.rs:89:pub enum MetricsPersistenceError {
crates/terraphim_orchestrator/src/metrics_persistence.rs:101:pub struct InMemoryMetricsPersistence {
crates/terraphim_orchestrator/src/metrics_persistence.rs:187:pub struct FileMetricsPersistence {
crates/terraphim_orchestrator/src/kg_router.rs:22:pub struct KgRouteDecision {
crates/terraphim_orchestrator/src/kg_router.rs:73:pub struct KgRouter {
crates/terraphim_orchestrator/src/kg_router.rs:306:pub enum KgRouterError {
crates/terraphim_orchestrator/src/nightwatch.rs:12:pub struct Claim {
crates/terraphim_orchestrator/src/nightwatch.rs:24:pub struct ReasoningCertificate {
crates/terraphim_orchestrator/src/nightwatch.rs:53:pub struct DualPanelResult {
crates/terraphim_orchestrator/src/nightwatch.rs:205:pub struct DriftMetrics {
crates/terraphim_orchestrator/src/nightwatch.rs:222:pub struct DriftScore {
crates/terraphim_orchestrator/src/nightwatch.rs:231:pub enum CorrectionLevel {
crates/terraphim_orchestrator/src/nightwatch.rs:246:pub struct DriftAlert {
crates/terraphim_orchestrator/src/nightwatch.rs:254:pub enum CorrectionAction {
crates/terraphim_orchestrator/src/nightwatch.rs:348:pub struct NightwatchMonitor {
crates/terraphim_orchestrator/src/nightwatch.rs:575:pub struct RateLimitTracker {
crates/terraphim_orchestrator/src/nightwatch.rs:582:pub struct RateLimitWindow {
crates/terraphim_orchestrator/src/dispatcher.rs:14:pub enum DispatchTask {
crates/terraphim_orchestrator/src/dispatcher.rs:138:pub struct Dispatcher {
crates/terraphim_orchestrator/src/dispatcher.rs:155:pub struct DispatcherStats {
crates/terraphim_orchestrator/src/concurrency.rs:12:pub struct ConcurrencyController {
crates/terraphim_orchestrator/src/concurrency.rs:28:pub struct ModeQuotas {
crates/terraphim_orchestrator/src/concurrency.rs:37:pub struct ProjectCaps {
crates/terraphim_orchestrator/src/concurrency.rs:62:pub enum FairnessPolicy {
crates/terraphim_orchestrator/src/concurrency.rs:85:pub struct AgentPermit {
crates/terraphim_orchestrator/src/persona.rs:13:pub struct PersonaRegistry {
crates/terraphim_orchestrator/src/persona.rs:112:pub enum MetapromptRenderError {
```

### Functions
```rust
crates/terraphim_orchestrator/src/metrics_persistence.rs:27:    pub fn new(agents: HashMap<String, AgentMetrics>, fleet: AgentMetrics) -> Self {
crates/terraphim_orchestrator/src/metrics_persistence.rs:108:    pub fn new() -> Self {
crates/terraphim_orchestrator/src/metrics_persistence.rs:193:    pub fn new(config: MetricsPersistenceConfig) -> Self {
crates/terraphim_orchestrator/src/kg_router.rs:41:    pub fn render_action(&self, prompt: &str) -> Option<String> {
crates/terraphim_orchestrator/src/kg_router.rs:52:    pub fn first_healthy_route(&self, unhealthy_providers: &[String]) -> Option<&RouteDirective> {
crates/terraphim_orchestrator/src/kg_router.rs:100:    pub fn load(taxonomy_path: impl Into<PathBuf>) -> Result<Self, KgRouterError> {
crates/terraphim_orchestrator/src/kg_router.rs:175:    pub fn route_agent(&self, task_description: &str) -> Option<KgRouteDecision> {
crates/terraphim_orchestrator/src/kg_router.rs:241:    pub fn reload(&mut self) -> Result<(), KgRouterError> {
crates/terraphim_orchestrator/src/kg_router.rs:254:    pub fn reload_if_changed(&mut self) -> bool {
crates/terraphim_orchestrator/src/kg_router.rs:287:    pub fn taxonomy_path(&self) -> &Path {
crates/terraphim_orchestrator/src/kg_router.rs:292:    pub fn rule_count(&self) -> usize {
crates/terraphim_orchestrator/src/kg_router.rs:297:    pub fn all_routes(&self) -> Vec<&RouteDirective> {
crates/terraphim_orchestrator/src/nightwatch.rs:44:pub fn validate_certificate(cert: &ReasoningCertificate) -> bool {
crates/terraphim_orchestrator/src/nightwatch.rs:81:pub fn dual_panel_evaluate(
crates/terraphim_orchestrator/src/nightwatch.rs:357:    pub fn new(config: NightwatchConfig) -> Self {
crates/terraphim_orchestrator/src/nightwatch.rs:368:    pub fn observe(&mut self, agent_name: &str, event: &OutputEvent) {
crates/terraphim_orchestrator/src/nightwatch.rs:399:    pub fn observe_health(&mut self, agent_name: &str, status: HealthStatus) {
crates/terraphim_orchestrator/src/nightwatch.rs:412:    pub fn observe_cost(
crates/terraphim_orchestrator/src/nightwatch.rs:433:    pub async fn next_alert(&mut self) -> DriftAlert {
crates/terraphim_orchestrator/src/nightwatch.rs:441:    pub fn evaluate(&mut self) {
crates/terraphim_orchestrator/src/nightwatch.rs:467:    pub fn drift_score(&self, agent_name: &str) -> Option<DriftScore> {
crates/terraphim_orchestrator/src/nightwatch.rs:482:    pub fn all_drift_scores(&self) -> Vec<DriftScore> {
crates/terraphim_orchestrator/src/nightwatch.rs:500:    pub fn reset(&mut self, agent_name: &str) {
crates/terraphim_orchestrator/src/nightwatch.rs:593:    pub fn record_call(&mut self, agent_name: &str, provider_id: &str) {
crates/terraphim_orchestrator/src/nightwatch.rs:612:    pub fn can_call(&self, agent_name: &str, provider_id: &str) -> bool {
crates/terraphim_orchestrator/src/nightwatch.rs:624:    pub fn update_limit(&mut self, agent_name: &str, provider_id: &str, limit: u32) {
crates/terraphim_orchestrator/src/nightwatch.rs:632:    pub fn remaining(&self, agent_name: &str, provider_id: &str) -> Option<u32> {
crates/terraphim_orchestrator/src/flow/state.rs:32:    pub fn new(flow_name: &str) -> Self {
crates/terraphim_orchestrator/src/flow/state.rs:45:    pub fn failed(flow_name: &str, reason: &str) -> Self {
crates/terraphim_orchestrator/src/flow/state.rs:53:    pub fn step_output(&self, step_name: &str) -> Option<&StepEnvelope> {
```

## terraphim_agent

### Types
```rust
crates/terraphim_agent/src/kg_validation.rs:22:pub struct ValidationFinding {
crates/terraphim_agent/src/kg_validation.rs:34:pub struct KgValidationResult {
crates/terraphim_agent/src/service.rs:13:pub struct TuiService {
crates/terraphim_agent/src/service.rs:838:pub struct ConnectivityResult {
crates/terraphim_agent/src/service.rs:846:pub struct FuzzySuggestion {
crates/terraphim_agent/src/service.rs:853:pub struct ChecklistResult {
crates/terraphim_agent/src/learnings/suggest.rs:10:pub struct SuggestionMetricsEntry {
crates/terraphim_agent/src/learnings/suggest.rs:19:pub struct SuggestionMetricsSummary {
crates/terraphim_agent/src/learnings/suggest.rs:27:pub struct SuggestionMetrics {
crates/terraphim_agent/src/learnings/install.rs:21:pub enum AgentType {
crates/terraphim_agent/src/learnings/install.rs:130:pub enum InstallError {
crates/terraphim_agent/src/learnings/procedure.rs:50:pub enum HealthStatus {
crates/terraphim_agent/src/learnings/procedure.rs:74:pub struct ProcedureHealthReport {
crates/terraphim_agent/src/learnings/procedure.rs:88:pub struct ProcedureStore {
crates/terraphim_agent/src/learnings/hook.rs:33:pub enum LearnHookType {
crates/terraphim_agent/src/learnings/hook.rs:45:pub enum AgentFormat {
crates/terraphim_agent/src/learnings/hook.rs:264:pub enum HookError {
crates/terraphim_agent/src/learnings/hook.rs:283:pub struct HookInput {
crates/terraphim_agent/src/learnings/hook.rs:298:pub struct ToolInput {
crates/terraphim_agent/src/learnings/hook.rs:311:pub struct ToolResult {
crates/terraphim_agent/src/learnings/export_kg.rs:14:pub enum CorrectionTypeFilter {
crates/terraphim_agent/src/learnings/replay.rs:15:pub enum StepOutcome {
crates/terraphim_agent/src/learnings/replay.rs:26:pub struct ReplayResult {
crates/terraphim_agent/src/learnings/capture.rs:22:pub enum LearningError {
crates/terraphim_agent/src/learnings/capture.rs:35:pub enum LearningSource {
crates/terraphim_agent/src/learnings/capture.rs:44:pub enum CorrectionType {
crates/terraphim_agent/src/learnings/capture.rs:102:pub struct ImportanceScore {
crates/terraphim_agent/src/learnings/capture.rs:169:pub struct LearningContext {
crates/terraphim_agent/src/learnings/capture.rs:195:pub struct CapturedLearning {
crates/terraphim_agent/src/learnings/capture.rs:502:pub struct CorrectionEvent {
```

### Functions
```rust
crates/terraphim_agent/src/tui_backend.rs:40:    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>> {
crates/terraphim_agent/src/tui_backend.rs:55:    pub async fn get_config(&self) -> Result<Config> {
crates/terraphim_agent/src/tui_backend.rs:70:    pub async fn get_rolegraph_terms(&self, role: &str) -> Result<Vec<String>> {
crates/terraphim_agent/src/tui_backend.rs:88:    pub async fn autocomplete(&self, role: &str, query: &str) -> Result<Vec<String>> {
crates/terraphim_agent/src/tui_backend.rs:111:    pub async fn summarize(
crates/terraphim_agent/src/tui_backend.rs:133:    pub async fn switch_role(&self, role: &str) -> Result<Config> {
crates/terraphim_agent/src/kg_validation.rs:43:    pub fn empty() -> Self {
crates/terraphim_agent/src/kg_validation.rs:59:pub fn validate_command_against_kg(command: &str) -> KgValidationResult {
crates/terraphim_agent/src/kg_validation.rs:77:pub fn validate_command_with_thesaurus(command: &str, thesaurus: Thesaurus) -> KgValidationResult {
crates/terraphim_agent/src/client.rs:13:    pub fn new(base_url: impl Into<String>) -> Self {
crates/terraphim_agent/src/client.rs:32:    pub async fn health(&self) -> Result<()> {
crates/terraphim_agent/src/client.rs:42:    pub async fn get_config(&self) -> Result<ConfigResponse> {
crates/terraphim_agent/src/client.rs:51:    pub async fn resolve_role(&self, role: &str) -> Result<terraphim_types::RoleName> {
crates/terraphim_agent/src/client.rs:72:    pub async fn update_selected_role(&self, role: &str) -> Result<ConfigResponse> {
crates/terraphim_agent/src/client.rs:90:    pub async fn post_config(&self, cfg: &terraphim_config::Config) -> Result<ConfigResponse> {
crates/terraphim_agent/src/client.rs:98:    pub async fn get_rolegraph_edges(&self, role: Option<&str>) -> Result<RoleGraphResponseDto> {
crates/terraphim_agent/src/client.rs:102:    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResponse> {
crates/terraphim_agent/src/client.rs:109:    pub async fn rolegraph(&self, role: Option<&str>) -> Result<RoleGraphResponseDto> {
crates/terraphim_agent/src/client.rs:394:    pub async fn chat(
crates/terraphim_agent/src/client.rs:414:    pub async fn summarize_document(
crates/terraphim_agent/src/client.rs:429:    pub async fn get_thesaurus(&self, role_name: &str) -> Result<ThesaurusResponse> {
crates/terraphim_agent/src/client.rs:436:    pub async fn get_autocomplete(
crates/terraphim_agent/src/client.rs:456:    pub async fn async_summarize_document(
crates/terraphim_agent/src/client.rs:475:    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskStatusResponse> {
crates/terraphim_agent/src/client.rs:487:    pub async fn cancel_task(&self, task_id: &str) -> Result<TaskStatusResponse> {
crates/terraphim_agent/src/client.rs:499:    pub async fn get_queue_stats(&self) -> Result<QueueStatsResponse> {
crates/terraphim_agent/src/client.rs:507:    pub async fn batch_summarize_documents(
crates/terraphim_agent/src/client.rs:528:    pub async fn list_vms(&self) -> Result<VmPoolListResponse> {
crates/terraphim_agent/src/client.rs:536:    pub async fn get_vm_pool_stats(&self) -> Result<VmPoolStatsResponse> {
crates/terraphim_agent/src/client.rs:547:    pub async fn get_vm_status(&self, vm_id: &str) -> Result<VmStatusResponse> {
```

## terraphim_server

### Types
```rust
terraphim_server/src/api_conversations.rs:17:pub struct ListPersistentConversationsQuery {
terraphim_server/src/api_conversations.rs:26:pub struct ListPersistentConversationsResponse {
terraphim_server/src/api_conversations.rs:34:pub struct GetPersistentConversationResponse {
terraphim_server/src/api_conversations.rs:41:pub struct CreatePersistentConversationRequest {
terraphim_server/src/api_conversations.rs:47:pub struct CreatePersistentConversationResponse {
terraphim_server/src/api_conversations.rs:54:pub struct UpdatePersistentConversationRequest {
terraphim_server/src/api_conversations.rs:60:pub struct UpdatePersistentConversationResponse {
terraphim_server/src/api_conversations.rs:67:pub struct DeletePersistentConversationResponse {
terraphim_server/src/api_conversations.rs:73:pub struct SearchPersistentConversationsQuery {
terraphim_server/src/api_conversations.rs:79:pub struct SearchPersistentConversationsResponse {
terraphim_server/src/api_conversations.rs:87:pub struct ExportConversationResponse {
terraphim_server/src/api_conversations.rs:94:pub struct ImportConversationRequest {
terraphim_server/src/api_conversations.rs:99:pub struct ImportConversationResponse {
terraphim_server/src/api_conversations.rs:106:pub struct ConversationStatisticsResponse {
terraphim_server/src/error.rs:10:pub enum Status {
terraphim_server/src/error.rs:20:pub struct ErrorResponse {
terraphim_server/src/error.rs:29:pub struct ApiError(pub StatusCode, pub anyhow::Error);
terraphim_server/src/error.rs:107:pub type Result<T> = std::result::Result<T, ApiError>;
terraphim_server/src/lib.rs:164:    pub struct Asset;
terraphim_server/src/lib.rs:165:    pub struct EmbeddedFile;
terraphim_server/src/lib.rs:179:pub struct AppState {
terraphim_server/src/workflows/multi_agent_handlers.rs:25:pub struct MultiAgentWorkflowExecutor {
terraphim_server/src/workflows/websocket.rs:36:pub struct WebSocketSession {
terraphim_server/src/workflows/mod.rs:26:pub struct LlmConfig {
terraphim_server/src/workflows/mod.rs:46:pub struct StepConfig {
terraphim_server/src/workflows/mod.rs:57:pub struct WorkflowRequest {
terraphim_server/src/workflows/mod.rs:67:pub struct WorkflowResponse {
terraphim_server/src/workflows/mod.rs:76:pub struct WorkflowMetadata {
terraphim_server/src/workflows/mod.rs:86:pub struct WorkflowStatus {
terraphim_server/src/workflows/mod.rs:99:pub enum ExecutionStatus {
```

### Functions
```rust
terraphim_server/src/lib.rs:168:        pub fn get(_path: &str) -> Option<EmbeddedFile> {
terraphim_server/src/lib.rs:185:pub async fn axum_server(server_hostname: SocketAddr, mut config_state: ConfigState) -> Result<()> {
terraphim_server/src/lib.rs:650:pub async fn build_router_for_tests() -> Router {
terraphim_server/src/workflows/prompt_chain.rs:21:pub async fn execute_prompt_chain(
terraphim_server/src/workflows/optimization.rs:163:pub async fn execute_optimization(
terraphim_server/src/workflows/vm_execution.rs:11:pub async fn execute_vm_execution_demo(
terraphim_server/src/workflows/routing.rs:24:pub async fn execute_routing(
terraphim_server/src/workflows/parallel.rs:78:pub async fn execute_parallel(
terraphim_server/src/workflows/multi_agent_handlers.rs:32:    pub async fn new() -> MultiAgentResult<Self> {
terraphim_server/src/workflows/multi_agent_handlers.rs:45:    pub async fn new_with_config(config_state: ConfigState) -> MultiAgentResult<Self> {
terraphim_server/src/workflows/multi_agent_handlers.rs:164:    pub async fn execute_prompt_chain(
terraphim_server/src/workflows/multi_agent_handlers.rs:322:    pub async fn execute_routing(
terraphim_server/src/workflows/multi_agent_handlers.rs:427:    pub async fn execute_parallelization(
terraphim_server/src/workflows/multi_agent_handlers.rs:582:    pub async fn execute_orchestration(
terraphim_server/src/workflows/multi_agent_handlers.rs:751:    pub async fn execute_optimization(
terraphim_server/src/workflows/multi_agent_handlers.rs:1252:    pub async fn execute_vm_execution_demo(
terraphim_server/src/workflows/orchestration.rs:169:pub async fn execute_orchestration(
terraphim_server/src/workflows/websocket.rs:50:pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
terraphim_server/src/workflows/websocket.rs:480:pub async fn broadcast_workflow_event(
terraphim_server/src/workflows/websocket.rs:497:pub async fn notify_workflow_progress(
terraphim_server/src/workflows/websocket.rs:514:pub async fn notify_workflow_completion(
terraphim_server/src/workflows/websocket.rs:537:pub async fn notify_workflow_started(
terraphim_server/src/workflows/websocket.rs:553:pub async fn websocket_health_check() -> serde_json::Value {
terraphim_server/src/workflows/websocket.rs:576:pub fn get_websocket_stats(sessions: &HashMap<String, WebSocketSession>) -> serde_json::Value {
terraphim_server/src/workflows/mod.rs:121:pub fn create_router() -> Router<AppState> {
terraphim_server/src/workflows/mod.rs:201:pub fn generate_workflow_id() -> String {
terraphim_server/src/workflows/mod.rs:205:pub async fn update_workflow_status(
terraphim_server/src/workflows/mod.rs:241:pub async fn create_workflow_session(
terraphim_server/src/workflows/mod.rs:275:pub async fn complete_workflow_session(
terraphim_server/src/workflows/mod.rs:308:pub async fn fail_workflow_session(
```

## terraphim_tracker

### Types
```rust
crates/terraphim_tracker/src/lib.rs:23:pub struct Issue {
crates/terraphim_tracker/src/lib.rs:54:pub struct BlockerRef {
crates/terraphim_tracker/src/lib.rs:65:pub trait IssueTracker: Send + Sync {
crates/terraphim_tracker/src/lib.rs:78:pub enum TrackerError {
crates/terraphim_tracker/src/lib.rs:96:pub type Result<T> = std::result::Result<T, TrackerError>;
crates/terraphim_tracker/src/gitea.rs:14:pub enum ClaimResult {
crates/terraphim_tracker/src/gitea.rs:32:pub enum ClaimStrategy {
crates/terraphim_tracker/src/gitea.rs:46:pub enum MergeStyle {
crates/terraphim_tracker/src/gitea.rs:71:pub struct GiteaMergeResult {
crates/terraphim_tracker/src/gitea.rs:83:pub struct GiteaConfig {
crates/terraphim_tracker/src/gitea.rs:119:pub enum StatusState {
crates/terraphim_tracker/src/gitea.rs:165:pub struct GiteaTracker {
crates/terraphim_tracker/src/gitea.rs:176:pub struct GiteaIssue {
crates/terraphim_tracker/src/gitea.rs:190:pub struct GiteaLabel {
crates/terraphim_tracker/src/gitea.rs:196:pub struct IssueComment {
crates/terraphim_tracker/src/gitea.rs:209:pub struct CommentUser {
crates/terraphim_tracker/src/gitea.rs:214:pub type GiteaComment = IssueComment;
crates/terraphim_tracker/src/gitea.rs:1256:pub struct GiteaPrSummary {
crates/terraphim_tracker/src/pagerank.rs:7:pub struct PagerankScore {
crates/terraphim_tracker/src/pagerank.rs:26:pub struct ReadyResponse {
crates/terraphim_tracker/src/pagerank.rs:38:pub struct PagerankClient {
crates/terraphim_tracker/src/linear.rs:13:pub struct LinearConfig {
crates/terraphim_tracker/src/linear.rs:27:pub struct LinearTracker {
```

### Functions
```rust
crates/terraphim_tracker/src/gitea.rs:57:    pub fn as_str(&self) -> &'static str {
crates/terraphim_tracker/src/gitea.rs:99:    pub fn new(base_url: String, token: String, owner: String, repo: String) -> Self {
crates/terraphim_tracker/src/gitea.rs:132:    pub fn as_str(&self) -> &'static str {
crates/terraphim_tracker/src/gitea.rs:217:    pub fn new(config: GiteaConfig) -> Result<Self> {
crates/terraphim_tracker/src/gitea.rs:241:    pub fn with_status_backoff(mut self, backoff: Vec<Duration>) -> Self {
crates/terraphim_tracker/src/gitea.rs:247:    pub fn owner(&self) -> &str {
crates/terraphim_tracker/src/gitea.rs:252:    pub fn repo(&self) -> &str {
crates/terraphim_tracker/src/gitea.rs:297:    pub async fn fetch_issue(&self, issue_number: u64) -> Result<GiteaIssue> {
crates/terraphim_tracker/src/gitea.rs:359:    pub async fn fetch_open_issues(&self) -> Result<Vec<Issue>> {
crates/terraphim_tracker/src/gitea.rs:364:    pub async fn post_comment(&self, issue_number: u64, body: &str) -> Result<IssueComment> {
crates/terraphim_tracker/src/gitea.rs:388:    pub async fn create_issue(
crates/terraphim_tracker/src/gitea.rs:421:    pub async fn assign_issue(&self, issue_number: u64, assignees: &[&str]) -> Result<()> {
crates/terraphim_tracker/src/gitea.rs:447:    pub async fn fetch_issue_assignees(&self, issue_number: u64) -> Result<Vec<String>> {
crates/terraphim_tracker/src/gitea.rs:482:    pub async fn search_issues_by_title(&self, keyword: &str) -> Result<Vec<u64>> {
crates/terraphim_tracker/src/gitea.rs:506:    pub async fn fetch_comments(
crates/terraphim_tracker/src/gitea.rs:548:    pub async fn fetch_repo_comments(
crates/terraphim_tracker/src/gitea.rs:557:    pub async fn fetch_repo_comments_page(
crates/terraphim_tracker/src/gitea.rs:621:    pub async fn list_open_prs(&self) -> Result<Vec<GiteaPrSummary>> {
crates/terraphim_tracker/src/gitea.rs:672:    pub async fn merge_pull(
crates/terraphim_tracker/src/gitea.rs:765:    pub async fn claim_issue(
crates/terraphim_tracker/src/gitea.rs:1000:    pub async fn verify_assignment(
crates/terraphim_tracker/src/gitea.rs:1070:    pub async fn set_commit_status(
crates/terraphim_tracker/src/pagerank.rs:46:    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
crates/terraphim_tracker/src/pagerank.rs:55:    pub async fn fetch_ready(&self, owner: &str, repo: &str) -> crate::Result<ReadyResponse> {
crates/terraphim_tracker/src/pagerank.rs:89:    pub fn merge_scores(issues: &mut [crate::Issue], scores: &[PagerankScore]) {
crates/terraphim_tracker/src/lib.rs:100:    pub fn is_dispatchable(&self) -> bool {
crates/terraphim_tracker/src/lib.rs:108:    pub fn all_blockers_terminal(&self, terminal_states: &[String]) -> bool {
crates/terraphim_tracker/src/linear.rs:36:    pub fn new(config: LinearConfig) -> Result<Self> {
```

