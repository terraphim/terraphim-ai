//! LSP server implementation for Terraphim knowledge graphs.
//!
//! Implements the `LanguageServer` trait from `tower-lsp`, wiring up hover,
//! completion, diagnostics, synonym code actions, inlay hints and
//! `workspace/executeCommand` to the pure [`terraphim_lsp_core`] analysis
//! engine.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, PoisonError};

use serde_json::Value;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use terraphim_lsp_core::{
    CutId, GhostEdited, GhostError, KgEngine, LabAction, LabConfig, LabFinding, LineIndex,
    OverflowEdited, OverflowError, ReanchorOutcome, Source, StoredAlternative, StoredSpan,
    TextRange, TrimLevel, TrimPlan, TrimView, add_alternative, export_clean,
    export_knowledge_graph, ghost_availability, ghost_range, lab_findings, overflow_paragraphs,
    pull_back, reanchor_block, revive_range, select_alternative, split_annotation_block,
    stash_range, stored_anchor_ranges, stored_span_at, trim_cuts, trim_done, trim_plan_for,
    trim_view,
};
use terraphim_types::Thesaurus;

use crate::commands::{
    self, AddAlternativeArgs, DocumentArgs, ExportArgs, LabMarkArgs, TrimKeepArgs,
    TrimMakeCutsArgs, TrimNextArgs, TrimPreviewArgs,
};
use crate::completion::{build_completions, word_at_position};
use crate::convert;
use crate::diagnostics::{core_diagnostics, ghost_diagnostics, unknown_term_diagnostics};
use crate::handshake::{self, InitializeHint};
use crate::kg_analysis::analyse_kg_document;
use crate::settings::{LabSettings, LabTrigger, ServerSettings};
use crate::thesaurus::{
    LaunchOptions, effective_thesaurus_path, load_thesaurus, thesaurus_setting,
};

/// Terraphim LSP server backed by a knowledge-graph thesaurus.
///
/// The server tracks open text documents and provides:
///
/// - `textDocument/hover` - concept descriptions for matched KG terms
/// - `textDocument/completion` - thesaurus term suggestions
/// - `textDocument/diagnostic` for clients that pull, pushed
///   `publishDiagnostics` for the others (never both) - a
///   malformed `terraphim-alternatives` annotation block, faded
///   (`Unnecessary`) hints over ghosted text and, when the `unknownTerms`
///   setting is on, a warning on every unknown-word occurrence
/// - `textDocument/codeAction` - "Replace with X" for every other synonym of
///   the KG term at the cursor, keeping capitalisation and fixing `a`/`an`,
///   and "Apply fix: X" for Lab typo and punctuation marks
/// - Lab marks and trim candidates as diagnostics, computed on demand
///   (`terraphim.lab.mark`, `terraphim.trim.preview`) and, for the
///   configured `lab.actions`, on open and save; never on every keystroke
/// - the trim review (R-8.4, R-8.5): a status card
///   (`window/showMessageRequest`) with "Make the cuts"
///   (`workspace/applyEdit`), "Walk through" (`window/showDocument`) and
///   "Done", and "Keep" code actions on faded cuts
/// - `textDocument/inlayHint` - `[i/n]` after each KG term with alternatives,
///   when the `inlayHints` setting is on (off by default)
/// - `workspace/executeCommand` - the commands in [`commands::ALL`]
///
/// Settings come from `initializationOptions` and
/// `workspace/didChangeConfiguration`; see [`ServerSettings`]. The
/// thesaurus is loaded from the `thesaurus` setting, the `--thesaurus` flag
/// or `TERRAPHIM_THESAURUS` (see [`crate::thesaurus`]) and reloaded when the
/// path changes.
#[derive(Debug)]
pub struct TerraphimLspServer {
    client: Client,
    /// The knowledge graph in use. A std lock: it is only held to clone or
    /// replace the `Arc`, never across an `.await`.
    kg: std::sync::RwLock<Arc<Kg>>,
    /// The knowledge graph the server was constructed with, used while no
    /// thesaurus path is configured.
    base_kg: Arc<Kg>,
    /// Launch-time thesaurus path and home directory.
    launch: LaunchOptions,
    /// The `thesaurus` member of `initializationOptions`, kept under later
    /// `didChangeConfiguration` settings that do not name one.
    initial_thesaurus: std::sync::RwLock<Option<String>>,
    /// Bumped by every thesaurus load; a load installs its result only if
    /// no newer load started meanwhile.
    kg_loads: AtomicU64,
    cut_operations: Arc<AtomicU64>,
    documents: Arc<RwLock<HashMap<Url, OpenDocument>>>,
    /// Last acknowledged generated text, used to refuse changed output buffers.
    exported: RwLock<HashMap<Url, String>>,
    /// Whether the client accepts `WorkspaceEdit.documentChanges`
    /// (`workspace.workspaceEdit.documentChanges`), read at `initialize`.
    versioned_edits: Arc<AtomicBool>,
    /// Whether the client accepts `workspace/inlayHint/refresh`, read at
    /// `initialize`.
    inlay_hint_refresh: AtomicBool,
    /// Whether the client shows `window/showMessageRequest` with buttons
    /// (`window.showMessage.messageActionItem`), read at `initialize`. The
    /// trim status card is a prompt then, else a plain message.
    prompts: Arc<AtomicBool>,
    /// Whether the client takes `window/showDocument`
    /// (`window.showDocument.support`), read at `initialize`; "Walk
    /// through" selects the next cut with it.
    show_document: Arc<AtomicBool>,
    /// Whether the client takes `workspace/applyEdit`
    /// (`workspace.applyEdit`), read at `initialize`; "Make the cuts"
    /// sends its edit with it, else returns the edit.
    apply_edit: Arc<AtomicBool>,
    /// Whether the client can create files in a workspace edit
    /// (`workspace.workspaceEdit.resourceOperations` has `create`), read at
    /// `initialize`; the clean export is written with it.
    create_file: AtomicBool,
    /// Woken whenever a document's text is replaced, so a save that edits the
    /// block can wait for the client's change before refreshing the Lab.
    changed: Arc<tokio::sync::Notify>,
    /// Whether the client pulls diagnostics (`textDocument.diagnostic`),
    /// read at `initialize`. Then the server never pushes
    /// `publishDiagnostics`, so each diagnostic is shown once.
    pull_diagnostics: AtomicBool,
    /// Whether the client accepts `workspace/diagnostic/refresh`, decided at
    /// `initialize` from [`Self::refresh_hint`] and the typed params; used in pull mode when results change outside a
    /// client request (Lab runs, clears, settings and thesaurus changes).
    diagnostic_refresh: AtomicBool,
    /// Whether the raw `initialize` advertised refresh support under the
    /// spec key `workspace.diagnostics`, which the typed params drop (see
    /// [`crate::handshake`]). Set by the stdio entry points.
    refresh_hint: Option<bool>,
    /// Current settings. A std lock: it is never held across an `.await`.
    settings: std::sync::RwLock<ServerSettings>,
    /// The Lab engine's configuration (embedded default lists), or `None`
    /// if they failed to load.
    lab_config: Option<Arc<LabConfig>>,
}

/// A thesaurus and the engine compiled from it, swapped as one unit so
/// completion and matching never disagree.
#[derive(Debug)]
struct Kg {
    thesaurus: Thesaurus,
    engine: KgEngine,
    /// The file it was loaded from (even if loading failed and the engine
    /// is empty); `None` for the thesaurus given to the constructor.
    path: Option<PathBuf>,
}

impl Kg {
    fn empty_for(path: PathBuf) -> Self {
        Self {
            thesaurus: Thesaurus::new("empty".to_string()),
            engine: KgEngine::empty(),
            path: Some(path),
        }
    }
}

/// An open document: its full text, the version the client last sent, and
/// the Lab and trim results computed for that text.
#[derive(Debug, Clone, Default)]
struct OpenDocument {
    text: String,
    version: i32,
    /// Identity of this open/close lifecycle, preserved by snapshots and edits.
    incarnation: Arc<()>,
    /// Automatic reanchor reserved for this exact document version/incarnation.
    reanchoring: Option<u64>,
    /// Lab actions requested for this document through `terraphim.lab.mark`,
    /// sorted. Kept across edits until `terraphim.lab.clear`.
    lab_actions: Vec<LabAction>,
    /// Lab findings for the current text. Cleared on every change, because
    /// their ranges would be stale.
    lab: Vec<LabFinding>,
    /// The trim level previewed through `terraphim.trim.preview`, until
    /// `terraphim.trim.clear` (or "Trim: Original"), or until an edit
    /// leaves exactly the text "Make the cuts" produces.
    trim_level: Option<TrimLevel>,
    /// The cuts kept with `terraphim.trim.keep`. Cut ids belong to one plan,
    /// so they are forgotten with it on every change.
    trim_kept: Vec<CutId>,
    /// Every trim level planned for the current text. Cleared on every
    /// change; switching level and keeping cuts reuse it.
    trim_plan: Option<Arc<TrimPlan>>,
    /// The review of the trim level with its kept cuts, for the current
    /// text: the faded spans, the cuts and the status card numbers. Cleared
    /// on every change.
    trim: Option<Arc<TrimView>>,
    /// The text "Make the cuts" turns the current text into.
    trim_made: Option<Arc<str>>,
    /// The start (byte offset) of the span the card's "Walk through" last
    /// showed; the next one follows it.
    trim_walk: Option<usize>,
    /// A "Make the cuts" edit sent with `workspace/applyEdit` and not yet
    /// answered as failed. While one is pending no second cut is sent.
    trim_cutting: Option<PendingCut>,
    /// Bumped whenever what the Lab results depend on changes: the text,
    /// the requested actions, the trim level or the Lab settings. A Lab run
    /// installs its results only if the generation it snapshotted is still
    /// current, so runs finishing out of order never overwrite newer state.
    generation: u64,
}

/// A "Make the cuts" edit sent to the client with `workspace/applyEdit`.
///
/// The preview ends when the client confirms the edit (`applied: true`) and
/// the document has changed since the edit was built, in either order. So a
/// client whose text differs from [`OpenDocument::trim_made`] (a trailing
/// newline, say) still ends it, while typing during a request the client
/// then rejects does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingCut {
    /// Unique across preview replacements at the same document version.
    operation: u64,
    /// The version the edit was built for.
    version: i32,
    /// Whether the client answered `applied: true`.
    applied: bool,
}

/// A snapshot of one document for a Lab run.
#[derive(Debug, Clone)]
struct LabJob {
    text: String,
    version: i32,
    generation: u64,
    actions: Vec<LabAction>,
    trim_level: Option<TrimLevel>,
    trim_kept: Vec<CutId>,
    /// The plan of `text`, when one was already computed.
    trim_plan: Option<Arc<TrimPlan>>,
}

/// What became of a Lab run's results.
#[derive(Debug)]
enum Installed {
    /// Installed; the updated document.
    Yes(OpenDocument),
    /// Dropped: the document was edited (now at this version) or closed.
    Edited(Option<i32>),
    /// Dropped: the requested Lab state changed on the same version; a
    /// newer run covers it.
    Superseded,
}

/// The results of a [`LabJob`].
#[derive(Debug, Clone, Default)]
struct LabResults {
    lab: Vec<LabFinding>,
    trim: Option<TrimReview>,
}

/// A trim review computed for one text: the plan, the view of the level
/// with its kept cuts, and the text "Make the cuts" would leave.
#[derive(Debug, Clone)]
struct TrimReview {
    plan: Arc<TrimPlan>,
    view: TrimView,
    made: String,
}

impl TrimReview {
    fn compute(text: &str, plan: Arc<TrimPlan>, level: TrimLevel, kept: &[CutId]) -> Self {
        Self {
            view: trim_view(text, &plan, level, kept),
            made: trim_cuts(text, &plan, level, kept).text,
            plan,
        }
    }

    /// Install into `document` (computed for its current text).
    fn install(self, document: &mut OpenDocument) {
        document.trim_plan = Some(self.plan);
        document.trim = Some(Arc::new(self.view));
        document.trim_made = Some(self.made.into());
    }
}

impl LabJob {
    /// Run the configured and requested actions and the trim preview. CPU
    /// bound and whole-document; called on the blocking pool.
    fn run(&self, config: &LabConfig) -> LabResults {
        let lab = if self.actions.is_empty() {
            Vec::new()
        } else {
            lab_findings(&self.text, config, &self.actions)
        };
        let trim = self.trim_level.map(|level| {
            let plan = self
                .trim_plan
                .clone()
                .unwrap_or_else(|| Arc::new(trim_plan_for(&self.text, config)));
            TrimReview::compute(&self.text, plan, level, &self.trim_kept)
        });
        LabResults { lab, trim }
    }
}

/// The code-action kind of the Lab fixes.
const FIX_KIND: CodeActionKind = CodeActionKind::QUICKFIX;

/// The code-action kind of the synonym replacements.
const REPLACE_KIND: CodeActionKind = CodeActionKind::REFACTOR_REWRITE;

/// The code-action kind of the trim actions. Clients that cannot add
/// palette commands (Zed) reach the `terraphim.trim.*` commands through
/// these command-only actions.
const TRIM_KIND: CodeActionKind = CodeActionKind::new("refactor.terraphim.trim");

/// The code-action kind of "Ghost it" and "Revive" (R-5.1, R-5.2).
///
/// Not `refactor.rewrite`: the body text never changes, only the annotation
/// block does, so these are not rewrites of the writer's words. A kind of
/// their own, beside [`TRIM_KIND`], lets a client ask for or hide them with
/// `only`, and `refactor` still includes them.
const GHOST_KIND: CodeActionKind = CodeActionKind::new("refactor.terraphim.ghost");

/// The code-action kind of the Lab actions. Like [`TRIM_KIND`], command-only
/// so that Zed, which runs only the commands code actions carry, reaches
/// `terraphim.lab.mark` and `terraphim.lab.clear`.
const LAB_KIND: CodeActionKind = CodeActionKind::new("refactor.terraphim.lab");

