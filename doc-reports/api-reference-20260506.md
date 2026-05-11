# API Reference Snippets

**Generated:** 2026-05-06T06:57:02Z

## terraphim_agent

### `pub struct CommandExecutor {`
*Location:* `src/commands/executor.rs:13`

Main command executor

### `pub struct NotificationHook {`
*Location:* `src/commands/hooks.rs:176`

Hook that sends notifications for important commands

### `pub struct BackupHook {`
*Location:* `src/commands/hooks.rs:314`

Hook that creates backups before destructive commands

### `pub struct GitHook {`
*Location:* `src/commands/hooks.rs:478`

Hook that integrates with Git for command tracking

### `pub fn create_default_hooks() -> Vec<Box<dyn CommandHook + S`
*Location:* `src/commands/hooks.rs:583`

Utility function to create a default set of hooks

### `pub fn create_development_hooks() -> Vec<Box<dyn CommandHook`
*Location:* `src/commands/hooks.rs:594`

Utility function to create hooks for development environment

### `pub fn create_production_hooks() -> Vec<Box<dyn CommandHook `
*Location:* `src/commands/hooks.rs:613`

Utility function to create hooks for production environment

### `pub async fn parse_markdown_command(`
*Location:* `src/commands/markdown_parser.rs:808`

Convenience function to parse a markdown command file

### `pub struct HookManager {`
*Location:* `src/commands/mod.rs:345`

Hook manager for organizing and executing hooks

### `pub struct FirecrackerExecutor {`
*Location:* `src/commands/modes/firecracker.rs:15`

Firecracker VM executor

### `pub struct HybridExecutor {`
*Location:* `src/commands/modes/hybrid.rs:14`

Hybrid executor that selects the best execution mode

### `pub struct LocalExecutor {`
*Location:* `src/commands/modes/local.rs:16`

Local command executor

### `pub fn create_executor(mode: ExecutionMode) -> Box<dyn Comma`
*Location:* `src/commands/modes/mod.rs:53`

Create an executor for the given execution mode

### `pub fn default_resource_usage() -> ResourceUsage {`
*Location:* `src/commands/modes/mod.rs:62`

Default resource usage when no statistics are available

### `pub struct CommandValidator {`
*Location:* `src/commands/validator.rs:70`

Command validator that checks against knowledge graph and security policies

### `pub fn find_similar_commands(`
*Location:* `src/forgiving/suggestions.rs:45`

Find similar commands from a list of known commands

### `pub fn find_best_match(input: &str, known_commands: &[&str])`
*Location:* `src/forgiving/suggestions.rs:72`

Find the best matching command if it's a high-confidence match

### `pub fn edit_distance(a: &str, b: &str) -> usize {`
*Location:* `src/forgiving/suggestions.rs:79`

Calculate edit distance between two strings

### `pub fn similarity(a: &str, b: &str) -> f64 {`
*Location:* `src/forgiving/suggestions.rs:84`

Calculate Jaro-Winkler similarity between two strings

### `pub struct CommandGuard {`
*Location:* `src/guard_patterns.rs:79`

Guard that checks commands against destructive patterns using terraphim
thesaurus-driven Aho-Corasick matching.

### `pub fn validate_command_against_kg(command: &str) -> KgValid`
*Location:* `src/kg_validation.rs:59`

Validate a command against the KG thesaurus.

Loads the KG thesaurus from `docs/src/kg/*.md` files (cached after first call),
then uses Aho-Corasick matching to find terms in the command that have
known canonical replacements.

This function is fail-open: any errors during thesaurus loading or matching
result in an empty `KgValidationResult` rather than an error.

### `pub fn validate_command_with_thesaurus(command: &str, thesau`
*Location:* `src/kg_validation.rs:77`

Validate a command against a provided thesaurus (useful for testing).

This function is the core matching logic, separated from the global cache
so it can be tested with custom thesauruses.

### `pub fn annotate_with_entities(text: &str) -> Vec<String> {`
*Location:* `src/learnings/capture.rs:833`

