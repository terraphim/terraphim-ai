//! API Reference Snippets -- Paste into source files
//!
//! This file contains rustdoc comment templates for undocumented public items
//! identified in the documentation gap report. Copy-paste into the relevant
//! source locations.

// ============================================================================
// terraphim_server/src/api.rs
// ============================================================================

/// Stream of search results for server-sent events.
pub type SearchResultsStream = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

/// Paginated search results returned by the search endpoint.
pub struct SearchResponse {

/// DTO representing a node in the role graph.
pub struct GraphNodeDto {

/// DTO representing an edge in the role graph.
pub struct GraphEdgeDto {

/// Response wrapper for role-graph queries.
pub struct RoleGraphResponseDto {

/// Query parameters for role-graph exploration.
pub struct RoleGraphQuery {

/// Response payload listing available OpenRouter models.
pub struct OpenRouterModelsResponse {

/// Autocomplete suggestions for search queries.
pub struct AutocompleteResponse {

/// Single autocomplete suggestion entry.
pub struct AutocompleteSuggestion {

// ============================================================================
// terraphim_server/src/workflows/websocket.rs
// ============================================================================

/// Active WebSocket session state.
pub struct WebSocketSession {

/// Axum handler for WebSocket upgrade requests.
///
/// Handles the HTTP upgrade handshake and spawns the WebSocket loop.
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {

/// Broadcast a generic workflow event to all connected WebSocket clients.
///
/// # Arguments
/// * `broadcaster` -- the broadcast channel sender
/// * `workflow_id` -- identifier of the workflow emitting the event
/// * `event_type` -- event classification string
/// * `payload` -- optional JSON payload
pub async fn broadcast_workflow_event(

/// Notify connected clients of workflow progress.
///
/// Sends a partial-progress update for long-running workflows.
pub async fn notify_workflow_progress(

/// Notify connected clients that a workflow has completed.
///
/// # Arguments
/// * `success` -- whether the workflow finished successfully
/// * `result` -- optional JSON result payload
/// * `error` -- optional error message if `success` is false
pub async fn notify_workflow_completion(

/// Notify connected clients that a workflow has started.
pub async fn notify_workflow_started(

/// Return a JSON health-check payload for the WebSocket subsystem.
pub async fn websocket_health_check() -> serde_json::Value {

/// Collect aggregate statistics for all active WebSocket sessions.
pub fn get_websocket_stats(sessions: &HashMap<String, WebSocketSession>) -> serde_json::Value {

// ============================================================================
// crates/terraphim_agent/src/repl/handler.rs
// ============================================================================

/// Interactive REPL command handler.
///
/// Bridges user input to the Terraphim service layer. Supports both offline
/// (local TUI service) and server-backed modes.
pub struct ReplHandler {

/// Create a handler operating in offline mode against a local `TuiService`.
pub fn new_offline(service: TuiService) -> Self {

/// Create a handler operating in server mode against a remote API.
#[cfg(feature = "server")]
pub fn new_server(api_client: ApiClient) -> Self {

/// Run the interactive REPL loop.
///
/// Blocks until the user issues an exit command or an unrecoverable error
/// occurs. Requires the `repl` feature.
#[cfg(feature = "repl")]
pub async fn run(&mut self) -> Result<()> {

// ============================================================================
// crates/terraphim_agent/src/listener.rs
// ============================================================================

/// Identity of an agent for notification routing.
pub struct AgentIdentity {

/// Create a new agent identity from role and agent names.
pub fn new(role_name: &str, agent_name: &str) -> Self {

/// Resolve the Gitea login for this identity.
pub fn resolved_gitea_login(&self) -> Option<String> {

/// Return the list of target names this identity accepts notifications for.
pub fn accepted_target_names(&self) -> Vec<String> {

/// Classification of notification rule kinds.
pub enum NotificationRuleKind {

/// Rule defining when and how to notify an agent.
pub struct NotificationRule {

/// Policy for delegating work between agents.
pub struct DelegationPolicy {

/// Connection parameters for Gitea notification targets.
pub struct GiteaConnection {

/// Configuration for the agent notification listener.
pub struct ListenerConfig {

/// Build listener configuration for a specific agent identity.
pub fn for_identity(identity: &AgentIdentity) -> Self {

/// Validate the listener configuration.
pub fn validate(&self) -> Result<(), Vec<String>> {

/// Load listener configuration from a file path.
pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {

/// Create a new listener instance.
pub fn new(...) -> Self {

/// Run the listener event loop indefinitely.
pub async fn run_forever(&mut self) -> Result<(), Box<dyn std::error::Error>> {

/// Process a single batch of events.
pub async fn run_once(&mut self) -> Result<(), Box<dyn std::error::Error>> {

/// Poll for events once without blocking.
pub async fn poll_once(&mut self) -> Result<Vec<NotificationEvent>, Box<dyn std::error::Error>> {

/// Hand off an issue to another agent via Gitea.
pub async fn handoff_issue(

/// Hand off an issue with full context payload.
pub async fn handoff_issue_with_context(

/// Main entry point for the listener service.
pub async fn run_listener(config: ListenerConfig) -> Result<(), Box<dyn std::error::Error>> {

// ============================================================================
// crates/terraphim_agent/src/robot/budget.rs
// ============================================================================

/// Search results with token budget metadata attached.
pub struct BudgetedResults<T> {

/// Errors that can occur during token budget computation.
pub enum BudgetError {

/// Engine for enforcing token budgets on robot-mode output.
pub struct BudgetEngine {

/// Create a new budget engine with default limits.
pub fn new() -> Self {

/// Apply the budget to a set of raw results.
pub fn apply(&self, results: Vec<T>) -> Result<BudgetedResults<T>, BudgetError> {

// ============================================================================
// crates/terraphim_rolegraph/src/lib.rs
// ============================================================================

/// Errors that can occur during role graph operations.
pub enum Error {

/// Check whether the role graph contains no nodes.
pub fn is_empty(&self) -> bool {

/// Add or update a document in the role graph index.
pub fn add_or_update_document(&mut self, doc: Document) -> Result<(), Error> {

/// Split text into paragraphs using newline-based heuristics.
pub fn split_paragraphs(paragraphs: &str) -> Vec<&str> {

// ============================================================================
// crates/terraphim_rolegraph/src/input.rs
// ============================================================================

/// Test constant: single character.
pub const TEST1: &str = "...";

/// Test constant: two characters.
pub const TEST12: &str = "...";

/// Test constant: three characters.
pub const TEST123: &str = "...";

/// Test constant: four characters.
pub const TEST1234: &str = "...";

/// Test constant: five characters.
pub const TEST12345: &str = "...";

/// Test constant: full test corpus.
pub const TEST_CORPUS: &str = "...";

// ============================================================================
// crates/haystack_core/src/lib.rs
// ============================================================================

/// Trait implemented by all Haystack search providers.
///
/// Provides a uniform interface for querying external search backends
/// (JMAP, GrepApp, Atlassian, etc.).
pub trait HaystackProvider {