/// The code-action kind of "Replace with" a stored alternative (R-2.2,
/// R-4.4).
///
/// Not `refactor.rewrite`: the action rewrites the annotation block as well
/// as the words (the span's `active` and anchor), so it is not a plain
/// rewrite. A kind of its own, beside [`GHOST_KIND`], lets a client ask for
/// or hide it with `only`, and `refactor` still includes it.
const ALTERNATIVE_KIND: CodeActionKind = CodeActionKind::new("refactor.terraphim.alternative");

/// The code-action kind of "Stash in Overflow" and "Pull back" (R-6.3, R-6.4).
///
/// Not `refactor.move`: the words move between the body and the overflow
/// kept in the annotation block, which only this server understands. A kind
/// of its own lets a client ask for or hide them with `only`, and
/// `refactor` still includes them.
const OVERFLOW_KIND: CodeActionKind = CodeActionKind::new("refactor.terraphim.overflow");

/// The code-action kind of "Export clean Markdown" (R-9.3): a source action,
/// not a refactor, because it leaves the document alone.
const EXPORT_KIND: CodeActionKind = CodeActionKind::new("source.terraphim.export");

/// How many overflow paragraphs "Pull back" offers at once.
const PULL_BACK_LIMIT: usize = 5;

/// The status card's buttons (R-8.4).
const CARD_MAKE_CUTS: &str = "Make the cuts";
const CARD_WALK: &str = "Walk through";
const CARD_DONE: &str = "Done";

/// The status card's hint. Zed has no clickable fades, so a cut is kept
/// through its code action.
const CARD_HINT: &str = "Faded words would go. Keep one with the Keep action.";

impl TerraphimLspServer {
    /// Create a new LSP server instance tied to the given LSP client and
    /// knowledge-graph thesaurus.
    ///
    /// If the thesaurus cannot be compiled into a matcher, the error is logged
    /// and the server runs without KG matches rather than failing to start.
    ///
    /// A `thesaurus` setting from the client replaces this thesaurus.
    pub fn new(client: Client, thesaurus: Thesaurus) -> Self {
        Self::build(client, thesaurus, LaunchOptions::default())
    }

    /// Create a server that starts with an empty thesaurus and loads the
    /// launch thesaurus (`--thesaurus` / `TERRAPHIM_THESAURUS`, resolved
    /// into `launch`) at `initialize`, unless the client's `thesaurus`
    /// setting names another file.
    pub fn with_launch_options(client: Client, launch: LaunchOptions) -> Self {
        Self::build(client, Thesaurus::new("empty".to_string()), launch)
    }

    fn build(client: Client, thesaurus: Thesaurus, launch: LaunchOptions) -> Self {
        let engine = KgEngine::new(&thesaurus).unwrap_or_else(|error| {
            log::error!("terraphim_lsp: KG engine unavailable: {error}");
            KgEngine::empty()
        });
        log_skipped_patterns(&engine);
        let base_kg = Arc::new(Kg {
            thesaurus,
            engine,
            path: None,
        });
        Self {
            client,
            kg: std::sync::RwLock::new(Arc::clone(&base_kg)),
            base_kg,
            launch,
            initial_thesaurus: std::sync::RwLock::new(None),
            kg_loads: AtomicU64::new(0),
            cut_operations: Arc::new(AtomicU64::new(0)),
            documents: Arc::new(RwLock::new(HashMap::new())),
            exported: RwLock::new(HashMap::new()),
            versioned_edits: Arc::new(AtomicBool::new(false)),
            inlay_hint_refresh: AtomicBool::new(false),
            prompts: Arc::new(AtomicBool::new(false)),
            show_document: Arc::new(AtomicBool::new(false)),
            apply_edit: Arc::new(AtomicBool::new(false)),
            create_file: AtomicBool::new(false),
            changed: Arc::new(tokio::sync::Notify::new()),
            pull_diagnostics: AtomicBool::new(false),
            diagnostic_refresh: AtomicBool::new(false),
            refresh_hint: None,
            settings: std::sync::RwLock::new(ServerSettings::default()),
            lab_config: LabConfig::with_defaults()
                .inspect_err(|error| {
                    log::error!("terraphim_lsp: Lab engine unavailable: {error}");
                })
                .ok()
                .map(Arc::new),
        }
    }

    /// Convenience constructor used by `LspService::new` when no custom
    /// thesaurus is available. It creates an empty thesaurus so the server
    /// can still start; real deployments should use [`Self::new`].
    pub fn new_with_empty_thesaurus(client: Client) -> Self {
        Self::new(client, Thesaurus::new("empty".to_string()))
    }

    /// Record whether the raw `initialize` advertised
    /// `workspace/diagnostic/refresh` support under the spec key (see
    /// [`crate::handshake`]). The stdio entry points set it; a server built
    /// directly relies on the typed (legacy) key alone.
    #[must_use]
    pub fn with_refresh_hint(mut self, hint: InitializeHint) -> Self {
        self.refresh_hint = hint.diagnostic_refresh;
        self
    }

    /// Run the LSP server over stdio using an empty thesaurus, until the
    /// client sends `exit` or closes stdin; the process exit code (see
    /// [`Self::serve_stdio`]).
    ///
    /// This is the entry point for the `terraphim-lsp` binary. For programmatic
    /// use with a custom thesaurus, construct the server via [`LspService::new`]
    /// and [`Self::new`].
    pub async fn run_stdio() -> i32 {
        Self::serve_stdio(Self::new_with_empty_thesaurus).await
    }

    /// Run the LSP server over stdio with the given thesaurus; the process
    /// exit code.
    pub async fn run_stdio_with_thesaurus(thesaurus: Thesaurus) -> i32 {
        Self::serve_stdio(move |client| Self::new(client, thesaurus)).await
    }

    /// Run the LSP server over stdio with launch options (see
    /// [`Self::with_launch_options`]). The `terraphim-lsp` binary uses this;
    /// it exits the process with the code returned.
    pub async fn run_stdio_with_launch_options(launch: LaunchOptions) -> i32 {
        Self::serve_stdio(move |client| Self::with_launch_options(client, launch)).await
    }

    /// Serve stdio, first reading the raw `initialize` for the capabilities
    /// the typed params lose ([`crate::handshake`]) and replaying it in
    /// front of the rest of stdin.
    ///
    /// Returns when the client sends `exit` or closes stdin. tower-lsp
    /// notices an exit only when the next message arrives, and the blocked
    /// read of stdin then keeps the process alive, so the `exit` notification
    /// is watched for here ([`ExitWatch`]). The code follows LSP 3.17: 0 after
    /// a `shutdown` request (and at end of input), 1 for an `exit` without
    /// one. The caller exits the process with it.
    async fn serve_stdio(build: impl FnOnce(Client) -> Self) -> i32 {
        let mut stdin = tokio::io::stdin();
        let (hint, buffered) = handshake::peek_initialize(&mut stdin).await;
        let input = tokio::io::AsyncReadExt::chain(std::io::Cursor::new(buffered), stdin);
        let (service, socket) =
            LspService::new(move |client| build(client).with_refresh_hint(hint));
        let exited = Arc::new(tokio::sync::Notify::new());
        let shutdown_requested = Arc::new(AtomicBool::new(false));
        let exit_seen = Arc::new(AtomicBool::new(false));
        let service = ExitWatch {
            inner: service,
            exited: Arc::clone(&exited),
            shutdown_requested: Arc::clone(&shutdown_requested),
            exit_seen: Arc::clone(&exit_seen),
        };
        // 1 for an `exit` without a `shutdown` before it, whether the client
        // then keeps the pipe open or closes it; 0 otherwise.
        let code = || {
            i32::from(
                exit_seen.load(Ordering::SeqCst) && !shutdown_requested.load(Ordering::SeqCst),
            )
        };
        let serve = Server::new(input, tokio::io::stdout(), socket).serve(service);
        tokio::pin!(serve);
        tokio::select! {
            () = &mut serve => code(),
            () = exited.notified() => {
                // Let responses already queued (the answer to `shutdown`
                // just before `exit`) reach the client.
                let _ = tokio::time::timeout(EXIT_GRACE, &mut serve).await;
                code()
            }
        }
    }