Annotate text with KG entities using Aho-Corasick matching.

Returns a deduplicated list of matched entity display names.
If the KG thesaurus is unavailable, returns an empty Vec (non-blocking).

### `pub fn capture_failed_command(`
*Location:* `src/learnings/capture.rs:933`

Capture a failed command as a learning document.

This function:
1. Checks if the command should be ignored
2. Redacts secrets from error output
3. Auto-suggests correction from existing learnings (optional)
4. Writes to storage location

# Arguments

* `command` - The command that failed
* `error_output` - The error output (stderr)
* `exit_code` - The exit code
* `config` - Learning capture configuration

# Returns

Path to the saved learning file, or error if capture failed.

### `pub fn capture_correction(`
*Location:* `src/learnings/capture.rs:1022`

Capture a user correction as a learning document.

# Arguments

* `correction_type` - Type of correction
* `original` - What the agent said/did
* `corrected` - What the user said instead
* `context_description` - Surrounding context
* `config` - Learning capture configuration

# Returns

Path to the saved correction file.

### `pub fn query_learnings(`
*Location:* `src/learnings/capture.rs:1166`

Query learnings by pattern (simple text search).

### `pub fn correct_learning(`
*Location:* `src/learnings/capture.rs:1194`

Add a correction to an existing learning document.

Finds the learning by exact ID or prefix match, updates the correction
field, and overwrites the markdown file.

### `pub fn list_all_entries(`
*Location:* `src/learnings/capture.rs:1298`

List all entries (learnings + corrections) from storage.

### `pub fn query_all_entries(`
*Location:* `src/learnings/capture.rs:1372`

Query all entries (learnings + corrections) by pattern.

### `pub fn query_all_entries_semantic(`
*Location:* `src/learnings/capture.rs:1410`

Query all entries by pattern, optionally including entity-based matching.

When `semantic` is true, the query pattern is also matched against
the `entities` field of learning entries using KG annotation.

### `pub fn compile_corrections_to_thesaurus(learnings_dir: &Path`
*Location:* `src/learnings/compile.rs:26`

Scan learnings directory for `correction-*.md` files, parse them,
and generate thesaurus entries from `ToolPreference` corrections.

Each `ToolPreference` correction maps:
- `original` -> the synonym/pattern to match (thesaurus key)
- `corrected` -> the normalized term (nterm value)

Non-ToolPreference corrections are silently skipped.
Returns an empty thesaurus if the directory is empty or has no
qualifying corrections.

### `pub fn merge_thesauruses(curated: Thesaurus, compiled: Thesa`
*Location:* `src/learnings/compile.rs:96`

Merge compiled corrections with an existing curated thesaurus.

Compiled corrections override curated entries with the same key
(learned preferences win over curated defaults).

### `pub fn write_thesaurus_json(`
*Location:* `src/learnings/compile.rs:127`

Write thesaurus to JSON file in the format expected by `terraphim_automata`.

The output format is:
```json
{
"name": "...",
"data": {
"pattern_to_match": {
"id": 1,
"nterm": "replacement_term",
"url": null
}
}
}
```

### `pub fn capture_from_hook(input: &HookInput) -> Result<PathBu`
*Location:* `src/learnings/hook.rs:66`

Capture learning from hook input.

Extracts the command, error output, and exit code from the hook input
and delegates to `capture_failed_command` for storage.

# Arguments

* `input` - The parsed hook input

# Returns

Path to the saved learning file, or error if capture failed/ignored.

### `pub async fn process_hook_input_with_type(`
*Location:* `src/learnings/hook.rs:87`

Process hook input with an explicit hook type.

Routes to the appropriate handler based on the hook type:
- PreToolUse: checks command against known error patterns, warns if similar to past failure
- PostToolUse: captures failed commands (original behavior)
- UserPromptSubmit: captures user corrections inline

All hook types maintain fail-open behavior: errors are logged but
never block the pipeline.

