// API Reference Snippets
// Generated automatically from documented public APIs


// === terraphim_agent ===

// File: terraphim_agent/src/listener.rs
/// Configuration for the shell dispatch bridge.
///
/// When present in `ListenerConfig`, enables executing terraphim-agent
/// subcommands from `@adf:` mention context and posting results back
/// as Gitea comments.
pub struct DispatchConfig {

// File: terraphim_agent/src/listener.rs
/// Runtime for a single identity-bound listener.
pub struct ListenerRuntime {
    config: ListenerConfig,
    tracker: terraphim_tracker::gitea::GiteaTracker,
    parser: terraphim_orchestrator::adf_commands::AdfCommandParser,
    accepted_target_names: BTreeSet<String>,
    repo_full_name: String,

// File: terraphim_agent/src/guard_patterns.rs
/// Three-valued guard decision: Allow, Sandbox, or Block
pub enum GuardDecision {
    Allow,
    Sandbox,
    Block,
}

// File: terraphim_agent/src/guard_patterns.rs
/// Result of checking a command against guard patterns
pub struct GuardResult {

// File: terraphim_agent/src/guard_patterns.rs
/// Guard that checks commands against destructive patterns using terraphim
/// thesaurus-driven Aho-Corasick matching.
pub struct CommandGuard {
    destructive_thesaurus: Thesaurus,
    allowlist_thesaurus: Thesaurus,
    suspicious_thesaurus: Thesaurus,
}
impl Default for CommandGuard {

// File: terraphim_agent/src/main.rs
/// Hook types for Claude Code integration
pub enum HookType {

// File: terraphim_agent/src/main.rs
/// Boundary mode for text replacement
pub enum BoundaryMode {

// File: terraphim_agent/src/tui_backend.rs
/// Backend for TUI operations, supporting both local (offline) and remote (server) modes.
pub enum TuiBackend {

// File: terraphim_agent/src/mcp_tool_index.rs
/// Index of MCP tools for searchable discovery.
///
/// The index stores tools and provides fast search capabilities using
/// terraphim_automata's Aho-Corasick pattern matching against tool names
/// and descriptions.
pub struct McpToolIndex {
    tools: Vec<McpToolEntry>,
    index_path: PathBuf,
}
impl McpToolIndex {

// File: terraphim_agent/src/service.rs
/// Result of connectivity check
pub struct ConnectivityResult {

// File: terraphim_agent/src/service.rs
/// Fuzzy suggestion result
pub struct FuzzySuggestion {

// File: terraphim_agent/src/service.rs
/// Checklist validation result
pub struct ChecklistResult {

// File: terraphim_agent/src/kg_validation.rs
/// A single validation finding from KG pattern matching.
pub struct ValidationFinding {

// File: terraphim_agent/src/kg_validation.rs
/// Result of KG-based command validation.
pub struct KgValidationResult {

// File: terraphim_agent/src/kg_validation.rs
/// Validate a command against the KG thesaurus.
///
/// Loads the KG thesaurus from `docs/src/kg/*.md` files (cached after first call),
/// then uses Aho-Corasick matching to find terms in the command that have
/// known canonical replacements.
///
/// This function is fail-open: any errors during thesaurus loading or matching
/// result in an empty `KgValidationResult` rather than an error.
pub fn validate_command_against_kg(command: &str) -> KgValidationResult {
    let thesaurus_opt = VALIDATION_KG_THESAURUS.get_or_init(|| {

// File: terraphim_agent/src/kg_validation.rs
/// Validate a command against a provided thesaurus (useful for testing).
///
/// This function is the core matching logic, separated from the global cache
/// so it can be tested with custom thesauruses.
pub fn validate_command_with_thesaurus(command: &str, thesaurus: Thesaurus) -> KgValidationResult {
    let matches = match terraphim_automata::find_matches(command, thesaurus, false) {

// File: terraphim_agent/src/robot/schema.rs
/// Standard response envelope for all robot mode outputs
pub struct RobotResponse<T: Serialize> {

// File: terraphim_agent/src/robot/schema.rs
/// Metadata about the response
pub struct ResponseMeta {

// File: terraphim_agent/src/robot/schema.rs
/// Information about auto-corrected commands
pub struct AutoCorrection {

// File: terraphim_agent/src/robot/schema.rs
/// Pagination information
pub struct Pagination {

// File: terraphim_agent/src/robot/schema.rs
/// Token budget tracking
pub struct TokenBudget {

// File: terraphim_agent/src/robot/schema.rs
/// Structured error information
pub struct RobotError {

// File: terraphim_agent/src/robot/schema.rs
/// Search results data structure
pub struct SearchResultsData {

// File: terraphim_agent/src/robot/schema.rs
/// Individual search result
pub struct SearchResultItem {

// File: terraphim_agent/src/robot/schema.rs
/// Capabilities response data
pub struct CapabilitiesData {

// File: terraphim_agent/src/robot/schema.rs
/// Feature flags
pub struct FeatureFlags {

// File: terraphim_agent/src/robot/schema.rs
/// Index status information
pub struct IndexStatus {

// File: terraphim_agent/src/robot/output.rs
/// Output format selection
pub enum OutputFormat {

// File: terraphim_agent/src/robot/output.rs
/// Field selection mode for output
pub enum FieldMode {

// File: terraphim_agent/src/robot/output.rs
/// Robot mode configuration
pub struct RobotConfig {

// File: terraphim_agent/src/robot/output.rs
/// Formatter for robot mode output
pub struct RobotFormatter {
    config: RobotConfig,
}
impl RobotFormatter {

// File: terraphim_agent/src/robot/exit_codes.rs
/// Exit codes for terraphim-agent robot mode
pub enum ExitCode {

// File: terraphim_agent/src/robot/docs.rs
/// Self-documentation provider
pub struct SelfDocumentation {
    commands: Vec<CommandDoc>,
}
impl SelfDocumentation {

// File: terraphim_agent/src/robot/docs.rs
/// Capabilities summary
pub struct Capabilities {

// File: terraphim_agent/src/robot/docs.rs
/// Documentation for a single command
pub struct CommandDoc {

// File: terraphim_agent/src/robot/docs.rs
/// Documentation for a command argument
pub struct ArgumentDoc {

// File: terraphim_agent/src/robot/docs.rs
/// Documentation for a command flag
pub struct FlagDoc {

// File: terraphim_agent/src/robot/docs.rs
/// Documentation for a command example
pub struct ExampleDoc {

// File: terraphim_agent/src/forgiving/aliases.rs
/// Registry for command aliases
pub struct AliasRegistry {
    aliases: HashMap<String, String>,
}
impl AliasRegistry {

// File: terraphim_agent/src/forgiving/parser.rs
/// Result of parsing with the forgiving parser
pub enum ParseResult {

// File: terraphim_agent/src/forgiving/parser.rs
/// Forgiving command parser with typo tolerance
pub struct ForgivingParser {

// File: terraphim_agent/src/forgiving/suggestions.rs
/// A command suggestion with similarity score
pub struct CommandSuggestion {

// File: terraphim_agent/src/forgiving/suggestions.rs
/// Find similar commands from a list of known commands
pub fn find_similar_commands(
    input: &str,
    known_commands: &[&str],
    max_suggestions: usize,
) -> Vec<CommandSuggestion> {

// File: terraphim_agent/src/forgiving/suggestions.rs
/// Find the best matching command if it's a high-confidence match
pub fn find_best_match(input: &str, known_commands: &[&str]) -> Option<CommandSuggestion> {
    let suggestions = find_similar_commands(input, known_commands, 1);

// File: terraphim_agent/src/forgiving/suggestions.rs
/// Calculate edit distance between two strings
pub fn edit_distance(a: &str, b: &str) -> usize {
    levenshtein(a, b)
}

// File: terraphim_agent/src/forgiving/suggestions.rs
/// Calculate Jaro-Winkler similarity between two strings
pub fn similarity(a: &str, b: &str) -> f64 {
    jaro_winkler(a, b)
}
#[cfg(test)]
mod tests {

// File: terraphim_agent/src/shared_learning/wiki_sync.rs
/// Errors that can occur during wiki sync
pub enum WikiSyncError {
    #[error("gitea-robot command failed: {0}")]

// File: terraphim_agent/src/shared_learning/wiki_sync.rs
/// Configuration for Gitea wiki client
pub struct GiteaWikiConfig {

// File: terraphim_agent/src/shared_learning/wiki_sync.rs
/// Client for Gitea wiki operations
pub struct GiteaWikiClient {
    config: GiteaWikiConfig,
}

// File: terraphim_agent/src/shared_learning/wiki_sync.rs
/// Result of a wiki sync operation
pub enum SyncResult {

// File: terraphim_agent/src/shared_learning/wiki_sync.rs
/// Sync service that periodically syncs learnings to Gitea wiki
pub struct WikiSyncService {
    client: GiteaWikiClient,
}
#[allow(dead_code)]
impl WikiSyncService {

// File: terraphim_agent/src/shared_learning/wiki_sync.rs
/// Report of a wiki sync operation
pub struct WikiSyncReport {

// File: terraphim_agent/src/shared_learning/markdown_store.rs
/// Configuration for the markdown learning store
pub struct MarkdownStoreConfig {

// File: terraphim_agent/src/shared_learning/markdown_store.rs
/// A markdown-based learning store that saves learnings as files with YAML frontmatter
pub struct MarkdownLearningStore {
    config: MarkdownStoreConfig,
}

// File: terraphim_agent/src/shared_learning/injector.rs
/// Configuration for the learning injector
pub struct InjectorConfig {

// File: terraphim_agent/src/shared_learning/injector.rs
/// Result of an injection poll
pub struct InjectionResult {

// File: terraphim_agent/src/shared_learning/injector.rs
/// Learning injector that polls shared learnings and injects relevant ones
pub struct LearningInjector {
    config: InjectorConfig,
}
impl LearningInjector {

// File: terraphim_agent/src/shared_learning/store.rs
/// BM25 scoring for text similarity
pub struct Bm25Scorer {
    avg_doc_len: f64,
    total_docs: usize,
    idf_cache: HashMap<String, f64>,
}
impl Bm25Scorer {

// File: terraphim_agent/src/onboarding/mod.rs
/// Errors that can occur during onboarding
pub enum OnboardingError {

// File: terraphim_agent/src/onboarding/mod.rs
/// List all available templates
pub fn list_templates() -> Vec<ConfigTemplate> {
    TemplateRegistry::new().list().to_vec()
}
#[cfg(test)]
mod tests {

// File: terraphim_agent/src/onboarding/templates.rs
/// A pre-built configuration template for quick start
pub struct ConfigTemplate {

// File: terraphim_agent/src/onboarding/templates.rs
/// Registry of all available templates
pub struct TemplateRegistry {
    templates: Vec<ConfigTemplate>,
}
impl Default for TemplateRegistry {

// File: terraphim_agent/src/onboarding/wizard.rs
/// Result of running the setup wizard
pub enum SetupResult {

// File: terraphim_agent/src/onboarding/wizard.rs
/// Mode for running the setup wizard
pub enum SetupMode {

// File: terraphim_agent/src/onboarding/wizard.rs
/// Quick start menu choices
pub enum QuickStartChoice {

// File: terraphim_agent/src/onboarding/wizard.rs
/// Apply a template directly without interactive wizard
///
/// # Arguments
/// * `template_id` - ID of the template to apply
/// * `custom_path` - Optional custom path override
///
/// # Returns
/// The configured Role or an error
pub fn apply_template(
    template_id: &str,
    custom_path: Option<&str>,
) -> Result<Role, OnboardingError> {

// File: terraphim_agent/src/onboarding/wizard.rs
/// Run the interactive setup wizard
///
/// # Arguments
/// * `mode` - Whether this is first-run or add-role mode
///
/// # Returns
/// SetupResult indicating what the user chose
pub async fn run_setup_wizard(mode: SetupMode) -> Result<SetupResult, OnboardingError> {
    #[cfg(feature = "repl-interactive")]
    {

// File: terraphim_agent/src/onboarding/prompts.rs
/// Result that can include a "go back" navigation
pub enum PromptResult<T> {
    Value(T),
    Back,
}

// File: terraphim_agent/src/onboarding/prompts.rs
/// Prompt for role basic info (name, shortname)
pub fn prompt_role_basics() -> Result<PromptResult<(String, Option<String>)>, OnboardingError> {
    let theme = ColorfulTheme::default();

// File: terraphim_agent/src/onboarding/prompts.rs
/// Prompt for theme selection
pub fn prompt_theme() -> Result<PromptResult<String>, OnboardingError> {
    let theme = ColorfulTheme::default();

// File: terraphim_agent/src/onboarding/prompts.rs
/// Prompt for relevance function selection
pub fn prompt_relevance_function() -> Result<PromptResult<RelevanceFunction>, OnboardingError> {
    let theme = ColorfulTheme::default();

// File: terraphim_agent/src/onboarding/prompts.rs
/// Prompt for haystack configuration (can add multiple)
pub fn prompt_haystacks() -> Result<PromptResult<Vec<Haystack>>, OnboardingError> {
    let mut haystacks = Vec::new();

// File: terraphim_agent/src/onboarding/prompts.rs
/// Prompt for LLM provider configuration
pub fn prompt_llm_config() -> Result<PromptResult<LlmConfig>, OnboardingError> {
    let theme = ColorfulTheme::default();

// File: terraphim_agent/src/onboarding/prompts.rs
/// LLM configuration from wizard
pub struct LlmConfig {

// File: terraphim_agent/src/onboarding/prompts.rs
/// Prompt for knowledge graph configuration
pub fn prompt_knowledge_graph() -> Result<PromptResult<Option<KnowledgeGraph>>, OnboardingError> {
    let theme = ColorfulTheme::default();

// File: terraphim_agent/src/onboarding/validation.rs
/// Validation errors that can occur
pub enum ValidationError {

// File: terraphim_agent/src/onboarding/validation.rs
/// Validate a role configuration
///
/// # Returns
/// - `Ok(())` if validation passes
/// - `Err(Vec<ValidationError>)` if any validations fail
pub fn validate_role(role: &Role) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

// File: terraphim_agent/src/onboarding/validation.rs
/// Validate a haystack configuration
pub fn validate_haystack(haystack: &Haystack) -> Result<(), ValidationError> {
    if haystack.location.trim().is_empty() {

// File: terraphim_agent/src/onboarding/validation.rs
/// Validate knowledge graph configuration
pub fn validate_knowledge_graph(kg: &KnowledgeGraph) -> Result<(), ValidationError> {
    let has_remote = kg.automata_path.is_some();

// File: terraphim_agent/src/onboarding/validation.rs
/// Check if a path exists on the filesystem
///
/// Handles tilde expansion for home directory
pub fn path_exists(path: &str) -> bool {
    let expanded = expand_tilde(path);

// File: terraphim_agent/src/onboarding/validation.rs
/// Expand tilde (~) to home directory
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {

// File: terraphim_agent/src/onboarding/validation.rs
/// Validate that a URL is well-formed
pub fn validate_url(url: &str) -> Result<(), ValidationError> {
    if !url.starts_with("http://") && !url.starts_with("https://") {

// File: terraphim_agent/src/repl/file_operations.rs
/// File classification types based on content analysis
pub enum FileCategory {

// File: terraphim_agent/src/repl/file_operations.rs
/// Semantic metadata extracted from file content
pub struct SemanticMetadata {

// File: terraphim_agent/src/repl/file_operations.rs
/// Named entity extracted from file content
pub struct FileEntity {

// File: terraphim_agent/src/repl/file_operations.rs
/// Position reference within a file
pub struct FilePosition {

// File: terraphim_agent/src/repl/file_operations.rs
/// File relationship analysis results
pub struct FileRelationships {

// File: terraphim_agent/src/repl/file_operations.rs
/// Similar file with similarity metrics
pub struct SimilarFile {

// File: terraphim_agent/src/repl/file_operations.rs
/// Types of similarity between files
pub enum SimilarityType {

// File: terraphim_agent/src/repl/file_operations.rs
/// Related file with relationship metrics
pub struct RelatedFile {

// File: terraphim_agent/src/repl/file_operations.rs
/// Types of relationships between files
pub enum RelationshipType {

// File: terraphim_agent/src/repl/file_operations.rs
/// File reference information
pub struct FileReference {

// File: terraphim_agent/src/repl/file_operations.rs
/// Types of file references
pub enum ReferenceType {

// File: terraphim_agent/src/repl/file_operations.rs
/// File search result with semantic information
pub struct FileSearchResult {

// File: terraphim_agent/src/repl/file_operations.rs
/// Match information for search results
pub struct FileMatch {

// File: terraphim_agent/src/repl/file_operations.rs
/// Types of file matches
pub enum MatchType {

// File: terraphim_agent/src/repl/file_operations.rs
/// File indexing status and statistics
pub struct FileIndexStatus {

// File: terraphim_agent/src/repl/file_operations.rs
/// Current indexing operation status
pub enum IndexingStatus {

// File: terraphim_agent/src/repl/file_operations.rs
/// File operation configuration
pub struct FileOperationConfig {

// File: terraphim_agent/src/repl/file_operations.rs
/// File analysis request
pub struct FileAnalysisRequest {