    /// The knowledge graph in use.
    fn kg(&self) -> Arc<Kg> {
        Arc::clone(&self.kg.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// The path of the thesaurus file in use, if one was loaded from a file
    /// (successfully or not).
    pub fn thesaurus_path(&self) -> Option<PathBuf> {
        self.kg().path.clone()
    }

    /// The number of thesaurus terms in use (0 when none is loaded).
    pub fn thesaurus_len(&self) -> usize {
        self.kg().thesaurus.len()
    }

    /// Make the thesaurus match `settings`: load the effective path
    /// ([`thesaurus_setting`] over the `initializationOptions` value, then
    /// [`effective_thesaurus_path`]) if it differs from the one in use, or
    /// return to the constructor's thesaurus when no path is configured.
    /// Returns whether the thesaurus was replaced.
    ///
    /// A missing or invalid file is logged and shown once to the user
    /// (`window/showMessage`, Warning); the server keeps running with an
    /// empty engine for that path. The same path is not retried until it
    /// changes.
    async fn apply_thesaurus_setting(&self, settings: &ServerSettings) -> bool {
        let initial = self
            .initial_thesaurus
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let wanted = effective_thesaurus_path(
            thesaurus_setting(settings.thesaurus.as_deref(), initial.as_deref()),
            self.launch.thesaurus.as_deref(),
            self.launch.home.as_deref(),
        );
        if self.kg().path == wanted {
            return false;
        }
        let load = self.kg_loads.fetch_add(1, Ordering::SeqCst) + 1;
        let kg = match wanted {
            None => Arc::clone(&self.base_kg),
            Some(path) => Arc::new(self.load_kg(path).await),
        };
        let mut current = self.kg.write().unwrap_or_else(PoisonError::into_inner);
        if self.kg_loads.load(Ordering::SeqCst) != load {
            // A newer load started while this one ran; it decides.
            return false;
        }
        *current = kg;
        true
    }

    /// Load the thesaurus at `path` on the blocking pool; on failure, report
    /// it and return an empty knowledge graph for that path.
    async fn load_kg(&self, path: PathBuf) -> Kg {
        let task_path = path.clone();
        let result = tokio::task::spawn_blocking(move || load_thesaurus(&task_path)).await;
        match result {
            Ok(Ok(loaded)) => {
                log::info!(
                    "terraphim_lsp: loaded {} thesaurus terms from {}",
                    loaded.thesaurus.len(),
                    path.display()
                );
                log_skipped_patterns(&loaded.engine);
                Kg {
                    thesaurus: loaded.thesaurus,
                    engine: loaded.engine,
                    path: Some(path),
                }
            }
            Ok(Err(error)) => {
                self.report_load_failure(&path, &error).await;
                Kg::empty_for(path)
            }
            Err(error) => {
                self.report_load_failure(&path, &error).await;
                Kg::empty_for(path)
            }
        }
    }

    async fn report_load_failure(&self, path: &Path, error: &(dyn std::fmt::Display + Sync)) {
        log::error!(
            "terraphim_lsp: cannot load thesaurus {}: {error}",
            path.display()
        );
        self.client
            .show_message(
                MessageType::WARNING,
                format!(
                    "terraphim-lsp: cannot load thesaurus {}: {error}. Hover, synonym \
                     actions and inlay hints stay off until the `thesaurus` setting \
                     names a valid thesaurus JSON file.",
                    path.display()
                ),
            )
            .await;
    }

    /// The current settings.
    pub fn settings(&self) -> ServerSettings {
        self.settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set_settings(&self, settings: ServerSettings) {
        *self
            .settings
            .write()
            .unwrap_or_else(PoisonError::into_inner) = settings;
    }

    /// The current text of an open document.
    async fn document_text(&self, uri: &Url) -> Option<String> {
        self.documents
            .read()
            .await
            .get(uri)
            .map(|document| document.text.clone())
    }

    /// Record the latest text and version of a document. Requested Lab
    /// actions and the trim level survive; their results, computed for the
    /// old text, are dropped.
    async fn store_document(&self, uri: &Url, text: &str, version: i32) {
        let mut documents = self.documents.write().await;
        if documents
            .get(uri)
            .is_some_and(|document| version <= document.version)
        {
            log::debug!("terraphim_lsp: ignored non-increasing document version {version}");
            return;
        }
        let document = documents.entry(uri.clone()).or_default();
        document.text = text.to_string();
        document.version = version;
        document.reanchoring = None;
        document.generation += 1;
        document.lab.clear();
        let cut = document.trim_made.as_deref() == Some(text)
            || document
                .trim_cutting
                .is_some_and(|pending| pending.applied && version > pending.version);
        if cut {
            // "Make the cuts" was applied: the preview is over.
            end_preview(document);
        }
        document.trim_kept.clear();
        document.trim_plan = None;
        document.trim = None;
        document.trim_made = None;
        document.trim_walk = None;
        drop(documents);
        self.changed.notify_waiters();
    }

    /// What a Lab run needs from a document, taken under a brief lock so
    /// the (whole-document) computation runs without holding any lock.
    fn lab_job(&self, document: &OpenDocument) -> LabJob {
        let mut actions = self.settings().lab.actions;
        actions.extend(document.lab_actions.iter().copied());
        actions.sort_unstable();
        actions.dedup();
        LabJob {
            text: document.text.clone(),
            version: document.version,
            generation: document.generation,
            actions,
            trim_level: document.trim_level,
            trim_kept: document.trim_kept.clone(),
            trim_plan: document.trim_plan.clone(),
        }
    }

    /// Run a Lab job on the blocking pool. `None` when the Lab engine is
    /// unavailable or the task failed.
    async fn run_lab_job(&self, job: LabJob) -> Option<LabResults> {
        let config = Arc::clone(self.lab_config.as_ref()?);
        tokio::task::spawn_blocking(move || job.run(&config))
            .await
            .inspect_err(|error| log::error!("terraphim_lsp: Lab run failed: {error}"))
            .ok()
    }

    /// Install the results of `job` for `uri`, unless the document was
    /// edited or closed, or its requested Lab state changed, since the job
    /// was snapshotted: then they are dropped.
    async fn install_lab(&self, uri: &Url, job: &LabJob, results: LabResults) -> Installed {
        let mut documents = self.documents.write().await;
        let Some(document) = documents.get_mut(uri) else {
            return Installed::Edited(None);
        };
        if document.version != job.version {
            return Installed::Edited(Some(document.version));
        }
        if document.generation != job.generation {
            return Installed::Superseded;
        }
        document.lab = results.lab;
        match results.trim {
            Some(review) => review.install(document),
            None => {
                document.trim = None;
                document.trim_made = None;
            }
        }
        Installed::Yes(document.clone())
    }

    /// Recompute the Lab results of an open document, then publish. Results
    /// overtaken by an edit or a newer request are dropped; whatever
    /// overtook them recomputes.
    async fn refresh_lab_and_publish(&self, uri: &Url) {
        let job = {
            let documents = self.documents.read().await;
            documents.get(uri).map(|document| self.lab_job(document))
        };
        let Some(job) = job else {
            return;
        };
        let Some(results) = self.run_lab_job(job.clone()).await else {
            return;
        };
        if let Installed::Yes(document) = self.install_lab(uri, &job, results).await {
            let had_work = !job.actions.is_empty() || job.trim_level.is_some();
            if had_work || !self.pull_diagnostics.load(Ordering::Relaxed) {
                // A pulling client already pulled for the edit or open; ask
                // it again only when the Lab run could have changed the
                // results.
                self.publish(uri, &document).await;
            }
        }
    }

    /// The open document `uri`, checked against an expected version.
    async fn checked_document(&self, uri: &Url, version: Option<i32>) -> Result<OpenDocument> {
        let document = self
            .documents
            .read()
            .await
            .get(uri)
            .cloned()
            .ok_or_else(|| commands::invalid_params(format!("{uri} is not open")))?;
        match version {
            Some(version) if version != document.version => {
                Err(commands::content_modified(version, document.version))
            }
            _ => Ok(document),
        }
    }

    /// Apply `update` to the requested Lab state of the open document `uri`
    /// (if it is still at `version`), recompute its Lab results outside the
    /// lock and publish.
    ///
    /// The command result always matches the installed state:
    /// - if the document is edited while the results are computed, they are
    ///   dropped and the request fails with `ContentModified`;
    /// - if another Lab command changes the requested state of the same
    ///   version meanwhile, the run is superseded and this command
    ///   recomputes from the newer state (which includes its own change)
    ///   until its results are the ones installed.
    async fn update_lab(
        &self,
        uri: &Url,
        version: Option<i32>,
        update: impl FnOnce(&mut OpenDocument),
    ) -> Result<OpenDocument> {
        let job = self.begin_lab_update(uri, version, update).await?;
        self.finish_lab_update(uri, job).await
    }

    /// The first half of [`Self::update_lab`]: check the version, apply
    /// `update`, bump the generation and snapshot the job, under a brief
    /// lock.
    async fn begin_lab_update(
        &self,
        uri: &Url,
        version: Option<i32>,
        update: impl FnOnce(&mut OpenDocument),
    ) -> Result<LabJob> {
        if self.lab_config.is_none() {
            return Err(commands::invalid_params("the Lab engine is unavailable"));
        }
        let mut documents = self.documents.write().await;
        let document = documents
            .get_mut(uri)
            .ok_or_else(|| commands::invalid_params(format!("{uri} is not open")))?;
        if let Some(version) = version
            && version != document.version
        {
            return Err(commands::content_modified(version, document.version));
        }
        update(document);
        document.generation += 1;
        Ok(self.lab_job(document))
    }

    /// The second half of [`Self::update_lab`]: compute without a lock,
    /// install (recomputing while superseded) and publish.
    async fn finish_lab_update(&self, uri: &Url, mut job: LabJob) -> Result<OpenDocument> {
        loop {
            let results = self
                .run_lab_job(job.clone())
                .await
                .ok_or_else(|| commands::invalid_params("the Lab run failed"))?;
            match self.install_lab(uri, &job, results).await {
                Installed::Yes(document) => {
                    self.publish(uri, &document).await;
                    return Ok(document);
                }
                Installed::Edited(Some(held)) => {
                    return Err(commands::content_modified(job.version, held));
                }
                Installed::Edited(None) => {
                    return Err(commands::invalid_params(format!("{uri} is not open")));
                }
                Installed::Superseded => {
                    let documents = self.documents.read().await;
                    let document = documents
                        .get(uri)
                        .ok_or_else(|| commands::invalid_params(format!("{uri} is not open")))?;
                    if document.version != job.version {
                        return Err(commands::content_modified(job.version, document.version));
                    }
                    job = self.lab_job(document);
                }
            }
        }
    }

    /// A workspace edit applying `edits` to version `version` of `uri`.
    ///
    /// Clients that accept `documentChanges` get a versioned
    /// `TextDocumentEdit`, so a stale action is rejected instead of applied
    /// to text it was not computed for; others get plain `changes`.
    fn workspace_edit(&self, uri: &Url, version: i32, edits: Vec<TextEdit>) -> WorkspaceEdit {
        workspace_edit(
            uri,
            version,
            edits,
            self.versioned_edits.load(Ordering::Relaxed),
        )
    }

    /// "Replace with X" code actions for the KG term at `position`.
    fn replacement_actions(
        &self,
        uri: &Url,
        document: &OpenDocument,
        position: Position,
    ) -> Vec<CodeAction> {
        let text = document.text.as_str();
        let index = LineIndex::new(text);
        let offset = convert::byte_offset(&index, position);
        let Some(set) = self.kg().engine.alternatives_at(text, offset) else {
            return Vec::new();
        };
        // A plain rewrite of text inside a stored span or ghost (the term, or
        // its a/an fix-up) would leave that anchor stale: the span's own
        // "Replace with" actions and the hover cover it instead.
        let anchors = stored_anchor_ranges(text);
        set.replacements
            .iter()
            .filter(|replacement| {
                !replacement
                    .edits
                    .iter()
                    .any(|edit| anchors.iter().any(|anchor| overlaps(edit.range, *anchor)))
            })
            .map(|replacement| {
                let edits = replacement
                    .edits
                    .iter()
                    .map(|edit| convert::text_edit(&index, edit))
                    .collect();
                CodeAction {
                    title: format!("Replace with {}", replacement.text),
                    kind: Some(REPLACE_KIND),
                    edit: Some(self.workspace_edit(uri, document.version, edits)),
                    ..CodeAction::default()
                }
            })
            .collect()
    }

    /// "Replace with" each inactive alternative of the stored span at
    /// `position` (R-2.2, R-4.4): one versioned workspace edit that swaps
    /// the body text and records the new active alternative in the block,
    /// as the editor does. Nothing when the block cannot be rewritten (it is
    /// malformed, or an anchor cannot be placed).
    fn stored_alternative_actions(
        &self,
        uri: &Url,
        document: &OpenDocument,
        position: Position,
    ) -> Vec<CodeAction> {
        let text = document.text.as_str();
        let index = LineIndex::new(text);
        let offset = convert::byte_offset(&index, position);
        let Some(span) = stored_span_at(text, offset) else {
            return Vec::new();
        };
        span.alts
            .iter()
            .filter(|alt| alt.index != span.active)
            .filter_map(|alt| {
                let selected = select_alternative(text, &span.span_id, alt.index).ok()?;
                let edits = selected
                    .edits
                    .iter()
                    .map(|edit| convert::text_edit(&index, edit))
                    .collect();
                Some(CodeAction {
                    title: format!("Replace with {} ({})", alt.text, provenance(alt)),
                    kind: Some(ALTERNATIVE_KIND),
                    edit: Some(self.workspace_edit(uri, document.version, edits)),
                    ..CodeAction::default()
                })
            })
            .collect()
    }

    /// Every diagnostic for a document: unknown terms (when enabled), a
    /// malformed annotation block, ghost hints (when enabled), Lab marks and
    /// trim candidates. Pushed and pulled diagnostics both come from here, so
    /// they always agree.
    fn diagnostics_for(&self, document: &OpenDocument) -> Vec<Diagnostic> {
        let text = document.text.as_str();
        let settings = self.settings();
        let analysis = analyse_kg_document(text, &self.kg().engine);
        let mut diagnostics = if settings.unknown_terms {
            unknown_term_diagnostics(&analysis, text)
        } else {
            Vec::new()
        };
        diagnostics.extend(core_diagnostics(&analysis, text));
        if settings.ghost_diagnostics {
            diagnostics.extend(ghost_diagnostics(&analysis, text));
        }
        let trim = document
            .trim
            .as_ref()
            .map_or(&[][..], |view| &view.diagnostics);
        if !document.lab.is_empty() || !trim.is_empty() {
            let index = LineIndex::new(text);
            let lab = document.lab.iter().map(|finding| &finding.diagnostic);
            diagnostics.extend(
                lab.chain(trim)
                    .map(|diagnostic| convert::diagnostic(&index, diagnostic)),
            );
        }
        diagnostics
    }

    /// Push a document's diagnostics, unless the client pulls them: a
    /// client that pulls re-requests after its own edits and opens, and
    /// would show pushed diagnostics a second time.
    async fn push(&self, uri: &Url, document: &OpenDocument) {
        if self.pull_diagnostics.load(Ordering::Relaxed) {
            return;
        }
        let diagnostics = self.diagnostics_for(document);
        self.send_push(uri.clone(), diagnostics, Some(document.version))
            .await;
    }

    /// The only place `publishDiagnostics` is sent: nothing in pull mode.
    async fn send_push(&self, uri: Url, diagnostics: Vec<Diagnostic>, version: Option<i32>) {
        if self.pull_diagnostics.load(Ordering::Relaxed) {
            return;
        }
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    /// Push the diagnostics of the open document `uri` (push mode only).
    async fn push_open(&self, uri: &Url) {
        let document = self.documents.read().await.get(uri).cloned();
        if let Some(document) = document {
            self.push(uri, &document).await;
        }
    }

    /// A document's diagnostics changed without a client edit (a Lab run,
    /// a clear): push them, or in pull mode ask the client to pull again.
    async fn publish(&self, uri: &Url, document: &OpenDocument) {
        if self.pull_diagnostics.load(Ordering::Relaxed) {
            self.request_diagnostic_refresh();
        } else {
            self.push(uri, document).await;
        }
    }

    /// Send `workspace/diagnostic/refresh` when the client pulls and
    /// accepts it. Spawned, so no handler waits on the client's reply.
    fn request_diagnostic_refresh(&self) {
        if !(self.pull_diagnostics.load(Ordering::Relaxed)
            && self.diagnostic_refresh.load(Ordering::Relaxed))
        {
            return;
        }
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(error) = client.workspace_diagnostic_refresh().await {
                log::debug!("terraphim_lsp: diagnostic refresh failed: {error}");
            }
        });
    }

    /// "Apply fix: X" code actions for the Lab marks touching `range`.
    fn fix_actions(&self, uri: &Url, document: &OpenDocument, range: Range) -> Vec<CodeAction> {
        let text = document.text.as_str();
        let index = LineIndex::new(text);
        let wanted = convert::byte_range_of(&index, range);
        document
            .lab
            .iter()
            .filter(|finding| {
                let marked = finding.diagnostic.range.bytes();
                marked.start <= wanted.end && wanted.start <= marked.end
            })
            .filter_map(|finding| {
                let fix = finding.fix.as_ref()?;
                Some(CodeAction {
                    title: fix.title.clone(),
                    kind: Some(FIX_KIND),
                    diagnostics: Some(vec![convert::diagnostic(&index, &finding.diagnostic)]),
                    edit: Some(self.workspace_edit(
                        uri,
                        document.version,
                        vec![convert::text_edit(&index, &fix.edit)],
                    )),
                    is_preferred: Some(true),
                    ..CodeAction::default()
                })
            })
            .collect()
    }

    /// `[i/n]` inlay hints for the KG terms of `text` inside `range`.
    fn inlay_hints(&self, text: &str, range: Range) -> Vec<InlayHint> {
        let index = LineIndex::new(text);
        let wanted = convert::byte_range_of(&index, range);
        self.kg()
            .engine
            .synonym_positions(text)
            .into_iter()
            .filter(|hint| {
                let term = hint.term.range.bytes();
                term.start <= wanted.end && wanted.start <= term.end
            })
            .map(|hint| {
                let concept = hint
                    .term
                    .description
                    .clone()
                    .unwrap_or_else(|| hint.term.nterm.clone());
                InlayHint {
                    position: convert::position(&index, hint.term.range.end.byte),
                    label: InlayHintLabel::String(hint.label()),
                    kind: None,
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(format!(
                        "Synonym {} of {} for {concept}",
                        hint.index, hint.count
                    ))),
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                }
            })
            .collect()
    }

    /// [`commands::ADD_ALTERNATIVE`]: the workspace edit that rewrites the
    /// annotation block with the new alternative.
    async fn add_alternative(&self, args: AddAlternativeArgs) -> Result<Option<Value>> {
        let document = self.checked_document(&args.uri, args.version).await?;
        let text = document.text.as_str();
        let index = LineIndex::new(text);
        let bytes = convert::byte_range_of(&index, args.range);
        let range = TextRange::from_bytes(text, bytes.start, bytes.end.max(bytes.start));
        let added = add_alternative(text, range, &args.text, args.kind)
            .map_err(|error| commands::invalid_params(error.to_string()))?;
        let edit = self.workspace_edit(
            &args.uri,
            document.version,
            vec![convert::text_edit(&index, &added.edit)],
        );
        let value = serde_json::to_value(edit)
            .map_err(|error| commands::invalid_params(error.to_string()))?;
        Ok(Some(value))
    }

    /// The explicit apply variant for clients that only display command results.
    /// Never holds a document lock while awaiting the client. A successful
    /// acknowledgement must also match a change in the same open document.
    async fn add_alternative_and_apply(&self, args: AddAlternativeArgs) -> Result<Option<Value>> {
        if !self.apply_edit.load(Ordering::Relaxed) || !self.versioned_edits.load(Ordering::Relaxed)
        {
            return Err(commands::invalid_params(
                "add_and_apply requires workspace.applyEdit and versioned documentChanges",
            ));
        }
        let document = self.checked_document(&args.uri, args.version).await?;
        let index = LineIndex::new(&document.text);
        let bytes = convert::byte_range_of(&index, args.range);
        let range = TextRange::from_bytes(&document.text, bytes.start, bytes.end.max(bytes.start));
        let added = add_alternative(&document.text, range, &args.text, args.kind)
            .map_err(|error| commands::invalid_params(error.to_string()))?;
        let expected =
            terraphim_lsp_core::apply_edits(&document.text, std::slice::from_ref(&added.edit));
        let edit = self.workspace_edit(
            &args.uri,
            document.version,
            vec![convert::text_edit(&index, &added.edit)],
        );
        // The client must also reject an edit overtaken after this check.
        let current = self
            .checked_document(&args.uri, Some(document.version))
            .await?;
        if !Arc::ptr_eq(&current.incarnation, &document.incarnation) {
            return Err(alternative_apply_error(
                true,
                "document_reopened",
                "Document closed or reopened before the edit request",
            ));
        }
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client.apply_edit(edit),
        )
        .await;
        match response {
            Ok(Ok(response)) if response.applied => {}
            Ok(Ok(_)) => {
                return Err(alternative_apply_error(
                    false,
                    "rejected",
                    "Client rejected the alternative edit",
                ));
            }
            Ok(Err(_)) => {
                return Err(alternative_apply_error(
                    false,
                    "request_failed",
                    "Alternative edit request failed; inspect the document before retrying",
                ));
            }
            Err(_) => {
                return Err(alternative_apply_error(
                    false,
                    "acknowledgement_timeout",
                    "Client did not acknowledge the edit; inspect the document before retrying",
                ));
            }
        }
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let changed = self.changed.notified();
            let current = self.documents.read().await.get(&args.uri).cloned();
            match current {
                Some(current) if Arc::ptr_eq(&current.incarnation, &document.incarnation) => {
                    if current.version > document.version {
                        if current.text == expected {
                            return Ok(Some(serde_json::json!({
                                "applied": true, "uri": args.uri, "version": current.version,
                            })));
                        }
                        return Err(alternative_apply_error(
                            true,
                            "document_changed",
                            "Client acknowledged the edit, but the document changed to different text",
                        ));
                    }
                }
                _ => {
                    return Err(alternative_apply_error(
                        true,
                        "document_reopened",
                        "Document closed or reopened while applying the alternative",
                    ));
                }
            }
            if tokio::time::timeout_at(deadline, changed).await.is_err() {
                return Err(alternative_apply_error(
                    false,
                    "accepted_not_observed",
                    "Client accepted the edit but did not report the expected document change; inspect the document before retrying",
                ));
            }
        }
    }

    /// "Ghost it" and "Revive" for the selection `range` (R-5.1, R-5.2): each
    /// is a versioned workspace edit that rewrites only the annotation block.
    /// A malformed block, or one with anchors that no longer match the body,
    /// offers neither, as [`Self::add_alternative`] refuses to rewrite it.
    fn ghost_actions(&self, uri: &Url, document: &OpenDocument, range: Range) -> Vec<CodeAction> {
        let text = document.text.as_str();
        let index = LineIndex::new(text);
        let bytes = convert::byte_range_of(&index, range);
        let selection = TextRange::from_bytes(text, bytes.start, bytes.end.max(bytes.start));
        let available = ghost_availability(text, selection);
        let action = |title: &str, edited: std::result::Result<GhostEdited, GhostError>| {
            let edited = edited.ok()?;
            Some(CodeAction {
                title: title.to_string(),
                kind: Some(GHOST_KIND),
                edit: Some(self.workspace_edit(
                    uri,
                    document.version,
                    vec![convert::text_edit(&index, &edited.edit)],
                )),
                ..CodeAction::default()
            })
        };
        let ghost = available
            .can_ghost
            .then(|| action("Ghost it", ghost_range(text, selection)))
            .flatten();
        let revive = available
            .can_revive
            .then(|| action("Revive", revive_range(text, selection)))
            .flatten();
        ghost.into_iter().chain(revive).collect()
    }

    /// Re-anchor the annotation block of the saved document `uri` (R-9.2):
    /// one versioned `workspace/applyEdit` rewrites only the block, and only
    /// when a span or ghost moved or its stored context went stale. Saving
    /// again changes nothing, so the edit cannot loop. The edit leaves the
    /// buffer modified, so it is saved by the next save; clients that
    /// cannot take `workspace/applyEdit` are left alone.
    ///
    /// Waits (briefly) for the client's change to arrive, so the Lab refresh
    /// that follows runs on the text the client now has, not on text the
    /// change is about to replace.
    async fn reanchor_on_save(&self, uri: &Url) {
        // Version the edit, but do not rely on clients rejecting an old
        // version: some rebase against retained snapshots. Suppress duplicate
        // automatic rewrites of the same snapshot before sending anything.
        if !self.apply_edit.load(Ordering::Relaxed) || !self.versioned_edits.load(Ordering::Relaxed)
        {
            return;
        }
        // Snapshot, computation and reservation share one lock. No task can
        // reserve from a snapshot of a previous revision or a closed document.
        let (document, outcome, operation) = {
            let mut documents = self.documents.write().await;
            let Some(document) = documents.get_mut(uri) else {
                return;
            };
            if document.reanchoring.is_some() {
                return;
            }
            let outcome = reanchor_block(&document.text);
            let operation = self.cut_operations.fetch_add(1, Ordering::Relaxed);
            if matches!(outcome, Ok(ReanchorOutcome::Rewritten(_))) {
                document.reanchoring = Some(operation);
            }
            (document.clone(), outcome, operation)
        };
        let edit = match outcome {
            Ok(ReanchorOutcome::Rewritten(edit)) => edit,
            Ok(ReanchorOutcome::Unresolved(count)) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!(
                            "terraphim-lsp: {count} span(s) or ghost(s) of {uri} no longer \
                             match the body; the annotation block was not re-anchored"
                        ),
                    )
                    .await;
                return;
            }
            // Nothing moved, or a malformed block (already a diagnostic).
            Ok(ReanchorOutcome::Unchanged) | Err(_) => return,
        };
        let index = LineIndex::new(&document.text);
        let edit = self.workspace_edit(
            uri,
            document.version,
            vec![convert::text_edit(&index, &edit)],
        );
        let response = self.client.apply_edit(edit).await;
        // A negative acknowledgement permits retry. An old response cannot
        // clear a newer operation (including one after close/reopen).
        if !matches!(&response, Ok(response) if response.applied) {
            let mut documents = self.documents.write().await;
            if let Some(document) = documents.get_mut(uri)
                && document.reanchoring == Some(operation)
            {
                document.reanchoring = None;
            }
        }
        match response {
            Ok(response) if response.applied => {
                self.wait_for_change(uri, document.version).await;
            }
            Ok(response) => log::info!(
                "terraphim_lsp: the client did not re-anchor {uri}: {:?}",
                response.failure_reason
            ),
            Err(error) => log::debug!("terraphim_lsp: applyEdit failed: {error}"),
        }
    }

    /// Wait until the stored version of `uri` is past `version`, or two
    /// seconds. A client sends `didChange` for an edit it applied to an open
    /// document, so the wait ends as soon as that arrives; the limit only
    /// bounds a client that never reports it. Changes to other documents
    /// wake the wait but do not end it.
    async fn wait_for_change(&self, uri: &Url, version: i32) {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            // Created before the check, so a change in between is not missed.
            let changed = self.changed.notified();
            let arrived = self
                .documents
                .read()
                .await
                .get(uri)
                .is_none_or(|stored| stored.version > version);
            if arrived || tokio::time::timeout_at(deadline, changed).await.is_err() {
                return;
            }
        }
    }

    /// [`commands::EXPORT`]: the clean Markdown of a document, written beside
    /// it by the client when it can create files.
    async fn export(&self, args: ExportArgs) -> Result<Option<Value>> {
        let document = self.checked_document(&args.uri, args.version).await?;
        let markdown = export_clean(&document.text)
            .map_err(|error| commands::invalid_params(error.to_string()))?;
        let target = clean_export_uri(&args.uri);
        // Resource operations travel in `documentChanges`.
        let can_write = self.apply_edit.load(Ordering::Relaxed)
            && self.versioned_edits.load(Ordering::Relaxed)
            && self.create_file.load(Ordering::Relaxed);
        let mut written = false;
        if let Some(target) = target.as_ref().filter(|_| can_write) {
            written = self
                .write_export(&args.uri, &document, target, &markdown)
                .await?;
        }
        Ok(Some(serde_json::json!({
            "markdown": markdown,
            "uri": target,
            "written": written,
        })))
    }

    /// Share committed document synonym groups through the existing client edit path.
    async fn export_graph(&self, args: ExportArgs) -> Result<Option<Value>> {
        let document = self.checked_document(&args.uri, args.version).await?;
        let graph = export_knowledge_graph(&document.text)
            .map_err(|error| commands::invalid_params(error.to_string()))?;
        let json = serde_json::to_string_pretty(&graph)
            .map_err(|error| commands::invalid_params(error.to_string()))?;
        let target = args.uri.to_file_path().ok().and_then(|path| {
            let stem = path.file_stem()?.to_str()?;
            Url::from_file_path(path.with_file_name(format!("{stem}.knowledge-graph.json"))).ok()
        });
        let can_write = self.apply_edit.load(Ordering::Relaxed)
            && self.versioned_edits.load(Ordering::Relaxed)
            && self.create_file.load(Ordering::Relaxed);
        let mut written = false;
        if let Some(target) = target.as_ref().filter(|_| can_write) {
            written = self
                .write_export(&args.uri, &document, target, &json)
                .await?;
        }
        Ok(Some(
            serde_json::json!({"graph": graph, "uri": target, "written": written}),
        ))
    }

    /// Refuse filesystem aliases even when their URI strings are different.
    /// LSP does not report an authoritative dirty flag, so protect every other
    /// open document. The exact output URI still uses the generated/disk-text
    /// check below, allowing an unchanged output to be exported again.
    async fn export_target_is_separate(&self, source: &Url, target: &Url) -> bool {
        if source == target {
            return false;
        }
        let Ok(target_path) = target.to_file_path() else {
            return false;
        };
        let mut protected: Vec<PathBuf> = self
            .documents
            .read()
            .await
            .keys()
            .filter(|uri| *uri != target)
            .filter_map(|uri| uri.to_file_path().ok())
            .collect();
        if let Ok(path) = source.to_file_path() {
            protected.push(path);
        }
        // same-file uses Unix device/inode and Windows volume/file identity;
        // canonical paths alone cannot detect hard links. Its file opens run
        // off the async executor, and all inspection errors fail closed.
        let separate = tokio::task::spawn_blocking(move || -> std::io::Result<bool> {
            let target_handle = match same_file::Handle::from_path(&target_path) {
                Ok(handle) => handle,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // A genuinely new path is safe to create without overwrite.
                    // A dangling symlink is not a new independent output file.
                    return match std::fs::symlink_metadata(&target_path) {
                        Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Ok(true),
                        _ => Err(error),
                    };
                }
                Err(error) => return Err(error),
            };
            for path in protected {
                match same_file::Handle::from_path(path) {
                    Ok(handle) if handle == target_handle => return Ok(false),
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(true)
        })
        .await;
        match separate {
            Ok(Ok(value)) => value,
            error => {
                log::warn!("terraphim_lsp: cannot verify export target identity: {error:?}");
                false
            }
        }
    }

    /// Create (or replace) `target` with `markdown` through the client, then
    /// show it when the client can. False when the client did not apply it;
    /// ContentModified when destination inspection outlives the source snapshot.
    async fn write_export(
        &self,
        source: &Url,
        snapshot: &OpenDocument,
        target: &Url,
        markdown: &str,
    ) -> Result<bool> {
        if !self.export_target_is_separate(source, target).await {
            self.client.show_message(MessageType::WARNING,
                "terraphim-lsp: choose an independent export file; the destination may alias the source or another open document.").await;
            return Ok(false);
        }
        let open = self.documents.read().await.get(target).cloned();
        let disk = if let Ok(path) = target.to_file_path() {
            match tokio::fs::read_to_string(path).await {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    log::warn!("terraphim_lsp: cannot inspect export target: {error}");
                    return Ok(false);
                }
            }
        } else {
            return Ok(false);
        };
        if let Some(document) = open.as_ref() {
            let generated = self.exported.read().await.get(target).cloned();
            if generated.as_deref() != Some(&document.text)
                && disk.as_deref() != Some(&document.text)
            {
                self.client.show_message(MessageType::WARNING,
                    "terraphim-lsp: the export buffer has unconfirmed changes; save or close it before exporting again.").await;
                return Ok(false);
            }
        }
        let previous = open
            .as_ref()
            .map(|d| d.text.as_str())
            .or(disk.as_deref())
            .unwrap_or("");
        let end = convert::position(&LineIndex::new(previous), previous.len());
        let mut operations = Vec::new();
        if open.is_none() && disk.is_none() {
            operations.push(DocumentChangeOperation::Op(ResourceOp::Create(
                CreateFile {
                    uri: target.clone(),
                    options: Some(CreateFileOptions {
                        overwrite: Some(false),
                        ignore_if_exists: None,
                    }),
                    annotation_id: None,
                },
            )));
        }
        operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: target.clone(),
                version: open.as_ref().map(|d| d.version),
            },
            edits: vec![OneOf::Left(TextEdit {
                range: Range {
                    start: Position::default(),
                    end,
                },
                new_text: markdown.to_string(),
            })],
        }));
        let edit = WorkspaceEdit {
            document_changes: Some(DocumentChanges::Operations(operations)),
            ..WorkspaceEdit::default()
        };
        // Refresh protection after inspecting output buffers and disk contents;
        // neither a newly opened alias nor a changed link may cross that work.
        if !self.export_target_is_separate(source, target).await {
            return Ok(false);
        }
        // Filesystem checks above yield to didChange/close/open notifications.
        // Check both version and text immediately before dispatch; reopening a
        // different document at the same URI can restart its version sequence.
        if !self
            .documents
            .read()
            .await
            .get(source)
            .is_some_and(|current| {
                current.version == snapshot.version && current.text == snapshot.text
            })
        {
            return Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::ContentModified,
                message: "Source document changed or closed while preparing export".into(),
                data: None,
            });
        }
        let applied = match self.client.apply_edit(edit).await {
            Ok(response) => response.applied,
            Err(error) => {
                log::debug!("terraphim_lsp: applyEdit failed: {error}");
                false
            }
        };
        if applied {
            self.exported
                .write()
                .await
                .insert(target.clone(), markdown.to_string());
        }
        if applied && self.show_document.load(Ordering::Relaxed) {
            let shown = self
                .client
                .show_document(ShowDocumentParams {
                    uri: target.clone(),
                    external: Some(false),
                    take_focus: Some(true),
                    selection: None,
                })
                .await;
            if let Err(error) = shown {
                log::debug!("terraphim_lsp: showDocument failed: {error}");
            }
        }
        Ok(applied)
    }

    /// "Stash in Overflow" for the selection `range` and "Pull back:
    /// «excerpt»" for the first paragraphs of the overflow at the cursor
    /// (R-6.3, R-6.4): each is a versioned workspace edit that moves the text
    /// and rewrites the annotation block with it. A malformed block, anchors
    /// that no longer match the body, and (for a stash) a selection touching
    /// a span or (for a pull back) a cursor inside one offer nothing, as
    /// the edit would be refused.
    fn overflow_actions(
        &self,
        uri: &Url,
        document: &OpenDocument,
        range: Range,
    ) -> Vec<CodeAction> {
        let text = document.text.as_str();
        let index = LineIndex::new(text);
        let bytes = convert::byte_range_of(&index, range);
        let action = |title: String, edited: std::result::Result<OverflowEdited, OverflowError>| {
            let edited = edited.ok()?;
            let edits = edited
                .edits
                .iter()
                .map(|edit| convert::text_edit(&index, edit))
                .collect();
            Some(CodeAction {
                title,
                kind: Some(OVERFLOW_KIND),
                edit: Some(self.workspace_edit(uri, document.version, edits)),
                ..CodeAction::default()
            })
        };
        let stash = (bytes.start < bytes.end)
            .then(|| {
                let selection = TextRange::from_bytes(text, bytes.start, bytes.end);
                action(
                    "Stash in Overflow".to_string(),
                    stash_range(text, selection),
                )
            })
            .flatten();
        let pulls = overflow_paragraphs(text)
            .into_iter()
            .take(PULL_BACK_LIMIT)
            .filter_map(|paragraph| {
                action(
                    format!(
                        "Pull back: «{}»",
                        excerpt(
                            &paragraph
                                .text
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                        )
                    ),
                    pull_back(text, paragraph.index, bytes.start),
                )
            });
        stash.into_iter().chain(pulls).collect()
    }

    /// The command-only "Export clean Markdown" (R-9.3), running
    /// `terraphim.export`. Not offered when the export would be refused (a
    /// malformed block, or a ghost that can no longer be found), or would
    /// change nothing. The command carries no version: Zed runs a cached
    /// action after the document changed, and an export reads the document
    /// as it is when it runs (it edits nothing, so there is nothing to
    /// protect).
    fn export_action(&self, uri: &Url, document: &OpenDocument) -> Option<CodeAction> {
        // Nothing to export from a document with no ghosts, overflow or
        // block: the text would come out unchanged.
        let clean = export_clean(&document.text).ok()?;
        (clean != document.text).then_some(())?;
        let title = "Export clean Markdown".to_string();
        Some(CodeAction {
            title: title.clone(),
            kind: Some(EXPORT_KIND),
            command: Some(Command {
                title,
                command: commands::EXPORT.to_string(),
                arguments: Some(vec![serde_json::json!({"uri": uri})]),
            }),
            ..CodeAction::default()
        })
    }

    fn export_graph_action(&self, uri: &Url, document: &OpenDocument) -> Option<CodeAction> {
        let graph = export_knowledge_graph(&document.text).ok()?;
        if graph.is_empty() {
            return None;
        }
        let title = "Export document knowledge graph".to_string();
        Some(CodeAction {
            title: title.clone(),
            kind: Some(EXPORT_KIND),
            command: Some(Command {
                title,
                command: commands::EXPORT_KNOWLEDGE_GRAPH.to_string(),
                arguments: Some(vec![
                    serde_json::json!({"uri": uri, "version": document.version}),
                ]),
            }),
            ..CodeAction::default()
        })
    }

    /// [`commands::LAB_MARK`]: run one Lab action and publish its marks.
    async fn lab_mark(&self, args: LabMarkArgs) -> Result<Option<Value>> {
        let action = args.action;
        let document = self
            .update_lab(&args.uri, args.version, |document| {
                if let Err(at) = document.lab_actions.binary_search(&action) {
                    document.lab_actions.insert(at, action);
                }
            })
            .await?;
        let marks = document
            .lab
            .iter()
            .filter(|finding| finding.action == action)
            .count();
        Ok(Some(
            serde_json::json!({ "action": action, "marks": marks }),
        ))
    }

    /// [`commands::LAB_CLEAR`]: drop the requested Lab actions and every
    /// Lab mark until the configured actions next run.
    async fn lab_clear(&self, args: DocumentArgs) -> Result<Option<Value>> {
        self.clear_overlay(&args.uri, |document| {
            document.lab_actions.clear();
            document.lab.clear();
        })
        .await
    }

    /// The command-only Lab actions: one "Lab: <label>" per Lab action
    /// (running `terraphim.lab.mark`) and "Lab: Clear marks" (running
    /// `terraphim.lab.clear`).
    ///
    /// Stateless like the trim menu, because Zed caches code actions per
    /// cursor position: every action is always offered, and marking again or
    /// clearing with nothing marked is harmless. The marks reach the client
    /// through the command's usual publish (a diagnostic refresh for pulling
    /// clients). Not offered without the Lab engine, whose absence makes the
    /// commands fail.
    fn lab_code_actions(&self, uri: &Url, document: &OpenDocument) -> Vec<CodeAction> {
        if self.lab_config.is_none() {
            return Vec::new();
        }
        let command_action = |title: String, command: &str, argument: Value| CodeAction {
            title: title.clone(),
            kind: Some(LAB_KIND),
            command: Some(Command {
                title,
                command: command.to_string(),
                arguments: Some(vec![argument]),
            }),
            ..CodeAction::default()
        };
        let mut actions: Vec<CodeAction> = LabAction::ALL
            .into_iter()
            .map(|action| {
                command_action(
                    format!("Lab: {}", action.label()),
                    commands::LAB_MARK,
                    serde_json::json!({"uri": uri, "version": document.version, "action": action}),
                )
            })
            .collect();
        actions.push(command_action(
            "Lab: Clear marks".to_string(),
            commands::LAB_CLEAR,
            serde_json::json!({"uri": uri}),
        ));
        actions
    }

    /// The command-only trim actions at `position`: the five levels ("Trim:
    /// Original" ends the preview), "Make the cuts", "Walk through" and,
    /// with the cursor inside a faded cut, "Keep: «excerpt»".
    ///
    /// The menu is stateless on purpose: Zed caches code actions per cursor
    /// position and does not re-request them after `workspace/executeCommand`,
    /// so every action except Keep is always offered and resolves the trim
    /// state when it runs, not when the menu was built. That is why "Make the
    /// cuts" is a command, not an edit: a Keep does not change the document
    /// version, so an edit cached before it would delete the kept words.
    /// Re-previewing the active level, and making the cuts or walking with
    /// nothing faded, are harmless no-ops. None carries an `edit`, so the
    /// client runs the command through `workspace/executeCommand`. Not
    /// offered without the Lab engine, whose absence makes the commands fail.
    fn trim_actions(
        &self,
        uri: &Url,
        document: &OpenDocument,
        position: Position,
    ) -> Vec<CodeAction> {
        if self.lab_config.is_none() {
            return Vec::new();
        }
        let version = document.version;
        let command_action = |title: String, command: &str, argument: Value| CodeAction {
            title: title.clone(),
            kind: Some(TRIM_KIND),
            command: Some(Command {
                title,
                command: command.to_string(),
                arguments: Some(vec![argument]),
            }),
            ..CodeAction::default()
        };
        let mut actions: Vec<CodeAction> = TrimLevel::ALL
            .into_iter()
            .map(|level| {
                command_action(
                    level_title(level),
                    commands::TRIM_PREVIEW,
                    serde_json::json!({"uri": uri, "level": level, "version": version}),
                )
            })
            .collect();
        actions.push(command_action(
            format!("Trim: {CARD_MAKE_CUTS}"),
            commands::TRIM_MAKE_CUTS,
            serde_json::json!({"uri": uri, "version": version}),
        ));
        actions.push(command_action(
            format!("Trim: {CARD_WALK}"),
            commands::TRIM_NEXT,
            serde_json::json!({"uri": uri, "version": version, "position": position}),
        ));
        if let Some(view) = &document.trim {
            let index = LineIndex::new(&document.text);
            let offset = convert::byte_offset(&index, position);
            let mut under: Vec<_> = view
                .cuts
                .iter()
                .filter(|cut| cut.range.start.byte <= offset && offset < cut.range.end.byte)
                .collect();
            // Innermost first: keeping a filler inside a faded sentence keeps
            // just the filler; keeping the sentence keeps both.
            under.sort_by_key(|cut| cut.range.end.byte - cut.range.start.byte);
            let mut offered = Vec::new();
            for cut in under {
                if offered.contains(&cut.id) {
                    continue;
                }
                offered.push(cut.id);
                actions.push(command_action(
                    format!(
                        "Keep: \u{ab}{}\u{bb}",
                        excerpt(&document.text[cut.range.bytes()])
                    ),
                    commands::TRIM_KEEP,
                    serde_json::json!({"uri": uri, "cut": cut.id, "version": version}),
                ));
            }
        }
        actions
    }

    /// The JSON result of the trim commands that change the preview.
    fn trim_result(document: &OpenDocument) -> Value {
        let level = document.trim_level.unwrap_or(TrimLevel::Original);
        match &document.trim {
            Some(view) => serde_json::json!({
                "level": level,
                "candidates": view.spans.len(),
                "status": view.status.card_text(),
                "words_before": view.status.words_before,
                "words_after": view.status.words_after,
                "percent": view.status.percent,
            }),
            None => serde_json::json!({
                "level": level,
                "candidates": 0,
                "status": Value::Null,
            }),
        }
    }

    /// [`commands::TRIM_PREVIEW`]: fade the spans a trim level would cut,
    /// then show the status card (without waiting for it).
    async fn trim_preview(&self, args: TrimPreviewArgs) -> Result<Option<Value>> {
        let level = args.level;
        let document = self
            .update_lab(&args.uri, args.version, |document| {
                document.trim_level = (level != TrimLevel::Original).then_some(level);
                document.trim_walk = None;
                document.trim_cutting = None;
                if level == TrimLevel::Original {
                    document.trim_kept.clear();
                }
            })
            .await?;
        if document.trim.is_some() {
            self.show_card(&args.uri);
        }
        Ok(Some(Self::trim_result(&document)))
    }

    /// [`commands::TRIM_CLEAR`]: stop the trim preview.
    async fn trim_clear(&self, args: DocumentArgs) -> Result<Option<Value>> {
        self.clear_overlay(&args.uri, end_preview).await
    }

    /// [`commands::TRIM_KEEP`]: un-fade one cut, recompute the card numbers
    /// from the plan already computed for this text, publish and re-send
    /// the card.
    async fn trim_keep(&self, args: TrimKeepArgs) -> Result<Option<Value>> {
        let document = {
            let mut documents = self.documents.write().await;
            let document = documents
                .get_mut(&args.uri)
                .ok_or_else(|| commands::invalid_params(format!("{} is not open", args.uri)))?;
            if let Some(version) = args.version
                && version != document.version
            {
                return Err(commands::content_modified(version, document.version));
            }
            let (Some(level), Some(plan)) = (document.trim_level, document.trim_plan.clone())
            else {
                return Err(commands::invalid_params(
                    "no trim preview to keep a cut from",
                ));
            };
            if !plan.faded(level).any(|cut| cut.id == args.cut) {
                return Err(commands::invalid_params(format!(
                    "cut {} is not faded at {}",
                    args.cut.0,
                    level.label()
                )));
            }
            if !document.trim_kept.contains(&args.cut) {
                document.trim_kept.push(args.cut);
            }
            TrimReview::compute(&document.text, plan, level, &document.trim_kept).install(document);
            // A Lab run snapshotted before the keep must not install.
            document.generation += 1;
            document.clone()
        };
        self.publish(&args.uri, &document).await;
        self.show_card(&args.uri);
        Ok(Some(Self::trim_result(&document)))
    }

    /// [`commands::TRIM_MAKE_CUTS`]: delete every still-faded span, as the
    /// trim state is now (not as a cached menu saw it).
    async fn trim_make_cuts(&self, args: TrimMakeCutsArgs) -> Result<Option<Value>> {
        let document = self.checked_document(&args.uri, args.version).await?;
        let Some((edit, words_after)) = self.card().cuts_edit(&args.uri, &document) else {
            return Ok(Some(serde_json::json!({"edits": 0})));
        };
        let edits = edit_count(&edit);
        if self.apply_edit.load(Ordering::Relaxed) {
            let card = self.card();
            let (uri, version, generation) =
                (args.uri.clone(), document.version, document.generation);
            tokio::spawn(async move { card.apply_cuts(&uri, version, generation).await });
            return Ok(Some(
                serde_json::json!({"edits": edits, "words_after": words_after}),
            ));
        }
        Ok(Some(serde_json::json!({
            "edits": edits,
            "words_after": words_after,
            "edit": edit,
        })))
    }

    /// [`commands::TRIM_NEXT`]: select the next faded span after
    /// `position`, wrapping to the first.
    async fn trim_next(&self, args: TrimNextArgs) -> Result<Option<Value>> {
        let range = {
            let mut documents = self.documents.write().await;
            let document = documents
                .get_mut(&args.uri)
                .ok_or_else(|| commands::invalid_params(format!("{} is not open", args.uri)))?;
            if let Some(version) = args.version
                && version != document.version
            {
                return Err(commands::content_modified(version, document.version));
            }
            let index = LineIndex::new(&document.text);
            let offset = convert::byte_offset(&index, args.position);
            let Some(span) = walk_step(document, Some(offset)) else {
                return Ok(None);
            };
            document.trim_walk = Some(span.start.byte);
            convert::range(&index, span)
        };
        if self.show_document.load(Ordering::Relaxed) {
            let client = self.client.clone();
            let uri = args.uri.clone();
            tokio::spawn(async move { select(&client, uri, range).await });
        }
        Ok(Some(serde_json::json!({ "range": range })))
    }

    /// What the status card needs, detached from `self` so the card can run
    /// in its own task.
    fn card(&self) -> Card {
        Card {
            client: self.client.clone(),
            documents: Arc::clone(&self.documents),
            prompts: self.prompts.load(Ordering::Relaxed),
            show_document: self.show_document.load(Ordering::Relaxed),
            apply_edit: self.apply_edit.load(Ordering::Relaxed),
            versioned_edits: self.versioned_edits.load(Ordering::Relaxed),
            cut_operations: Arc::clone(&self.cut_operations),
        }
    }

    /// Show the status card for `uri` in a task of its own: no handler ever
    /// waits for the user's answer.
    fn show_card(&self, uri: &Url) {
        let card = self.card();
        let uri = uri.clone();
        tokio::spawn(async move { card.run(uri).await });
    }

    async fn clear_overlay(
        &self,
        uri: &Url,
        clear: impl FnOnce(&mut OpenDocument),
    ) -> Result<Option<Value>> {
        let cleared = {
            let mut documents = self.documents.write().await;
            let document = documents
                .get_mut(uri)
                .ok_or_else(|| commands::invalid_params(format!("{uri} is not open")))?;
            clear(document);
            document.generation += 1;
            document.clone()
        };
        self.publish(uri, &cleared).await;
        Ok(None)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TerraphimLspServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let versioned_edits = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.workspace_edit.as_ref())
            .and_then(|workspace_edit| workspace_edit.document_changes)
            .unwrap_or(false);
        let inlay_hint_refresh = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.inlay_hint.as_ref())
            .and_then(|inlay_hint| inlay_hint.refresh_support)
            .unwrap_or(false);
        let pull_diagnostics = params
            .capabilities
            .text_document
            .as_ref()
            .is_some_and(|text_document| text_document.diagnostic.is_some());
        // The spec key is `workspace.diagnostics`; lsp-types only models the
        // singular `workspace.diagnostic`, so the typed params carry the
        // legacy key and `refresh_hint` the raw one. Absent means no
        // refresh, as does an explicit `false`.
        let diagnostic_refresh = self
            .refresh_hint
            .or(params
                .capabilities
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.diagnostic.as_ref())
                .and_then(|diagnostic| diagnostic.refresh_support))
            .unwrap_or(false);
        self.pull_diagnostics
            .store(pull_diagnostics, Ordering::Relaxed);
        self.diagnostic_refresh
            .store(diagnostic_refresh, Ordering::Relaxed);
        self.versioned_edits
            .store(versioned_edits, Ordering::Relaxed);
        self.inlay_hint_refresh
            .store(inlay_hint_refresh, Ordering::Relaxed);
        let window = params.capabilities.window.as_ref();
        self.prompts.store(
            window
                .and_then(|window| window.show_message.as_ref())
                .is_some_and(|show| show.message_action_item.is_some()),
            Ordering::Relaxed,
        );
        self.show_document.store(
            window
                .and_then(|window| window.show_document.as_ref())
                .is_some_and(|show| show.support),
            Ordering::Relaxed,
        );
        self.apply_edit.store(
            params
                .capabilities
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.apply_edit)
                .unwrap_or(false),
            Ordering::Relaxed,
        );
        self.create_file.store(
            params
                .capabilities
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.workspace_edit.as_ref())
                .and_then(|edit| edit.resource_operations.as_ref())
                .is_some_and(|operations| operations.contains(&ResourceOperationKind::Create)),
            Ordering::Relaxed,
        );
        let settings = ServerSettings::from_value(params.initialization_options.as_ref());
        self.set_settings(settings.clone());
        settings.thesaurus.clone_into(
            &mut self
                .initial_thesaurus
                .write()
                .unwrap_or_else(PoisonError::into_inner),
        );
        self.apply_thesaurus_setting(&settings).await;
        if self.kg().thesaurus.is_empty() && self.kg().path.is_none() {
            let message = "terraphim-lsp: no thesaurus configured; set the `thesaurus` \
                           initialization option, --thesaurus or TERRAPHIM_THESAURUS to \
                           enable hover, synonym actions and inlay hints";
            log::warn!("{message}");
            self.client.log_message(MessageType::WARNING, message).await;
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        // Configured Lab actions re-run on save.
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            REPLACE_KIND,
                            FIX_KIND,
                            TRIM_KIND,
                            GHOST_KIND,
                            LAB_KIND,
                            ALTERNATIVE_KIND,
                            OVERFLOW_KIND,
                            EXPORT_KIND,
                        ]),
                        resolve_provider: Some(false),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: None,
                    resolve_provider: Some(false),
                    ..CompletionOptions::default()
                }),
                // One diagnostics model per client: pull (advertised here,
                // never pushed) when the client supports it, else push only.
                diagnostic_provider: pull_diagnostics.then(|| {
                    DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("terraphim-lsp".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    })
                }),
                // Always advertised so hints can be switched on later through
                // `workspace/didChangeConfiguration`; the server returns none
                // while the `inlayHints` setting is off (the default).
                inlay_hint_provider: Some(OneOf::Left(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: commands::ALL.iter().map(|c| c.to_string()).collect(),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "terraphim-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("terraphim_lsp initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Apply new settings to every open document:
    ///
    /// - Lab findings of actions that are neither configured any more nor
    ///   requested by command are removed at once. Command-requested actions
    ///   and trim previews stay until their `*.clear` command: the settings
    ///   only describe the automatic actions.
    /// - With `lab.trigger` `save`, configured actions that have not been
    ///   computed automatically (newly added ones, or all of them when the
    ///   trigger switches from `command` to `save`) are computed now,
    ///   through the same generation-checked path as a save; with `command`
    ///   they wait for the next Lab command. Switching `save` to `command`
    ///   keeps the installed marks: they are still valid for the current
    ///   text, and the next edit drops them as usual.
    /// - A changed `thesaurus` path is loaded first, on the blocking pool;
    ///   a missing or invalid file is reported once.
    /// - Every document is republished (pushed, or one
    ///   `workspace/diagnostic/refresh` in pull mode), so ghost hints and
    ///   unknown-term warnings follow their settings and diagnostics follow
    ///   the thesaurus, and inlay hints are refreshed.
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let old = self.settings();
        let new = ServerSettings::from_value(Some(&params.settings));
        self.set_settings(new.clone());
        self.apply_thesaurus_setting(&new).await;
        let recompute = lab_recompute_needed(&old.lab, &new.lab);
        let uris: Vec<Url> = {
            let mut documents = self.documents.write().await;
            for document in documents.values_mut() {
                // Runs in flight with the old settings must not install.
                document.generation += 1;
                let requested = &document.lab_actions;
                document.lab.retain(|finding| {
                    new.lab.actions.contains(&finding.action) || requested.contains(&finding.action)
                });
            }
            documents.keys().cloned().collect()
        };
        for uri in uris {
            if recompute {
                self.refresh_lab_and_publish(&uri).await;
            } else {
                self.push_open(&uri).await;
            }
        }
        self.request_diagnostic_refresh();
        if self.inlay_hint_refresh.load(Ordering::Relaxed)
            && let Err(error) = self.client.inlay_hint_refresh().await
        {
            log::debug!("terraphim_lsp: inlay hint refresh failed: {error}");
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        // Opening starts a new client version sequence, including version 0.
        self.documents.write().await.remove(&uri);
        self.store_document(&uri, &text, params.text_document.version)
            .await;
        if self.settings().lab.trigger == LabTrigger::Save {
            self.refresh_lab_and_publish(&uri).await;
        } else {
            self.push_open(&uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if self.settings().reanchor_on_save {
            self.reanchor_on_save(&uri).await;
        }
        if self.settings().lab.trigger == LabTrigger::Save {
            self.refresh_lab_and_publish(&uri).await;
        }
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let Some(text) = self.document_text(&params.text_document.uri).await else {
            return Ok(None);
        };
        let fold = split_annotation_block(&text)
            .block
            .and_then(|block| block.fold_lines(&text));
        Ok(fold.map(|(start_line, end_line)| {
            vec![FoldingRange {
                start_line,
                start_character: None,
                end_line,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None,
            }]
        }))
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.store_document(&uri, &change.text, version).await;
            self.push_open(&uri).await;
        }
    }

    /// Forget the document. A push client gets an empty set, clearing what
    /// was pushed. A pull client gets nothing: in the pull model the client
    /// owns the results it pulled and stops pulling (and drops them) for a
    /// closed document, so neither a push nor a refresh is needed.
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        self.send_push(params.text_document.uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(document) = self.documents.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let text = document.text;
        let index = LineIndex::new(&text);
        let offset = convert::byte_offset(&index, position);

        // A faded trim span: the card and why the span would go.
        if let Some((view, span)) = document.trim.as_ref().and_then(|view| {
            view.spans
                .iter()
                .find(|span| span.range.start.byte <= offset && offset < span.range.end.byte)
                .map(|span| (view, span))
        }) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "**{}** {}\n\n{}\n\n{CARD_HINT}",
                        view.level.label(),
                        view.status.card_text(),
                        span.reason
                    ),
                }),
                range: Some(convert::range(&index, span.range)),
            }));
        }

        let analysis = analyse_kg_document(&text, &self.kg().engine);
        let matched = analysis
            .matched_terms
            .iter()
            .find(|matched| matched.range.touches_byte(offset));
        let stored = stored_span_at(&text, offset);
        let range = match (matched, &stored) {
            (Some(matched), _) => matched.range,
            (None, Some(span)) => span.range,
            (None, None) => return Ok(None),
        };

        // The term and its description, then the stored alternatives with
        // their provenance (R-4.4), then the KG synonyms not already stored.
        let mut sections = Vec::new();
        if let Some(matched) = matched {
            sections.push(match &matched.description {
                Some(desc) => format!("**{}**\n\n{}", matched.term, desc),
                None => format!("**{}**", matched.term),
            });
        }
        if let Some(span) = &stored {
            sections.push(stored_section(span));
            if let Some(group) = terraphim_lsp_core::stored_synonym_group_at(&text, offset) {
                let current = &text[span.range.bytes()];
                let membership = group
                    .synonyms_of(current)
                    .iter()
                    .map(|term| format!("- {term} → concept 1"))
                    .collect::<Vec<_>>()
                    .join("\n");
                sections.push(format!("**Document KG synonyms** (span `{}`; local concept 1)\n\n```markdown\n{}```\n\n{}", group.span_id, group.markdown, membership));
            }
        }
        if let Some(matched) = matched {
            let synonyms: Vec<String> = self
                .kg()
                .engine
                .alternatives_at(&text, offset)
                .map(|set| set.replacements.into_iter().map(|r| r.text).collect())
                .unwrap_or_default();
            if let Some(section) =
                kg_section(&text[matched.range.bytes()], &synonyms, stored.as_ref())
            {
                sections.push(section);
            }
        }
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: sections.join("\n\n"),
            }),
            range: Some(convert::range(&index, range)),
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let wants = |kind: &CodeActionKind| {
            params
                .context
                .only
                .as_ref()
                .is_none_or(|only| only.iter().any(|requested| kind_includes(requested, kind)))
        };
        let (fixes, replacements, trims) =
            (wants(&FIX_KIND), wants(&REPLACE_KIND), wants(&TRIM_KIND));
        let (ghosts, labs) = (wants(&GHOST_KIND), wants(&LAB_KIND));
        let stored = wants(&ALTERNATIVE_KIND);
        let (overflow, export) = (wants(&OVERFLOW_KIND), wants(&EXPORT_KIND));
        if !fixes && !replacements && !trims && !ghosts && !labs && !stored && !overflow && !export
        {
            return Ok(None);
        }
        let uri = params.text_document.uri;
        let Some(document) = self.documents.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let mut actions = Vec::new();
        if fixes {
            actions.extend(self.fix_actions(&uri, &document, params.range));
        }
        if replacements {
            actions.extend(self.replacement_actions(&uri, &document, params.range.start));
        }
        if stored {
            actions.extend(self.stored_alternative_actions(&uri, &document, params.range.start));
        }
        if trims {
            actions.extend(self.trim_actions(&uri, &document, params.range.start));
        }
        if ghosts {
            actions.extend(self.ghost_actions(&uri, &document, params.range));
        }
        if overflow {
            actions.extend(self.overflow_actions(&uri, &document, params.range));
        }
        if labs {
            actions.extend(self.lab_code_actions(&uri, &document));
        }
        if export {
            actions.extend(self.export_action(&uri, &document));
            actions.extend(self.export_graph_action(&uri, &document));
        }
        let actions: Vec<CodeActionOrCommand> = actions
            .into_iter()
            .map(CodeActionOrCommand::CodeAction)
            .collect();
        Ok((!actions.is_empty()).then_some(actions))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        if !self.settings().inlay_hints {
            return Ok(None);
        }
        let Some(text) = self.document_text(&params.text_document.uri).await else {
            return Ok(None);
        };
        let hints = self.inlay_hints(&text, params.range);
        Ok((!hints.is_empty()).then_some(hints))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        match params.command.as_str() {
            commands::ADD_ALTERNATIVE => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.add_alternative(args).await
            }
            commands::ADD_ALTERNATIVE_AND_APPLY => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.add_alternative_and_apply(args).await
            }
            commands::LAB_MARK => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.lab_mark(args).await
            }
            commands::LAB_CLEAR => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.lab_clear(args).await
            }
            commands::TRIM_PREVIEW => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.trim_preview(args).await
            }
            commands::TRIM_CLEAR => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.trim_clear(args).await
            }
            commands::TRIM_KEEP => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.trim_keep(args).await
            }
            commands::TRIM_MAKE_CUTS => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.trim_make_cuts(args).await
            }
            commands::TRIM_NEXT => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.trim_next(args).await
            }
            commands::EXPORT => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.export(args).await
            }
            commands::EXPORT_KNOWLEDGE_GRAPH => {
                let args = commands::single_argument(&params.command, params.arguments)?;
                self.export_graph(args).await
            }
            other => Err(commands::invalid_params(format!(
                "unknown command {other:?}"
            ))),
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };

        let word = word_at_position(&text, position);
        let items = build_completions(&self.kg().thesaurus, &word);

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let document = self
            .documents
            .read()
            .await
            .get(&params.text_document.uri)
            .cloned();
        let items = document
            .map(|document| self.diagnostics_for(&document))
            .unwrap_or_default();

        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
                related_documents: None,
            }),
        ))
    }
}