### `pub async fn install_hook(agent: AgentType) -> Result<(), In`
*Location:* `src/learnings/install.rs:165`

Install hook for the specified AI agent.

Creates a hook script in the agent's config directory that captures
failed commands and forwards them to terraphim-agent for learning.

# Arguments

* `agent` - The AI agent type to install the hook for

# Returns

Ok(()) if installation succeeds, Err(InstallError) otherwise.

# Examples

```rust,ignore
use terraphim_agent::learnings::{AgentType, install_hook};

install_hook(AgentType::Claude).await?;
```

### `pub struct ProcedureStore {`
*Location:* `src/learnings/procedure.rs:88`

Storage for captured procedures with deduplication support.

### `pub fn redact_secrets(text: &str) -> String {`
*Location:* `src/learnings/redaction.rs:64`

Redact secrets from text using regex pattern matching.

This function applies regex patterns to find and replace secret patterns
like AWS keys, API tokens, and connection strings.

# Arguments

* `text` - The text to redact

# Returns

The text with secrets replaced by `[REDACTED]` placeholders.

# Example

```
use terraphim_agent::learnings::redact_secrets;

let input = "AWS_KEY=AKIAIOSFODNN7EXAMPLE connected";
let redacted = redact_secrets(input);
assert!(redacted.contains("[AWS_KEY_REDACTED]"));
```

### `pub fn contains_secrets(text: &str) -> bool {`
*Location:* `src/learnings/redaction.rs:106`

Check if text contains potential secrets.

This is a quick check that can be used before capture to warn users.

### `pub fn replay_procedure(`
*Location:* `src/learnings/replay.rs:42`

Replay a captured procedure by executing its steps in order.

If `dry_run` is true, each step's command is printed without execution
and all steps are reported as successful.

Safety rules:
- Privileged steps are always skipped.
- Commands blocked by the guard pattern system are skipped.
- On any step failure, replay stops immediately (no further steps run).

### `pub struct ListenerRuntime {`
*Location:* `src/listener.rs:1268`

Runtime for a single identity-bound listener.

### `pub fn list_templates() -> Vec<ConfigTemplate> {`
*Location:* `src/onboarding/mod.rs:86`

List all available templates

### `pub enum PromptResult<T> {`
*Location:* `src/onboarding/prompts.rs:30`

Result that can include a "go back" navigation

### `pub fn prompt_role_basics() -> Result<PromptResult<(String, `
*Location:* `src/onboarding/prompts.rs:36`

Prompt for role basic info (name, shortname)

### `pub fn prompt_theme() -> Result<PromptResult<String>, Onboar`
*Location:* `src/onboarding/prompts.rs:82`

Prompt for theme selection

### `pub fn prompt_relevance_function() -> Result<PromptResult<Re`
*Location:* `src/onboarding/prompts.rs:102`

Prompt for relevance function selection

### `pub fn prompt_haystacks() -> Result<PromptResult<Vec<Haystac`
*Location:* `src/onboarding/prompts.rs:137`

Prompt for haystack configuration (can add multiple)

### `pub fn prompt_llm_config() -> Result<PromptResult<LlmConfig>`
*Location:* `src/onboarding/prompts.rs:368`

Prompt for LLM provider configuration

### `pub fn prompt_knowledge_graph() -> Result<PromptResult<Optio`
*Location:* `src/onboarding/prompts.rs:477`

Prompt for knowledge graph configuration

### `pub fn validate_role(role: &Role) -> Result<(), Vec<Validati`
*Location:* `src/onboarding/validation.rs:43`

Validate a role configuration

# Returns
- `Ok(())` if validation passes
- `Err(Vec<ValidationError>)` if any validations fail

### `pub fn validate_haystack(haystack: &Haystack) -> Result<(), `
*Location:* `src/onboarding/validation.rs:78`

Validate a haystack configuration

### `pub fn validate_knowledge_graph(kg: &KnowledgeGraph) -> Resu`
*Location:* `src/onboarding/validation.rs:113`

Validate knowledge graph configuration

