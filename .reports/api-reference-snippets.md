# API Reference Snippets

## `terraphim_agent`

### `pub struct DispatchConfig`

When present in `ListenerConfig`, enables executing terraphim-agent  Configuration for the shell dispatch bridge.

### `pub fn for_identity`

terraphim-agent subcommands from @adf: mention context.

### `pub struct ListenerRuntime`

Runtime for a single identity-bound listener.

### `pub enum GuardDecision`

Three-valued guard decision: Allow, Sandbox, or Block Default suspicious patterns thesaurus (embedded at compile time) Default allowlist thesaurus (embedded at compile time)

### `pub struct GuardResult`

Result of checking a command against guard patterns

### `pub fn allow`

Create an "allow" result The pattern that matched (only present if not Allow) The original command that was checked

### `pub fn block`

Create a "block" result

### `pub fn sandbox`

Create a "sandbox" result

### `pub struct CommandGuard`

thesaurus-driven Aho-Corasick matching. Guard that checks commands against destructive patterns using terraphim

### `pub fn new`

Create a new command guard with default embedded thesauruses


_Total documented items: 461_

## `terraphim_orchestrator`

### `pub struct ReviewPrRequest`

have to know about the dispatcher enum variant shape. [`crate::dispatcher::DispatchTask::ReviewPr`] so the helpers below don't Per-dispatch metadata for a PR-review task, mirroring

### `pub fn find_pr_reviewer`

Returns `None` when no matching agent is configured. Step E lands the  Locate the `pr-reviewer` [`AgentDefinition`] for a given project.

### `pub fn build_review_task`

 ultimately to the spawned pr-reviewer process. Build the task prompt fed into [`RoutingDecisionEngine::decide_route`] and

### `pub fn pr_env_overrides`

Keys use the `ADF_PR_*` prefix so skills running inside the agent can key  Environment variables the pr-reviewer process receives for a given PR.

### `pub fn layer_pr_env`

set (e.g. `ADF_PROJECT_ID`, `GITEA_OWNER`, `GITEA_REPO`). [`SpawnContext`] without clobbering existing keys the orchestrator already Layer the per-PR `ADF_PR_*` env overrides on top of a base

### `pub fn is_project_paused`

Return `true` when a pause flag file exists for the given project id. project-level pause. Default number of consecutive `project-meta` failures that trips the

### `pub fn touch_pause_flag`

success. Errors include missing parent dir permissions. Create the pause flag file for a project. Returns the final path on paused by this mechanism).

### `pub enum ShouldPause`

Outcome returned by [`ProjectFailureCounter::record_project_meta_result`].

### `pub fn is_project_meta_agent`

`project-meta` agent? Heuristic check: does this agent definition correspond to a per-project until the counter is reset by a success.

### `pub struct ProjectFailureCounter`

`project-meta-odilo`, `project-meta-digital-twins`). The suffix form supports multi-instance deployments (e.g. Exact name `project-meta` or any `project-meta-<suffix>` form qualifies.


_Total documented items: 473_

## `terraphim_service`

### `pub struct ContextConfig`

Configuration for the context management service

### `pub struct ContextManager`

Service for managing LLM conversation contexts

### `pub fn new`

Create a new context manager In-memory cache of recent conversations Service for managing LLM conversation contexts

### `pub fn get_conversation`

Get a conversation by ID

### `pub fn list_conversations`

List conversation summaries Get a conversation by ID

### `pub fn add_message`

Add a message to a conversation

### `pub fn add_context`

Add context to a conversation

### `pub fn delete_context`

Delete a context item from a conversation

### `pub fn update_context`

Update a context item in a conversation

### `pub fn create_search_context`

Create context item from search results


_Total documented items: 135_

## `terraphim_types`

### `pub enum MedicalNodeType`

Covers clinical entities (diseases, drugs, symptoms), molecular biology  Node types for medical/biomedical knowledge graphs.

### `pub enum MedicalEdgeType`

- PrimeKG molecular relationships (AssociatedWith, InteractsWith, etc.) - Core relationships (IsA, Treats, Causes, etc.) Organized by category:

### `pub struct MedicalNodeMetadata`

Stores arbitrary key-value metadata associated with a medical knowledge  Metadata container for medical nodes.

### `pub fn new`

Arbitrary key-value metadata or provenance information. graph node, such as source database identifiers, confidence scores,

### `pub fn insert`

Insert a key-value pair into the metadata Create a new empty metadata container

### `pub fn get`

Get a value by key Insert a key-value pair into the metadata

### `pub fn is_empty`

Check if the metadata is empty Get a value by key

### `pub fn len`

Get the number of metadata entries Check if the metadata is empty

### `pub struct HgncGene`

Handles exact matches, aliases, and fuzzy matching for gene names. Specialized gene normalization for HGNC (HUGO Gene Nomenclature Committee) gene symbols. 

### `pub struct HgncNormalizer`

Gene family Gene name/description Previous/alias symbols


_Total documented items: 285_

## `terraphim_automata`

### `pub struct UmlsConcept`

UMLS Concept with CUI and associated terms from TSV format for fast entity extraction using Aho-Corasick automaton.

### `pub fn new`

Preferred term (first term encountered or shortest term) All terms associated with this concept Concept Unique Identifier (e.g., "C0004238")

### `pub fn add_term`

Add a term to this concept Create a new UMLS concept with initial term

### `pub struct UmlsDataset`

UMLS dataset containing all concepts

### `pub fn new`

Create an empty UMLS dataset Total number of terms (including duplicates across concepts) Concepts indexed by CUI

### `pub fn from_tsv`

Format: term<TAB>cui (one mapping per line)  Load UMLS data from TSV file

### `pub fn add_term`

Add a term-CUI mapping to the dataset

### `pub fn concept_count`

Get total number of unique concepts

### `pub fn get_all_terms`

Get all terms as a flat list for automaton building Get total number of unique concepts

### `pub fn get_concept`

Get concept by CUI


_Total documented items: 100_