/// Report an unsuccessful or unconfirmed explicit client edit.
fn alternative_apply_error(
    modified: bool,
    status: &str,
    message: &str,
) -> tower_lsp::jsonrpc::Error {
    tower_lsp::jsonrpc::Error {
        code: if modified {
            tower_lsp::jsonrpc::ErrorCode::ContentModified
        } else {
            tower_lsp::jsonrpc::ErrorCode::ServerError(-32803)
        },
        message: message.to_owned().into(),
        data: Some(serde_json::json!({"status": status})),
    }
}

/// Log the thesaurus keys an engine could not compile (too short to match).
fn log_skipped_patterns(engine: &KgEngine) {
    if !engine.skipped_patterns().is_empty() {
        log::warn!(
            "terraphim_lsp: {} thesaurus keys too short to match: {:?}",
            engine.skipped_patterns().len(),
            engine.skipped_patterns()
        );
    }
}

/// Whether a settings change from `old` to `new` must recompute Lab results
/// now: only under the `save` trigger, and only if some configured action
/// has not been computed automatically, because it was just added or
/// because the trigger was `command` (configured actions were never run
/// by themselves). Removing actions never needs a run (their marks are
/// filtered out), and switching to `command` keeps what is installed.
fn lab_recompute_needed(old: &LabSettings, new: &LabSettings) -> bool {
    if new.trigger != LabTrigger::Save || new.actions.is_empty() {
        return false;
    }
    old.trigger != LabTrigger::Save
        || new
            .actions
            .iter()
            .any(|action| !old.actions.contains(action))
}