### `pub fn path_exists(path: &str) -> bool {`
*Location:* `src/onboarding/validation.rs:139`

Check if a path exists on the filesystem

Handles tilde expansion for home directory

### `pub fn expand_tilde(path: &str) -> String {`
*Location:* `src/onboarding/validation.rs:145`

Expand tilde (~) to home directory

### `pub fn validate_url(url: &str) -> Result<(), ValidationError`
*Location:* `src/onboarding/validation.rs:159`

Validate that a URL is well-formed

### `pub fn apply_template(`
*Location:* `src/onboarding/wizard.rs:119`

Apply a template directly without interactive wizard

# Arguments
* `template_id` - ID of the template to apply
* `custom_path` - Optional custom path override

# Returns
The configured Role or an error

### `pub async fn run_setup_wizard(mode: SetupMode) -> Result<Set`
*Location:* `src/onboarding/wizard.rs:160`

Run the interactive setup wizard

# Arguments
* `mode` - Whether this is first-run or add-role mode

# Returns
SetupResult indicating what the user chose

### `pub struct FileOperationBuilder {`
*Location:* `src/repl/file_operations.rs:810`

File operation builder for creating analysis requests

### `pub async fn run_repl_offline_mode() -> Result<()> {`
*Location:* `src/repl/handler.rs:2756`

Run REPL in offline mode

### `pub async fn run_repl_server_mode(server_url: &str) -> Resul`
*Location:* `src/repl/handler.rs:2764`

Run REPL in server mode

### `pub struct WebOperationBuilder {`
*Location:* `src/repl/web_operations.rs:261`

Builder for web operation requests

### `pub struct RobotFormatter {`
*Location:* `src/robot/output.rs:174`

Formatter for robot mode output

### `pub struct Bm25Scorer {`
*Location:* `src/shared_learning/store.rs:55`

BM25 scoring for text similarity

### `pub struct GiteaWikiClient {`
*Location:* `src/shared_learning/wiki_sync.rs:86`

Client for Gitea wiki operations


## terraphim_automata

### `pub fn build_autocomplete_index(`
*Location:* `src/autocomplete.rs:95`

Build autocomplete index from existing thesaurus

### `pub fn autocomplete_search(`
*Location:* `src/autocomplete.rs:183`

Perform autocomplete search with prefix

### `pub fn fuzzy_autocomplete_search_levenshtein(`
*Location:* `src/autocomplete.rs:251`

Fuzzy autocomplete search using Levenshtein edit distance (baseline comparison)

Uses Levenshtein distance calculation for baseline comparison with the default
Jaro-Winkler fuzzy search. Levenshtein is useful when you need exact edit distance control.

### `pub fn fuzzy_autocomplete_search(`
*Location:* `src/autocomplete.rs:344`

Fuzzy autocomplete search using Jaro-Winkler similarity (DEFAULT)

Jaro-Winkler is the recommended algorithm for autocomplete because it gives extra weight
to common prefixes and handles character transpositions better. It's 2.3x faster than
Levenshtein and produces higher quality results for autocomplete scenarios.

### `pub fn serialize_autocomplete_index(index: &AutocompleteInde`
*Location:* `src/autocomplete.rs:446`

Serialize index to bytes for caching

### `pub fn deserialize_autocomplete_index(data: &[u8]) -> Result`
*Location:* `src/autocomplete.rs:459`

Deserialize index from bytes

### `pub fn compute_kg_source_hash(dir: &std::path::Path) -> std:`
*Location:* `src/builder.rs:37`

Compute a SHA-256 hash of all markdown files in a directory tree.

Files are processed in sorted order to ensure deterministic output.
The hash incorporates both the relative file path and file content,
so renames and edits are both detected.

Returns `Ok(None)` if the directory does not exist or contains no markdown files.

### `pub trait ThesaurusBuilder {`
*Location:* `src/builder.rs:86`

A ThesaurusBuilder receives a path containing
resources (e.g. files) with key-value pairs and returns a `Thesaurus`
(a dictionary with synonyms which map to higher-level concepts)

