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
    Diagnostic as CoreDiagnostic, KgEngine, LabAction, LabConfig, LabFinding, LineIndex, TextRange,
    TrimLevel, add_alternative, lab_findings, trim_preview,
};
use terraphim_types::Thesaurus;

use crate::commands::{self, AddAlternativeArgs, DocumentArgs, LabMarkArgs, TrimPreviewArgs};
use crate::completion::{build_completions, word_at_position};
use crate::convert;
use crate::diagnostics::{core_diagnostics, ghost_diagnostics, unknown_term_diagnostics};
use crate::kg_analysis::analyse_kg_document;
use crate::settings::{LabSettings, LabTrigger, ServerSettings};
use crate::thesaurus::{LaunchOptions, effective_thesaurus_path, load_thesaurus};

/// Terraphim LSP server backed by a knowledge-graph thesaurus.
///
/// The server tracks open text documents and provides:
///
/// - `textDocument/hover` - concept descriptions for matched KG terms
/// - `textDocument/completion` - thesaurus term suggestions
/// - `textDocument/diagnostic` (and pushed `publishDiagnostics`) - a
///   malformed `terraphim-alternatives` annotation block, faded
///   (`Unnecessary`) hints over ghosted text and, when the `unknownTerms`
///   setting is on, a warning on every unknown-word occurrence
/// - `textDocument/codeAction` - "Replace with X" for every other synonym of
///   the KG term at the cursor, keeping capitalisation and fixing `a`/`an`,
///   and "Apply fix: X" for Lab typo and punctuation marks
/// - Lab marks and trim candidates as diagnostics, computed on demand
///   (`terraphim.lab.mark`, `terraphim.trim.preview`) and, for the
///   configured `lab.actions`, on open and save; never on every keystroke
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
    /// Bumped by every thesaurus load; a load installs its result only if
    /// no newer load started meanwhile.
    kg_loads: AtomicU64,
    documents: Arc<RwLock<HashMap<Url, OpenDocument>>>,
    /// Whether the client accepts `WorkspaceEdit.documentChanges`
    /// (`workspace.workspaceEdit.documentChanges`), read at `initialize`.
    versioned_edits: AtomicBool,
    /// Whether the client accepts `workspace/inlayHint/refresh`, read at
    /// `initialize`.
    inlay_hint_refresh: AtomicBool,
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
    /// Lab actions requested for this document through `terraphim.lab.mark`,
    /// sorted. Kept across edits until `terraphim.lab.clear`.
    lab_actions: Vec<LabAction>,
    /// Lab findings for the current text. Cleared on every change, because
    /// their ranges would be stale.
    lab: Vec<LabFinding>,
    /// The trim level previewed through `terraphim.trim.preview`, until
    /// `terraphim.trim.clear`.
    trim_level: Option<TrimLevel>,
    /// Trim candidates for the current text. Cleared on every change.
    trim: Vec<CoreDiagnostic>,
    /// The trim status card for the current text, `535 → 480 words · −10%`.
    trim_status: Option<String>,
    /// Bumped whenever what the Lab results depend on changes: the text,
    /// the requested actions, the trim level or the Lab settings. A Lab run
    /// installs its results only if the generation it snapshotted is still
    /// current, so runs finishing out of order never overwrite newer state.
    generation: u64,
}

/// A snapshot of one document for a Lab run.
#[derive(Debug, Clone)]
struct LabJob {
    text: String,
    version: i32,
    generation: u64,
    actions: Vec<LabAction>,
    trim_level: Option<TrimLevel>,
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
    trim: Vec<CoreDiagnostic>,
    trim_status: Option<String>,
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
        let preview = self
            .trim_level
            .map(|level| trim_preview(&self.text, config, level));
        LabResults {
            lab,
            trim_status: preview.as_ref().map(|preview| preview.status.clone()),
            trim: preview
                .map(|preview| preview.diagnostics)
                .unwrap_or_default(),
        }
    }
}