/// The status card (R-8.4) and the actions it drives, in a task of its own.
#[derive(Debug, Clone)]
struct Card {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, OpenDocument>>>,
    prompts: bool,
    show_document: bool,
    apply_edit: bool,
    versioned_edits: bool,
    cut_operations: Arc<AtomicU64>,
}

impl Card {
    /// Show the card for `uri` and act on the answer until the user is done:
    /// "Make the cuts" applies the edit, "Walk through" selects the next
    /// faded span and shows the card again, "Done" or dismissing it leaves
    /// the fades. Every answer is checked against the version the card was
    /// shown for. A client without prompts gets the card as a message.
    async fn run(self, uri: Url) {
        loop {
            let Some((version, message)) = self.message(&uri).await else {
                return;
            };
            if !self.prompts {
                self.client.show_message(MessageType::INFO, message).await;
                return;
            }
            let buttons = [CARD_MAKE_CUTS, CARD_WALK, CARD_DONE]
                .map(|title| MessageActionItem {
                    title: title.to_string(),
                    properties: HashMap::new(),
                })
                .to_vec();
            let answer = self
                .client
                .show_message_request(MessageType::INFO, message, Some(buttons))
                .await;
            let choice = match answer {
                Ok(Some(item)) => item.title,
                Ok(None) => return,
                Err(error) => {
                    log::debug!("terraphim_lsp: trim card failed: {error}");
                    return;
                }
            };
            match choice.as_str() {
                CARD_MAKE_CUTS => {
                    self.make_cuts(&uri, version).await;
                    return;
                }
                CARD_WALK => {
                    if !self.walk(&uri, version).await {
                        return;
                    }
                }
                CARD_DONE => {
                    self.persist_fades(&uri, version).await;
                    return;
                }
                _ => return,
            }
        }
    }