### `pub fn evaluate(ground_truth: &[GroundTruthDocument], thesau`
*Location:* `src/evaluation.rs:106`

Evaluate automata classification accuracy against ground truth.

Runs `find_matches()` on each document's text using the provided thesaurus,
then compares matched normalized term values against expected terms.

Returns overall micro-averaged metrics, per-term metrics, and systematic errors.

### `pub fn load_ground_truth(`
*Location:* `src/evaluation.rs:210`

Load ground truth documents from a JSON file.

The file must contain a JSON array of `GroundTruthDocument` objects.

### `pub type Result<T> = std::result::Result<T, TerraphimAutomat`
*Location:* `src/lib.rs:218`

Result type alias using `TerraphimAutomataError`.

### `pub fn load_thesaurus_from_json(json_str: &str) -> Result<Th`
*Location:* `src/lib.rs:319`

Load thesaurus from JSON string (sync version for WASM compatibility)

### `pub fn load_thesaurus_from_json_and_replace(`
*Location:* `src/lib.rs:325`

Load thesaurus from JSON string and replace terms using streaming matcher

### `pub fn extract_heading_from_path(path: &Path) -> Option<Stri`
*Location:* `src/markdown_directives.rs:92`

Extract the first `# Heading` from a markdown file at the given path.

Reads the file and delegates to `terraphim_markdown_parser::extract_first_heading`
for proper AST-based heading extraction. Returns `None` if the file cannot be read
or has no H1 heading.

### `pub fn replace_matches(text: &str, thesaurus: Thesaurus, lin`
*Location:* `src/matcher.rs:107`

Replace matches in text using the thesaurus.

Uses `display()` method on `NormalizedTerm` to get the case-preserved
display value for replacement output.

URLs (http, https, mailto, email addresses) are protected from replacement
to prevent corruption of links.

Patterns shorter than MIN_PATTERN_LENGTH (2) are filtered out to prevent
spurious matches at every character position.

### `pub fn extract_paragraphs_from_automata(`
*Location:* `src/matcher.rs:189`

Extract the paragraph text starting at each automata term match.

For every matched term in `text`, returns the substring from the start of the term
until the end of the containing paragraph (first blank line or end-of-text).

### `pub fn save_umls_artifact(`
*Location:* `src/medical_artifact.rs:44`

Save a UMLS artifact: header (bincode) + shard bytes, compressed with zstd

### `pub fn load_umls_artifact(path: &Path) -> anyhow::Result<(Ar`
*Location:* `src/medical_artifact.rs:89`

Load a UMLS artifact: returns (header, shard_bytes_list)

### `pub fn artifact_exists(path: &Path) -> bool {`
*Location:* `src/medical_artifact.rs:138`

Check if an artifact file exists

### `pub struct EntityExtractor {`
*Location:* `src/medical_extractor.rs:16`

Entity extractor using Aho-Corasick multi-pattern automaton

Builds an Aho-Corasick automaton from all concept terms and synonyms for
O(n + m + z) extraction where n = text length, m = total pattern length,
z = number of matches. This replaces the naive O(n*p) substring scan.

### `pub struct ShardedUmlsExtractor {`
*Location:* `src/sharded_extractor.rs:27`

Sharded UMLS extractor that distributes patterns across multiple automatons

This allows handling datasets with millions of patterns that would
otherwise exceed Aho-Corasick's state identifier limits.

### `pub fn create_concept_index(concepts: &[SnomedConcept]) -> H`
*Location:* `src/snomed.rs:99`

Create an in-memory index for quick lookup

### `pub struct UmlsExtractor {`
*Location:* `src/umls_extractor.rs:28`

UMLS entity extractor using Aho-Corasick automaton

Builds an Aho-Corasick automaton from all UMLS terms for
O(n + m + z) extraction where n = text length, m = total pattern length,
z = number of matches.

### `pub fn with_protected_urls<F>(text: &str, transform: F) -> S`
*Location:* `src/url_protector.rs:147`