/// The code-action kind of the Lab fixes.
const FIX_KIND: CodeActionKind = CodeActionKind::QUICKFIX;

/// The code-action kind of the synonym replacements.
const REPLACE_KIND: CodeActionKind = CodeActionKind::REFACTOR_REWRITE;

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
            kg_loads: AtomicU64::new(0),
            documents: Arc::new(RwLock::new(HashMap::new())),
            versioned_edits: AtomicBool::new(false),
            inlay_hint_refresh: AtomicBool::new(false),
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

    /// Run the LSP server over stdio using an empty thesaurus.
    ///
    /// This is the entry point for the `terraphim-lsp` binary. For programmatic
    /// use with a custom thesaurus, construct the server via [`LspService::new`]
    /// and [`Self::new`].
    pub async fn run_stdio() {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) = LspService::new(Self::new_with_empty_thesaurus);
        Server::new(stdin, stdout, socket).serve(service).await;
    }

    /// Run the LSP server over stdio with the given thesaurus.
    pub async fn run_stdio_with_thesaurus(thesaurus: Thesaurus) {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) = LspService::new(move |client| Self::new(client, thesaurus.clone()));
        Server::new(stdin, stdout, socket).serve(service).await;
    }

    /// Run the LSP server over stdio with launch options (see
    /// [`Self::with_launch_options`]). The `terraphim-lsp` binary uses this.
    pub async fn run_stdio_with_launch_options(launch: LaunchOptions) {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) =
            LspService::new(move |client| Self::with_launch_options(client, launch.clone()));
        Server::new(stdin, stdout, socket).serve(service).await;
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
    /// ([`effective_thesaurus_path`]) if it differs from the one in use, or
    /// return to the constructor's thesaurus when no path is configured.
    /// Returns whether the thesaurus was replaced.
    ///
    /// A missing or invalid file is logged and shown once to the user
    /// (`window/showMessage`, Warning); the server keeps running with an
    /// empty engine for that path. The same path is not retried until it
    /// changes.
    async fn apply_thesaurus_setting(&self, settings: &ServerSettings) -> bool {
        let wanted = effective_thesaurus_path(
            settings.thesaurus.as_deref(),
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
        let document = documents.entry(uri.clone()).or_default();
        document.text = text.to_string();
        document.version = version;
        document.generation += 1;
        document.lab.clear();
        document.trim.clear();
        document.trim_status = None;
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
        document.trim = results.trim;
        document.trim_status = results.trim_status;
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
            self.publish(uri, &document).await;
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
        if !self.versioned_edits.load(Ordering::Relaxed) {
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
        set.replacements
            .iter()
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
        if !document.lab.is_empty() || !document.trim.is_empty() {
            let index = LineIndex::new(text);
            let lab = document.lab.iter().map(|finding| &finding.diagnostic);
            diagnostics.extend(
                lab.chain(&document.trim)
                    .map(|diagnostic| convert::diagnostic(&index, diagnostic)),
            );
        }
        diagnostics
    }

    /// Publish a document's diagnostics to the client.
    async fn publish(&self, uri: &Url, document: &OpenDocument) {
        let diagnostics = self.diagnostics_for(document);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(document.version))
            .await;
    }

    /// Publish the diagnostics of the open document `uri`.
    async fn publish_open(&self, uri: &Url) {
        let document = self.documents.read().await.get(uri).cloned();
        if let Some(document) = document {
            self.publish(uri, &document).await;
        }
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

    /// [`commands::TRIM_PREVIEW`]: fade the spans a trim level would cut.
    async fn trim_preview(&self, args: TrimPreviewArgs) -> Result<Option<Value>> {
        let level = args.level;
        let document = self
            .update_lab(&args.uri, args.version, |document| {
                document.trim_level = (level != TrimLevel::Original).then_some(level);
            })
            .await?;
        Ok(Some(serde_json::json!({
            "level": level,
            "candidates": document.trim.len(),
            "status": document.trim_status,
        })))
    }

    /// [`commands::TRIM_CLEAR`]: stop the trim preview.
    async fn trim_clear(&self, args: DocumentArgs) -> Result<Option<Value>> {
        self.clear_overlay(&args.uri, |document| {
            document.trim_level = None;
            document.trim.clear();
            document.trim_status = None;
        })
        .await
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
        self.versioned_edits
            .store(versioned_edits, Ordering::Relaxed);
        self.inlay_hint_refresh
            .store(inlay_hint_refresh, Ordering::Relaxed);
        let settings = ServerSettings::from_value(params.initialization_options.as_ref());
        self.set_settings(settings.clone());
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![REPLACE_KIND, FIX_KIND]),
                        resolve_provider: Some(false),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: None,
                    resolve_provider: Some(false),
                    ..CompletionOptions::default()
                }),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("terraphim-lsp".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
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
            ..InitializeResult::default()
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
    /// - A changed `thesaurus` path is loaded first (see
    ///   [`Self::apply_thesaurus_setting`]).
    /// - Every document is republished, so ghost hints and unknown-term
    ///   warnings follow their settings and diagnostics follow the
    ///   thesaurus, and inlay hints are refreshed.
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
                self.publish_open(&uri).await;
            }
        }
        if self.inlay_hint_refresh.load(Ordering::Relaxed)
            && let Err(error) = self.client.inlay_hint_refresh().await
        {
            log::debug!("terraphim_lsp: inlay hint refresh failed: {error}");
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.store_document(&uri, &text, params.text_document.version)
            .await;
        if self.settings().lab.trigger == LabTrigger::Save {
            self.refresh_lab_and_publish(&uri).await;
        } else {
            self.publish_open(&uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if self.settings().lab.trigger == LabTrigger::Save {
            self.refresh_lab_and_publish(&params.text_document.uri)
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.store_document(&uri, &change.text, version).await;
            self.publish_open(&uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };

        let analysis = analyse_kg_document(&text, &self.kg().engine);
        let index = LineIndex::new(&text);
        let offset = convert::byte_offset(&index, position);

        let hover = analysis
            .matched_terms
            .iter()
            .find(|matched| matched.range.touches_byte(offset))
            .map(|matched| {
                let contents = match &matched.description {
                    Some(desc) => format!("**{}**\n\n{}", matched.term, desc),
                    None => format!("**{}**", matched.term),
                };
                Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(convert::range(&index, matched.range)),
                }
            });
        Ok(hover)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let wants = |kind: &CodeActionKind| {
            params
                .context
                .only
                .as_ref()
                .is_none_or(|only| only.iter().any(|requested| kind_includes(requested, kind)))
        };
        let (fixes, replacements) = (wants(&FIX_KIND), wants(&REPLACE_KIND));
        if !fixes && !replacements {
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

/// Whether a requested code-action kind (`params.context.only`) includes
/// `kind`: equal, or a dot-separated prefix of it (`refactor` includes
/// `refactor.rewrite`).
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

    const LAB_DOC: &str = include_str!("../../terraphim_lsp_core/tests/fixtures/lab_doc.md");

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
        assert_eq!(document.trim.len(), 5);

        // Through the command path, a superseded run recomputes from the
        // current state, so its result is what is installed.
        let finished = server.finish_lab_update(&uri(), trim_job).await.unwrap();
        assert_eq!(finished.lab_actions, [LabAction::LongSentences]);
        assert_eq!(finished.lab.len(), 1);
        assert_eq!(finished.trim_level, Some(TrimLevel::Slight));
        assert_eq!(finished.trim.len(), 5);
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
                document.trim.clear();
            })
            .await
            .unwrap();
        let results = server.run_lab_job(job.clone()).await.unwrap();
        assert!(matches!(
            server.install_lab(&uri(), &job, results).await,
            Installed::Superseded
        ));
        assert!(installed(server).await.trim.is_empty());
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