    /// Promote the current review into durable ghosts through one versioned
    /// block edit. A rejected/stale edit leaves the preview available.
    async fn persist_fades(&self, uri: &Url, version: i32) {
        let Some(document) = self.current(uri, version).await else {
            return;
        };
        if !self.apply_edit || !self.versioned_edits {
            self.client.show_message(MessageType::WARNING,
                "terraphim-lsp: this editor cannot persist Trim Done with versioned edits; the preview remains temporary.").await;
            return;
        }
        let Some(view) = document.trim.as_ref() else {
            return;
        };
        let promoted = match trim_done(&document.text, view) {
            Ok(promoted) => promoted,
            Err(error) => {
                self.client
                    .show_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        let expected =
            terraphim_lsp_core::apply_edits(&document.text, std::slice::from_ref(&promoted.edit));
        let operation = self.cut_operations.fetch_add(1, Ordering::Relaxed);
        {
            let mut documents = self.documents.write().await;
            let Some(stored) = documents.get_mut(uri) else {
                return;
            };
            if stored.version != version
                || stored.generation != document.generation
                || stored.trim_cutting.is_some()
            {
                return;
            }
            stored.trim_cutting = Some(PendingCut {
                operation,
                version,
                applied: false,
            });
        }
        let edit = workspace_edit(
            uri,
            version,
            vec![convert::text_edit(
                &LineIndex::new(&document.text),
                &promoted.edit,
            )],
            true,
        );
        let applied = self
            .client
            .apply_edit(edit)
            .await
            .is_ok_and(|response| response.applied);
        let mut documents = self.documents.write().await;
        if let Some(stored) = documents.get_mut(uri)
            && stored
                .trim_cutting
                .is_some_and(|pending| pending.operation == operation)
        {
            if !applied {
                stored.trim_cutting = None;
            } else if stored.text == expected {
                end_preview(stored);
                stored.generation += 1;
            } else if stored.version == version {
                // Acknowledgement first: only this exact promoted text ends
                // the preview, not an unrelated subsequent user edit.
                stored.trim_made = Some(Arc::from(expected));
                stored.trim_cutting = None;
            } else {
                stored.trim_cutting = None;
            }
        }
    }

    /// The card text for the current preview of `uri` and its version;
    /// `None` when nothing is previewed.
    async fn message(&self, uri: &Url) -> Option<(i32, String)> {
        let documents = self.documents.read().await;
        let document = documents.get(uri)?;
        let view = document.trim.as_ref()?;
        Some((
            document.version,
            format!(
                "{}: {}. {CARD_HINT}",
                view.level.label(),
                view.status.card_text()
            ),
        ))
    }

    /// The document if it is still at `version` with a preview; otherwise
    /// warn that the card is stale.
    async fn current(&self, uri: &Url, version: i32) -> Option<OpenDocument> {
        let document = self.documents.read().await.get(uri).cloned();
        match document {
            Some(document) if document.version == version && document.trim.is_some() => {
                Some(document)
            }
            _ => {
                self.client
                    .show_message(
                        MessageType::WARNING,
                        "terraphim-lsp: the document changed since the trim card was shown; \
                         preview the trim again.",
                    )
                    .await;
                None
            }
        }
    }

    /// The edit "Make the cuts" makes on `document` now, and the words left;
    /// `None` without a preview.
    fn cuts_edit(&self, uri: &Url, document: &OpenDocument) -> Option<(WorkspaceEdit, usize)> {
        let level = document.trim_level?;
        let plan = document.trim_plan.as_ref()?;
        let view = document.trim.as_ref()?;
        let made = trim_cuts(&document.text, plan, level, &document.trim_kept);
        let index = LineIndex::new(&document.text);
        let edits = made
            .edits
            .iter()
            .map(|edit| convert::text_edit(&index, edit))
            .collect();
        Some((
            workspace_edit(uri, document.version, edits, self.versioned_edits),
            view.status.words_after,
        ))
    }

    async fn make_cuts(&self, uri: &Url, version: i32) {
        let Some(document) = self.current(uri, version).await else {
            return;
        };
        if self.apply_edit {
            // The edit is built from the live state at sending time: a Keep
            // made since the card was shown is honoured.
            self.apply_cuts(uri, version, document.generation).await;
        } else {
            self.client
                .show_message(
                    MessageType::WARNING,
                    "terraphim-lsp: this editor cannot take server edits; use the \
                     \"Trim: Make the cuts\" code action.",
                )
                .await;
        }
    }

    /// Build "Make the cuts" for `uri` and send it with
    /// `workspace/applyEdit`, if the document is still at `version` and
    /// `generation` (no edit, Keep, level change or Lab run since the caller
    /// looked). The edit is built and the pending cut recorded under one
    /// lock, so what is sent is exactly the state that was checked. The
    /// preview ends as described at [`PendingCut`].
    async fn apply_cuts(&self, uri: &Url, version: i32, generation: u64) {
        let operation = self.cut_operations.fetch_add(1, Ordering::Relaxed);
        let edit = {
            let mut documents = self.documents.write().await;
            if documents
                .get(uri)
                .is_some_and(|document| document.trim_cutting.is_some())
            {
                // One cut at a time: a second request's answer must not
                // clear or overwrite the first one's pending state.
                drop(documents);
                self.client
                    .show_message(
                        MessageType::INFO,
                        "terraphim-lsp: the cuts were already sent to the editor.",
                    )
                    .await;
                return;
            }
            let built = documents.get_mut(uri).and_then(|document| {
                if document.version != version || document.generation != generation {
                    return None;
                }
                let (edit, _) = self.cuts_edit(uri, document)?;
                document.trim_cutting = Some(PendingCut {
                    operation,
                    version,
                    applied: false,
                });
                Some(edit)
            });
            match built {
                Some(edit) => edit,
                None => {
                    drop(documents);
                    self.client
                        .show_message(
                            MessageType::WARNING,
                            "terraphim-lsp: the trim preview changed before the cuts were \
                             made; nothing was cut. Choose \"Make the cuts\" again.",
                        )
                        .await;
                    return;
                }
            }
        };
        let applied = match self.client.apply_edit(edit).await {
            Ok(response) => {
                if !response.applied {
                    log::info!(
                        "terraphim_lsp: the client did not apply the cuts: {:?}",
                        response.failure_reason
                    );
                }
                response.applied
            }
            Err(error) => {
                log::debug!("terraphim_lsp: applyEdit failed: {error}");
                false
            }
        };
        let mut documents = self.documents.write().await;
        let Some(document) = documents.get_mut(uri) else {
            return;
        };
        if document.trim_cutting.map(|pending| pending.operation) != Some(operation) {
            return;
        }
        if !applied {
            document.trim_cutting = None;
        } else if document.version > version {
            // The edit's change arrived first: the preview is over.
            end_preview(document);
            document.generation += 1;
        } else if let Some(pending) = document.trim_cutting.as_mut() {
            // The change follows; it ends the preview.
            pending.applied = true;
        }
    }

    /// Select the faded span after the one the card last showed (wrapping)
    /// and remember it. False when the card is stale or nothing is faded.
    async fn walk(&self, uri: &Url, version: i32) -> bool {
        let Some(document) = self.current(uri, version).await else {
            return false;
        };
        let Some(span) = walk_step(&document, None) else {
            return false;
        };
        {
            let mut documents = self.documents.write().await;
            match documents.get_mut(uri) {
                Some(stored) if stored.version == version => {
                    stored.trim_walk = Some(span.start.byte);
                }
                _ => return false,
            }
        }
        let range = convert::range(&LineIndex::new(&document.text), span);
        if self.show_document {
            select(&self.client, uri.clone(), range).await;
        }
        true
    }
}

/// How long output already queued may drain once `exit` has arrived.
const EXIT_GRACE: std::time::Duration = std::time::Duration::from_millis(250);

/// A service that tells when the `exit` notification has been handled.
///
/// tower-lsp stops serving after `exit`, but only notices when the next
/// message arrives (or at end of input), so a client that keeps stdin open
/// would wait for a server that never leaves.
struct ExitWatch<S> {
    inner: S,
    exited: Arc<tokio::sync::Notify>,
    /// Set as soon as a `shutdown` request arrives, so the exit code does not
    /// depend on whether its handler or the `exit` finishes first.
    shutdown_requested: Arc<AtomicBool>,
    /// Set as soon as an `exit` notification arrives, before it is handled,
    /// so the end of input that may follow it cannot turn the code into 0.
    exit_seen: Arc<AtomicBool>,
}

impl<S> tower::Service<tower_lsp::jsonrpc::Request> for ExitWatch<S>
where
    S: tower::Service<tower_lsp::jsonrpc::Request, Response = Option<tower_lsp::jsonrpc::Response>>,
    S::Future: Send + 'static,
    S::Error: 'static,
{
    type Response = Option<tower_lsp::jsonrpc::Response>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<Self::Response, S::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: tower_lsp::jsonrpc::Request) -> Self::Future {
        if request.method() == "shutdown" && request.id().is_some() {
            self.shutdown_requested.store(true, Ordering::SeqCst);
        }
        // `exit` is a notification: no id.
        let is_exit = request.method() == "exit" && request.id().is_none();
        if is_exit {
            self.exit_seen.store(true, Ordering::SeqCst);
        }
        let handled = self.inner.call(request);
        let exited = Arc::clone(&self.exited);
        Box::pin(async move {
            let response = handled.await;
            if is_exit {
                exited.notify_one();
            }
            response
        })
    }
}

/// Where the clean export of `uri` goes: `<stem>.clean.md` beside it. `None`
/// for anything but a `file:` URI.
fn clean_export_uri(uri: &Url) -> Option<Url> {
    let path = uri.to_file_path().ok()?;
    let stem = path.file_stem()?.to_str()?;
    Url::from_file_path(path.with_file_name(format!("{stem}.clean.md"))).ok()
}

/// Select `range` of `uri` with `window/showDocument`.
async fn select(client: &Client, uri: Url, range: Range) {
    let shown = client
        .show_document(ShowDocumentParams {
            uri,
            external: Some(false),
            take_focus: Some(true),
            selection: Some(range),
        })
        .await;
    if let Err(error) = shown {
        log::debug!("terraphim_lsp: showDocument failed: {error}");
    }
}

/// The next faded span of a walk through `document`'s preview, wrapping.
///
/// The walk continues from the span it last showed while `offset` (the
/// cursor of the "Walk through" code action) lies within that span, ends
/// included: after `window/showDocument` the cursor sits at one end of the
/// selection, and a span can start exactly where the previous one ends.
/// From anywhere else, the first span starting at or after `offset`. The
/// card's button walks without a cursor (`None`).
fn walk_step(document: &OpenDocument, offset: Option<usize>) -> Option<TextRange> {
    let spans = &document.trim.as_ref()?.spans;
    let last = document
        .trim_walk
        .and_then(|start| spans.iter().position(|span| span.range.start.byte == start));
    let next = match (last, offset) {
        (Some(last), None) => last + 1,
        (Some(last), Some(offset))
            if spans[last].range.start.byte <= offset && offset <= spans[last].range.end.byte =>
        {
            last + 1
        }
        (None, None) => match document.trim_walk {
            // The last span went (a Keep): carry on after where it was.
            Some(start) => spans
                .iter()
                .position(|span| span.range.start.byte > start)
                .unwrap_or(0),
            None => 0,
        },
        (_, Some(offset)) => spans
            .iter()
            .position(|span| span.range.start.byte >= offset)
            .unwrap_or(0),
    };
    spans.get(next).or(spans.first()).map(|span| span.range)
}

/// End `document`'s trim preview: no level, no keeps, nothing faded.
fn end_preview(document: &mut OpenDocument) {
    document.trim_level = None;
    document.trim_kept.clear();
    document.trim = None;
    document.trim_made = None;
    document.trim_walk = None;
    document.trim_cutting = None;
}

/// The number of text edits in a workspace edit built by [`workspace_edit`].
fn edit_count(edit: &WorkspaceEdit) -> usize {
    match (&edit.document_changes, &edit.changes) {
        (Some(DocumentChanges::Edits(documents)), _) => {
            documents.iter().map(|document| document.edits.len()).sum()
        }
        (_, Some(changes)) => changes.values().map(Vec::len).sum(),
        _ => 0,
    }
}

/// A workspace edit applying `edits` to version `version` of `uri`:
/// a versioned `TextDocumentEdit` when the client accepts
/// `documentChanges`, so a stale edit is rejected instead of applied to
/// text it was not computed for; plain `changes` otherwise.
fn workspace_edit(uri: &Url, version: i32, edits: Vec<TextEdit>, versioned: bool) -> WorkspaceEdit {
    if !versioned {
        return WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), edits)])),
            ..WorkspaceEdit::default()
        };
    }
    WorkspaceEdit {
        document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: Some(version),
            },
            edits: edits.into_iter().map(OneOf::Left).collect(),
        }])),
        ..WorkspaceEdit::default()
    }
}