Convenience function to protect URLs, apply a transformation, and restore them.


## terraphim_service

### `pub async fn auto_select_role(`
*Location:* `src/auto_route.rs:113`

Choose a role for `query` by scoring each in-memory rolegraph.

Returns a concrete role per the policies in section 3 of the design.
The function never errors; in degenerate cases (no roles configured)
returns a synthesised result with `RoleName::from("Default")` and reason
`ZeroMatchDefault`.

### `pub struct ContextManager {`
*Location:* `src/context.rs:46`

Service for managing LLM conversation contexts

### `pub fn build_llm_messages_with_context(`
*Location:* `src/context.rs:332`

Build LLM messages with context injection

### `pub struct ConversationService {`
*Location:* `src/conversation_service.rs:40`

Service for managing conversations

### `pub trait TerraphimError: std::error::Error + Send + Sync + `
*Location:* `src/error.rs:11`

Base error trait for all terraphim errors

This trait provides common functionality for all error types
including error categories and user-friendly messages.

### `pub type TerraphimResult<T> = Result<T, CommonError>;`
*Location:* `src/error.rs:261`

Result type alias using CommonError

### `pub fn get_default_client() -> &'static Client {`
*Location:* `src/http_client.rs:102`

Get the global default HTTP client with connection pooling

This client includes:
- 30-second timeout for requests
- Terraphim user agent header
- Connection pooling and keep-alive

Use this for most HTTP operations where no special configuration is needed.

### `pub fn create_client_with_timeout(timeout_secs: u64) -> reqw`
*Location:* `src/http_client.rs:110`

Get an HTTP client with custom timeout

Note: This creates a new client instance. For better performance,
prefer `get_default_client()` when possible.

### `pub fn get_api_client() -> &'static Client {`
*Location:* `src/http_client.rs:128`

Get the global API HTTP client with JSON headers and connection pooling

This client is configured for typical REST API usage:
- Shorter timeout (10 seconds) for responsive APIs
- JSON content type header
- Accept JSON responses
- Connection pooling for repeated API calls

Use this for LLM API calls and other JSON-based APIs.

### `pub fn create_custom_client(`
*Location:* `src/http_client.rs:141`

Create a custom HTTP client with specific configuration

Note: This creates a new client instance. For better performance,
prefer the global clients when possible.

Use this for specialized use cases like:
- Custom headers (API keys, authentication)
- Proxy configuration
- Custom SSL/TLS settings

### `pub fn get_scraping_client() -> &'static Client {`
*Location:* `src/http_client.rs:177`

Get the global web scraping HTTP client with connection pooling

This client is configured for scraping web pages:
- Longer timeout (60 seconds) for slow websites
- Browser-like headers to avoid blocking
- HTML content acceptance
- Connection pooling for repeated requests

Use this for web scraping operations.

### `pub fn create_default_client() -> reqwest::Result<Client> {`
*Location:* `src/http_client.rs:188`

Backwards compatibility: returns a clone of the default client

This function returns `Ok(Client)` for full backwards compatibility.
The client is cheap to clone (internally Arc-based).

### `pub fn create_api_client() -> reqwest::Result<Client> {`
*Location:* `src/http_client.rs:196`

Backwards compatibility: returns a clone of the API client

This function returns `Ok(Client)` for full backwards compatibility.
The client is cheap to clone (internally Arc-based).

### `pub fn create_scraping_client() -> reqwest::Result<Client> {`
*Location:* `src/http_client.rs:204`

Backwards compatibility: returns a clone of the scraping client

This function returns `Ok(Client)` for full backwards compatibility.
The client is cheap to clone (internally Arc-based).

### `pub struct LlmProviderDescriptor {`
*Location:* `src/llm/bridge.rs:29`

Associates a Router Provider with its executable LLM client.

### `pub struct RouterBridgeLlmClient {`
*Location:* `src/llm/bridge.rs:40`

Router-based LLM client that selects the best provider for each request.