/// The code-action title of a trim level: "Trim: Original", "Trim: Slight
/// trim ~10%", ...
fn level_title(level: TrimLevel) -> String {
    if level == TrimLevel::Original {
        return format!("Trim: {}", level.label());
    }
    format!(
        "Trim: {} ~{}%",
        level.label(),
        (level.target_fraction() * 100.0).round() as u32
    )
}

/// A cut's text for a "Keep" title: trimmed of spaces and the punctuation
/// that joins it to its neighbours, cut to about 40 characters on a word.
fn excerpt(text: &str) -> String {
    let text = text
        .trim()
        .trim_matches(|c: char| matches!(c, ',' | ';' | ':' | '\u{2014}' | '\u{2013}'))
        .trim();
    const MAX: usize = 40;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    let cut: String = text.chars().take(MAX).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}\u{2026}", cut.trim_end())
}

/// Whether a requested code-action kind (`params.context.only`) includes
/// `kind`: equal, or a dot-separated prefix of it (`refactor` includes
/// `refactor.rewrite`).
/// Whether an edit of `edit` changes text inside `anchor`: a replacement
/// that overlaps it, or an insertion strictly inside it.
fn overlaps(edit: TextRange, anchor: TextRange) -> bool {
    if edit.start.byte == edit.end.byte {
        anchor.start.byte < edit.start.byte && edit.start.byte < anchor.end.byte
    } else {
        edit.start.byte < anchor.end.byte && anchor.start.byte < edit.end.byte
    }
}

/// Who produced a stored alternative, as the hover and action titles show
/// it: `original`, `human`, `ai`, or `ai, <model>`.
fn provenance(alt: &StoredAlternative) -> String {
    match (alt.source, alt.model.as_deref()) {
        (Source::Original, _) => "original".to_string(),
        (Source::Human, _) => "human".to_string(),
        (Source::Ai, Some(model)) => format!("ai, {model}"),
        (Source::Ai, None) => "ai".to_string(),
    }
}

/// The hover's list of a stored span's alternatives, the active one marked.
fn stored_section(span: &StoredSpan) -> String {
    let kind = match span.kind {
        terraphim_lsp_core::SpanKind::Word => "word",
        terraphim_lsp_core::SpanKind::Sentence => "sentence",
        terraphim_lsp_core::SpanKind::Paragraph => "paragraph",
    };
    let mut lines = vec![format!("**Alternatives** ({kind} span)")];
    for alt in &span.alts {
        let active = if alt.index == span.active {
            ", active"
        } else {
            ""
        };
        lines.push(format!("- {} ({}{active})", alt.text, provenance(alt)));
    }
    lines.join("\n")
}

/// The hover's list of KG synonyms (`kg`), leaving out any already stored.
/// Outside a stored span the current form is listed first as the active
/// one. `None` when there is nothing to list.
fn kg_section(current: &str, synonyms: &[String], stored: Option<&StoredSpan>) -> Option<String> {
    let stored_texts: Vec<String> = stored
        .map(|span| {
            span.alts
                .iter()
                .map(|alt| alt.text.to_lowercase())
                .collect()
        })
        .unwrap_or_default();
    let listed: Vec<&String> = synonyms
        .iter()
        .filter(|synonym| !stored_texts.contains(&synonym.to_lowercase()))
        .collect();
    if listed.is_empty() {
        return None;
    }
    let mut lines = vec!["**KG synonyms**".to_string()];
    if stored.is_none() {
        lines.push(format!("- {current} (kg, active)"));
    }
    lines.extend(listed.iter().map(|synonym| format!("- {synonym} (kg)")));
    Some(lines.join("\n"))
}

fn kind_includes(requested: &CodeActionKind, kind: &CodeActionKind) -> bool {
    let (requested, kind) = (requested.as_str(), kind.as_str());
    kind == requested
        || kind
            .strip_prefix(requested)
            .is_some_and(|rest| rest.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAB_DOC: &str = include_str!("../tests/fixtures/lab_doc.md");

    const URI: &str = "file:///tmp/race.md";

    fn server_without_thesaurus() -> (LspService<TerraphimLspServer>, tower_lsp::ClientSocket) {
        LspService::new(TerraphimLspServer::new_with_empty_thesaurus)
    }

    fn uri() -> Url {
        Url::parse(URI).unwrap()
    }

    async fn installed(server: &TerraphimLspServer) -> OpenDocument {
        server.documents.read().await[&uri()].clone()
    }

    fn trim_spans(document: &OpenDocument) -> usize {
        document.trim.as_ref().map_or(0, |view| view.spans.len())
    }

    fn request(action: LabAction) -> impl FnOnce(&mut OpenDocument) {
        move |document: &mut OpenDocument| document.lab_actions.push(action)
    }

    /// The steps of a Lab command are driven one by one, so these tests do
    /// not depend on scheduling: an edit landing between snapshot and
    /// install always wins.
    #[tokio::test]
    async fn an_edit_during_a_lab_command_wins() {
        let (service, _socket) = server_without_thesaurus();
        let server = service.inner();
        server.store_document(&uri(), LAB_DOC, 1).await;
        let job = server
            .begin_lab_update(&uri(), Some(1), request(LabAction::HedgesAndFiller))
            .await
            .unwrap();
        let edited = LAB_DOC.replacen("I think ", "", 1);
        server.store_document(&uri(), &edited, 2).await;
        let error = server.finish_lab_update(&uri(), job).await.unwrap_err();
        assert_eq!(error.code, tower_lsp::jsonrpc::ErrorCode::ContentModified);
        let document = installed(server).await;
        assert!(document.lab.is_empty(), "stale results never installed");
        assert_eq!(document.text, edited);
        assert_eq!(document.lab_actions, [LabAction::HedgesAndFiller]);
        // The next save computes for the new version.
        server.refresh_lab_and_publish(&uri()).await;
        let hedges = installed(server).await.lab.len();
        assert_eq!(hedges, 4, "perhaps, quite, really, basically");
    }

    #[tokio::test]
    async fn results_for_a_superseded_version_are_dropped() {
        let (service, _socket) = server_without_thesaurus();
        let server = service.inner();
        server.store_document(&uri(), LAB_DOC, 1).await;
        let job = {
            let mut documents = server.documents.write().await;
            let document = documents.get_mut(&uri()).unwrap();
            document.lab_actions = vec![LabAction::HedgesAndFiller];
            server.lab_job(document)
        };
        server.store_document(&uri(), "Edited.", 2).await;
        let results = server.run_lab_job(job.clone()).await.expect("Lab engine");
        assert!(!results.lab.is_empty(), "the run itself found marks");
        assert!(matches!(
            server.install_lab(&uri(), &job, results).await,
            Installed::Edited(Some(2))
        ));
        assert!(installed(server).await.lab.is_empty());
    }

    /// Two Lab commands on the same version finishing in the wrong order:
    /// the older snapshot is rejected, never installed over the newer
    /// requested state.
    #[tokio::test]
    async fn an_older_snapshot_never_overwrites_newer_lab_state() {
        let (service, _socket) = server_without_thesaurus();
        let server = service.inner();
        server.store_document(&uri(), LAB_DOC, 1).await;
        let trim_job = server
            .begin_lab_update(&uri(), Some(1), |document| {
                document.trim_level = Some(TrimLevel::Slight);
            })
            .await
            .unwrap();
        let mark_job = server
            .begin_lab_update(&uri(), Some(1), request(LabAction::LongSentences))
            .await
            .unwrap();
        assert_eq!(trim_job.version, mark_job.version);
        assert!(trim_job.actions.is_empty(), "snapshotted before the mark");
        assert_eq!(mark_job.trim_level, Some(TrimLevel::Slight));

        // The stale trim-only snapshot finishes last: rejected outright.
        let mark_results = server.run_lab_job(mark_job.clone()).await.unwrap();
        let trim_results = server.run_lab_job(trim_job.clone()).await.unwrap();
        assert!(matches!(
            server.install_lab(&uri(), &mark_job, mark_results).await,
            Installed::Yes(_)
        ));
        assert!(matches!(
            server.install_lab(&uri(), &trim_job, trim_results).await,
            Installed::Superseded
        ));
        let document = installed(server).await;
        assert_eq!(document.lab.len(), 1, "the long-sentence mark survives");
        assert_eq!(trim_spans(&document), 5);

        // Through the command path, a superseded run recomputes from the
        // current state, so its result is what is installed.
        let finished = server.finish_lab_update(&uri(), trim_job).await.unwrap();
        assert_eq!(finished.lab_actions, [LabAction::LongSentences]);
        assert_eq!(finished.lab.len(), 1);
        assert_eq!(finished.trim_level, Some(TrimLevel::Slight));
        assert_eq!(trim_spans(&finished), 5);
    }

    /// Clearing while a preview is in flight: the preview never comes back.
    #[tokio::test]
    async fn a_clear_supersedes_a_preview_in_flight() {
        let (service, _socket) = server_without_thesaurus();
        let server = service.inner();
        server.store_document(&uri(), LAB_DOC, 1).await;
        let job = server
            .begin_lab_update(&uri(), None, |document| {
                document.trim_level = Some(TrimLevel::Half);
            })
            .await
            .unwrap();
        server
            .clear_overlay(&uri(), |document| {
                document.trim_level = None;
                document.trim = None;
            })
            .await
            .unwrap();
        let results = server.run_lab_job(job.clone()).await.unwrap();
        assert!(matches!(
            server.install_lab(&uri(), &job, results).await,
            Installed::Superseded
        ));
        assert!(installed(server).await.trim.is_none());
    }

    #[test]
    fn lab_recompute_transition_table() {
        use LabAction::{HedgesAndFiller as Hedges, LongSentences as Long};
        use LabTrigger::{Command, Save};
        let lab = |actions: &[LabAction], trigger| LabSettings {
            actions: actions.to_vec(),
            trigger,
        };
        // (old actions, old trigger, new actions, new trigger, recompute?)
        let table = [
            // Actions added.
            (&[][..], Save, &[Long][..], Save, true),
            (&[][..], Command, &[Long][..], Save, true),
            (&[][..], Save, &[Long][..], Command, false),
            (&[][..], Command, &[Long][..], Command, false),
            (&[Long][..], Save, &[Long, Hedges][..], Save, true),
            // Actions removed.
            (&[Long, Hedges][..], Save, &[Long][..], Save, false),
            (&[Long, Hedges][..], Command, &[Long][..], Save, true),
            (&[Long][..], Command, &[][..], Save, false),
            (&[Long][..], Save, &[][..], Command, false),
            (&[Long, Hedges][..], Command, &[Long][..], Command, false),
            // Actions unchanged.
            (&[Long][..], Save, &[Long][..], Save, false),
            (&[Long][..], Command, &[Long][..], Save, true),
            (&[Long][..], Save, &[Long][..], Command, false),
            (&[Long][..], Command, &[Long][..], Command, false),
            (&[][..], Command, &[][..], Save, false),
        ];
        for (old_actions, old_trigger, new_actions, new_trigger, expected) in table {
            let old = lab(old_actions, old_trigger);
            let new = lab(new_actions, new_trigger);
            assert_eq!(
                lab_recompute_needed(&old, &new),
                expected,
                "{old:?} -> {new:?}"
            );
        }
    }

    #[test]
    fn requested_kinds_include_by_prefix() {
        let rewrite = CodeActionKind::REFACTOR_REWRITE;
        assert!(kind_includes(&CodeActionKind::REFACTOR, &rewrite));
        assert!(kind_includes(&CodeActionKind::REFACTOR_REWRITE, &rewrite));
        assert!(!kind_includes(&CodeActionKind::QUICKFIX, &rewrite));
        assert!(!kind_includes(
            &CodeActionKind::new("refactor.re"),
            &rewrite
        ));
    }
}