On each call, extracts capabilities from the prompt, routes via
`terraphim_router::Router`, then executes against the matched `LlmClient`.

### `pub fn provider_from_llm_client(client: &dyn LlmClient, role`
*Location:* `src/llm/bridge.rs:153`

Build a `Provider` from an LLM client and role configuration.

### `pub fn role_wants_ai_summarize(role: &terraphim_config::Role`
*Location:* `src/llm.rs:72`

Determine if the role requests AI summarization via generic LLM config in `extra`.

### `pub fn build_llm_from_role(role: &terraphim_config::Role) ->`
*Location:* `src/llm.rs:78`

Best-effort builder that inspects role settings and returns an LLM client if configured.

### `pub fn init_logging(config: LoggingConfig) {`
*Location:* `src/logging.rs:54`

Initialize logging based on configuration preset

### `pub fn init_server_logging() {`
*Location:* `src/logging.rs:65`

Initialize production server logging

### `pub fn init_development_logging() {`
*Location:* `src/logging.rs:74`

Initialize development server logging with more verbose output

### `pub fn init_test_logging() {`
*Location:* `src/logging.rs:82`

Initialize test environment logging

### `pub fn init_integration_test_logging() {`
*Location:* `src/logging.rs:90`

Initialize integration test logging with reduced noise

### `pub fn init_custom_logging(level: log::LevelFilter) {`
*Location:* `src/logging.rs:98`

Initialize logging with custom level

### `pub fn init_env_logging() {`
*Location:* `src/logging.rs:104`

Initialize logging respecting LOG_LEVEL environment variable
Falls back to INFO level if LOG_LEVEL is not set

### `pub fn init_external_tracing_logging(verbose: bool) {`
*Location:* `src/logging.rs:129`

Initialize tracing with simple format (for external crates that use tracing)
This function is available without the tracing feature for compatibility

### `pub fn detect_logging_config() -> LoggingConfig {`
*Location:* `src/logging.rs:144`

Get appropriate logging config based on environment

### `pub fn estimate_tokens(text: &str) -> f64 {`
*Location:* `src/rate_limiter.rs:228`

Utility function to estimate token usage for text

### `pub struct SummarizationManager {`
*Location:* `src/summarization_manager.rs:16`

High-level manager for the summarization system

### `pub struct SummarizationManagerBuilder {`
*Location:* `src/summarization_manager.rs:364`

Builder for creating summarization managers with custom configurations

### `pub struct SummarizationQueue {`
*Location:* `src/summarization_queue.rs:321`

The main summarization queue

### `pub struct SummarizationWorker {`
*Location:* `src/summarization_worker.rs:94`

Background worker for processing summarization tasks


## terraphim_types

### `pub struct HgncNormalizer {`
*Location:* `src/hgnc.rs:26`

HGNC Gene Normalizer

### `pub fn extract_first_paragraph(body: &str) -> String {`
*Location:* `src/lib.rs:1032`

Extract the first paragraph from document body text.

Skips YAML frontmatter (content between `---` markers) and returns
the first non-empty line or the first paragraph.

### `pub fn deduplicate_findings(findings: Vec<ReviewFinding>) ->`
*Location:* `src/review.rs:62`

Deduplicate findings by keeping the most severe finding per (file, line, category) key.

Results are sorted by severity (descending), then file, then line.

### `pub trait LearningStore: Send + Sync {`
*Location:* `src/shared_learning.rs:129`

Synchronous trait for a learning store shared between orchestrator and agent.

Implementations may persist to DeviceStorage, markdown files, or in-memory
maps. The trait is intentionally synchronous so that `terraphim_types`
remains free of async runtime dependencies. Implementations that need
async I/O can use internal synchronisation (e.g. `tokio::runtime::Handle`).

### `pub struct InMemoryLearningStore {`
*Location:* `src/shared_learning.rs:149`

In-memory `LearningStore` for tests and development.

No persistence -- data lives only for the lifetime of the struct.
Thread-safe via `std::sync::Mutex`.

