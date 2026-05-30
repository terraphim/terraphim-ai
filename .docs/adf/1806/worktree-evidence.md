# Worktree Isolation Evidence

Issue: 1806
Flow: adf-worktree-isolation-review
Generated: 2026-05-29 20:20 BST

## Source Inventory

```text
crates/terraphim_orchestrator/src/flow/executor.rs:15:use terraphim_spawner::{AgentSpawner, OutputEvent, SpawnContext, SpawnRequest};
crates/terraphim_orchestrator/src/flow/executor.rs:57:/// Per-project runtime metadata used to build a [`SpawnContext`] for flow
crates/terraphim_orchestrator/src/flow/executor.rs:62:    pub working_dir: PathBuf,
crates/terraphim_orchestrator/src/flow/executor.rs:68:    pub working_dir: PathBuf,
crates/terraphim_orchestrator/src/flow/executor.rs:72:    /// mean "use the FlowExecutor's top-level working_dir" (legacy mode).
crates/terraphim_orchestrator/src/flow/executor.rs:77:    pub fn new(working_dir: PathBuf, flow_state_dir: PathBuf) -> Self {
crates/terraphim_orchestrator/src/flow/executor.rs:79:            working_dir: working_dir.clone(),
crates/terraphim_orchestrator/src/flow/executor.rs:80:            spawner: AgentSpawner::new().with_working_dir(&working_dir),
crates/terraphim_orchestrator/src/flow/executor.rs:92:    /// Build a [`SpawnContext`] for the given flow's project. If the project
crates/terraphim_orchestrator/src/flow/executor.rs:93:    /// id is unknown or legacy, returns [`SpawnContext::global()`].
crates/terraphim_orchestrator/src/flow/executor.rs:94:    fn spawn_context_for_flow(&self, flow: &FlowDefinition) -> SpawnContext {
crates/terraphim_orchestrator/src/flow/executor.rs:96:            return SpawnContext::global();
crates/terraphim_orchestrator/src/flow/executor.rs:98:        let working_dir_str = runtime.working_dir.to_string_lossy().into_owned();
crates/terraphim_orchestrator/src/flow/executor.rs:99:        let mut ctx = SpawnContext::with_working_dir(runtime.working_dir.clone())
crates/terraphim_orchestrator/src/flow/executor.rs:101:            .with_env("ADF_WORKING_DIR", working_dir_str);
crates/terraphim_orchestrator/src/flow/executor.rs:136:                .current_dir(&self.working_dir)
crates/terraphim_orchestrator/src/flow/executor.rs:235:                self.working_dir.join(task_file)
crates/terraphim_orchestrator/src/flow/executor.rs:297:                working_dir: self.working_dir.clone(),
crates/terraphim_orchestrator/src/flow/executor.rs:402:                    self.working_dir.join(task_file)
crates/terraphim_spawner/src/lib.rs:23:/// pass per-project working_dir and env without constructing N spawners.
crates/terraphim_spawner/src/lib.rs:25:pub struct SpawnContext {
crates/terraphim_spawner/src/lib.rs:27:    pub working_dir: Option<PathBuf>,
crates/terraphim_spawner/src/lib.rs:32:impl SpawnContext {
crates/terraphim_spawner/src/lib.rs:33:    /// Use the spawner's default working_dir and no env overrides.
crates/terraphim_spawner/src/lib.rs:38:    /// Override working_dir; keep env untouched.
crates/terraphim_spawner/src/lib.rs:39:    pub fn with_working_dir(path: impl Into<PathBuf>) -> Self {
crates/terraphim_spawner/src/lib.rs:41:            working_dir: Some(path.into()),
crates/terraphim_spawner/src/lib.rs:397:    default_working_dir: PathBuf,
crates/terraphim_spawner/src/lib.rs:410:            default_working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:418:    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
crates/terraphim_spawner/src/lib.rs:419:        self.default_working_dir = dir.into();
crates/terraphim_spawner/src/lib.rs:457:        ctx: SpawnContext,
crates/terraphim_spawner/src/lib.rs:475:        ctx: SpawnContext,
crates/terraphim_spawner/src/lib.rs:493:        ctx: &SpawnContext,
crates/terraphim_spawner/src/lib.rs:510:        ctx: SpawnContext,
crates/terraphim_spawner/src/lib.rs:524:        ctx: SpawnContext,
crates/terraphim_spawner/src/lib.rs:599:        ctx: &SpawnContext,
crates/terraphim_spawner/src/lib.rs:655:        ctx: &SpawnContext,
crates/terraphim_spawner/src/lib.rs:657:        // Priority: ctx override > config working_dir > spawner default
crates/terraphim_spawner/src/lib.rs:658:        let working_dir = ctx
crates/terraphim_spawner/src/lib.rs:659:            .working_dir
crates/terraphim_spawner/src/lib.rs:661:            .or(config.working_dir.as_ref())
crates/terraphim_spawner/src/lib.rs:662:            .unwrap_or(&self.default_working_dir);
crates/terraphim_spawner/src/lib.rs:665:        cmd.current_dir(working_dir).args(&config.args);
crates/terraphim_spawner/src/lib.rs:841:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:855:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:865:            .with_working_dir("/workspace")
crates/terraphim_spawner/src/lib.rs:870:        assert_eq!(spawner.default_working_dir, PathBuf::from("/workspace"));
crates/terraphim_spawner/src/lib.rs:879:            .spawn(&provider, "Hello World", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:895:            .spawn(&provider, "done", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:914:            .spawn(&provider, "60", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:941:            .spawn(&provider, "hello", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:961:            .spawn(&provider, "broadcast test", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:994:            .spawn(&provider, "resource-limited", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:1005:            .spawn(&provider, "hello", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:1028:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:1042:            .spawn_with_model_stdin(&provider, "hello from stdin", None, SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:1078:            .spawn(&provider, "arg test", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:1114:            .spawn_with_model_stdin(&provider, &large_prompt, None, SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:1138:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:1149:                SpawnContext::global(),
crates/terraphim_spawner/src/lib.rs:1180:    // SpawnContext Tests (Gitea adf-fleet#3)
crates/terraphim_spawner/src/lib.rs:1185:        let ctx = SpawnContext::global();
crates/terraphim_spawner/src/lib.rs:1186:        assert!(ctx.working_dir.is_none());
crates/terraphim_spawner/src/lib.rs:1191:    fn test_spawn_context_with_working_dir() {
crates/terraphim_spawner/src/lib.rs:1192:        let ctx = SpawnContext::with_working_dir("/some/project");
crates/terraphim_spawner/src/lib.rs:1193:        assert_eq!(ctx.working_dir, Some(PathBuf::from("/some/project")));
crates/terraphim_spawner/src/lib.rs:1199:        let ctx = SpawnContext::global()
crates/terraphim_spawner/src/lib.rs:1202:        assert!(ctx.working_dir.is_none());
crates/terraphim_spawner/src/lib.rs:1208:    async fn test_spawn_global_uses_spawner_default_working_dir() {
crates/terraphim_spawner/src/lib.rs:1209:        let spawner = AgentSpawner::new().with_working_dir("/tmp");
crates/terraphim_spawner/src/lib.rs:1212:        // SpawnContext::global() should preserve spawner's default behaviour.
crates/terraphim_spawner/src/lib.rs:1215:            .spawn(&provider, "hello", SpawnContext::global())
crates/terraphim_spawner/src/lib.rs:1221:    async fn test_spawn_with_working_dir_override() {
crates/terraphim_spawner/src/lib.rs:1248:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:1253:        let spawner = AgentSpawner::new().with_working_dir("/tmp");
crates/terraphim_spawner/src/lib.rs:1254:        let ctx = SpawnContext::with_working_dir(tmppath.clone());
crates/terraphim_spawner/src/lib.rs:1259:            .expect("spawn with working_dir override should succeed");
crates/terraphim_spawner/src/lib.rs:1318:                working_dir: tmpdir.path().to_path_buf(),
crates/terraphim_spawner/src/lib.rs:1329:                SpawnContext::global()
crates/terraphim_spawner/src/lib.rs:1352:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:1358:        let ctx = SpawnContext::global().with_env("ADF_SPAWN_CTX_TEST", "hello-from-ctx");
crates/terraphim_spawner/src/lib.rs:1400:                working_dir: PathBuf::from("/tmp"),
crates/terraphim_spawner/src/lib.rs:1407:            .spawn(&provider, "ADF_INHERITED_SPAWN_CTX", SpawnContext::global())
crates/terraphim_spawner/src/config.rs:33:    pub working_dir: Option<PathBuf>,
crates/terraphim_spawner/src/config.rs:54:                working_dir,
crates/terraphim_spawner/src/config.rs:59:                working_dir: Some(working_dir.clone()),
crates/terraphim_spawner/src/config.rs:244:        self.validate_working_dir().await?;
crates/terraphim_spawner/src/config.rs:295:    async fn validate_working_dir(&self) -> Result<(), ValidationError> {
crates/terraphim_spawner/src/config.rs:296:        if let Some(dir) = &self.config.working_dir {
crates/terraphim_spawner/src/config.rs:437:                working_dir: std::env::current_dir().unwrap(),
crates/terraphim_spawner/src/config.rs:456:                working_dir: std::env::current_dir().unwrap(),
crates/terraphim_spawner/src/config.rs:539:                working_dir: std::env::current_dir().unwrap(),
crates/terraphim_orchestrator/src/compound.rs:12:use crate::scope::{WorktreeManager, WORKTREE_REVIEW_PREFIX};
crates/terraphim_orchestrator/src/compound.rs:61:    /// Root directory for worktrees.
crates/terraphim_orchestrator/src/compound.rs:62:    pub worktree_root: PathBuf,
crates/terraphim_orchestrator/src/compound.rs:108:            worktree_root: config.worktree_root.clone(),
crates/terraphim_orchestrator/src/compound.rs:122:            worktree_root: config.worktree_root.clone(),
crates/terraphim_orchestrator/src/compound.rs:234:    worktree_manager: WorktreeManager,
crates/terraphim_orchestrator/src/compound.rs:240:        let worktree_manager = WorktreeManager::with_base(&config.repo_path, &config.worktree_root);
crates/terraphim_orchestrator/src/compound.rs:243:            worktree_manager,
crates/terraphim_orchestrator/src/compound.rs:253:    /// Borrow the inner [`WorktreeManager`].
crates/terraphim_orchestrator/src/compound.rs:256:    /// `worktree_manager().sweep_stale(...)` from
crates/terraphim_orchestrator/src/compound.rs:261:    pub fn worktree_manager(&self) -> &WorktreeManager {
crates/terraphim_orchestrator/src/compound.rs:262:        &self.worktree_manager
crates/terraphim_orchestrator/src/compound.rs:308:        // Create worktree for this review.
crates/terraphim_orchestrator/src/compound.rs:320:        //   2. `guard: WorktreeGuard` drops LAST, running
crates/terraphim_orchestrator/src/compound.rs:321:        //      `git -C <repo> worktree remove --force <path>` (with
crates/terraphim_orchestrator/src/compound.rs:323:        //      `<repo>/.git/worktrees/<name>` is reconciled along
crates/terraphim_orchestrator/src/compound.rs:326:        // Inverting this order recreates the worktree storm race:
crates/terraphim_orchestrator/src/compound.rs:327:        // the guard would remove the worktree while subprocesses
crates/terraphim_orchestrator/src/compound.rs:333:        let worktree_name = format!("{}{}", WORKTREE_REVIEW_PREFIX, correlation_id);
crates/terraphim_orchestrator/src/compound.rs:335:            .worktree_manager
crates/terraphim_orchestrator/src/compound.rs:336:            .create_worktree(&worktree_name, git_ref)
crates/terraphim_orchestrator/src/compound.rs:339:                OrchestratorError::CompoundReviewFailed(format!("failed to create worktree: {}", e))
crates/terraphim_orchestrator/src/compound.rs:341:        let worktree_path = guard.path().to_path_buf();
crates/terraphim_orchestrator/src/compound.rs:347:        // synchronous `git worktree remove` runs.
crates/terraphim_orchestrator/src/compound.rs:351:            let worktree_path_task = worktree_path.clone();
crates/terraphim_orchestrator/src/compound.rs:359:                    &worktree_path_task,
crates/terraphim_orchestrator/src/compound.rs:407:        // invokes `git worktree remove --force` synchronously.
crates/terraphim_orchestrator/src/compound.rs:519:    worktree_path: &Path,
crates/terraphim_orchestrator/src/compound.rs:573:    cmd.current_dir(worktree_path);
crates/terraphim_orchestrator/src/compound.rs:1107:            worktree_root: std::env::temp_dir().join("test-compound-review-worktrees"),
crates/terraphim_orchestrator/src/compound.rs:1123:            worktree_root: std::env::temp_dir().join("test-compound-review-worktrees"),
crates/terraphim_orchestrator/src/compound.rs:1156:            worktree_root: PathBuf::from("/tmp/worktrees"),
crates/terraphim_orchestrator/src/compound.rs:1168:        assert_eq!(swarm_config.worktree_root, PathBuf::from("/tmp/worktrees"));
crates/terraphim_orchestrator/src/compound.rs:1343:            worktree_root: PathBuf::from("/tmp/worktrees"),
crates/terraphim_orchestrator/src/compound.rs:1365:            worktree_root: PathBuf::from("/tmp/worktrees"),
crates/terraphim_orchestrator/src/compound.rs:1432:            worktree_root: PathBuf::from("/tmp/worktrees"),
crates/terraphim_orchestrator/src/agent_runner.rs:136:    pub working_dir: String,
crates/terraphim_orchestrator/src/agent_runner.rs:179:    let working_dir = config.working_dir_for_agent(agent);
crates/terraphim_orchestrator/src/agent_runner.rs:180:    let repo_ok = working_dir.is_dir();
crates/terraphim_orchestrator/src/agent_runner.rs:185:            working_dir.display()
crates/terraphim_orchestrator/src/agent_runner.rs:232:        working_dir: working_dir.display().to_string(),
crates/terraphim_orchestrator/src/agent_runner.rs:375:    fn config(working_dir: &std::path::Path) -> OrchestratorConfig {
crates/terraphim_orchestrator/src/agent_runner.rs:377:            working_dir: working_dir.to_path_buf(),
crates/terraphim_orchestrator/src/agent_runner.rs:381:                repo_path: working_dir.to_path_buf(),
crates/terraphim_orchestrator/src/agent_runner.rs:449:            working_dir: project.path().to_path_buf(),
crates/terraphim_orchestrator/src/agent_runner.rs:475:        assert_eq!(report.working_dir, project.path().display().to_string());
crates/terraphim_orchestrator/src/scope.rs:7:use crate::worktree_guard::WorktreeGuard;
crates/terraphim_orchestrator/src/scope.rs:10:/// worktree.  Presence of this file with valid contents is the gate for
crates/terraphim_orchestrator/src/scope.rs:12:pub const WORKTREE_MANIFEST_FILENAME: &str = ".adf-worktree-manifest.json";
crates/terraphim_orchestrator/src/scope.rs:14:/// Ownership manifest stored at the root of each ADF worktree.
crates/terraphim_orchestrator/src/scope.rs:20:pub struct WorktreeManifest {
crates/terraphim_orchestrator/src/scope.rs:23:    /// Git repository this worktree belongs to.
crates/terraphim_orchestrator/src/scope.rs:25:    /// Absolute path of this worktree (self-referential).
crates/terraphim_orchestrator/src/scope.rs:26:    pub worktree_path: String,
crates/terraphim_orchestrator/src/scope.rs:27:    /// Name of the agent or component that created this worktree.
crates/terraphim_orchestrator/src/scope.rs:29:    /// Session or correlation ID linking this worktree to its task.
crates/terraphim_orchestrator/src/scope.rs:31:    /// Process ID that performed the `git worktree add`.
crates/terraphim_orchestrator/src/scope.rs:37:impl WorktreeManifest {
crates/terraphim_orchestrator/src/scope.rs:42:    /// Write a manifest to `dir / WORKTREE_MANIFEST_FILENAME`.
crates/terraphim_orchestrator/src/scope.rs:44:        let path = dir.join(WORKTREE_MANIFEST_FILENAME);
crates/terraphim_orchestrator/src/scope.rs:48:        debug!(path = %path.display(), "worktree manifest written");
crates/terraphim_orchestrator/src/scope.rs:57:        let path = dir.join(WORKTREE_MANIFEST_FILENAME);
crates/terraphim_orchestrator/src/scope.rs:59:        let m: WorktreeManifest = serde_json::from_slice(&bytes).ok()?;
crates/terraphim_orchestrator/src/scope.rs:65:                "worktree manifest version too new, skipping"
crates/terraphim_orchestrator/src/scope.rs:79:                "worktree manifest repo mismatch"
crates/terraphim_orchestrator/src/scope.rs:83:        if self.worktree_path != dir_on_disk.to_string_lossy() {
crates/terraphim_orchestrator/src/scope.rs:85:                manifest_path = %self.worktree_path,
crates/terraphim_orchestrator/src/scope.rs:87:                "worktree manifest path mismatch"
crates/terraphim_orchestrator/src/scope.rs:105:/// Directory-name prefix for compound-review worktrees.
crates/terraphim_orchestrator/src/scope.rs:109:/// - Layer 2 (`scope::WorktreeManager::sweep_stale`) when matching
crates/terraphim_orchestrator/src/scope.rs:115:pub const WORKTREE_REVIEW_PREFIX: &str = "review-";
crates/terraphim_orchestrator/src/scope.rs:312:/// Manages git worktrees for isolated agent workspaces.
crates/terraphim_orchestrator/src/scope.rs:314:/// Worktrees allow agents to work on different branches/refs without
crates/terraphim_orchestrator/src/scope.rs:317:pub struct WorktreeManager {
crates/terraphim_orchestrator/src/scope.rs:319:    worktree_base: PathBuf,
crates/terraphim_orchestrator/src/scope.rs:322:impl WorktreeManager {
crates/terraphim_orchestrator/src/scope.rs:323:    /// Create a new worktree manager for a git repository.
crates/terraphim_orchestrator/src/scope.rs:325:    /// Worktrees will be created under `<repo>/.worktrees/<name>`.
crates/terraphim_orchestrator/src/scope.rs:328:        let worktree_base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:332:            worktree_base,
crates/terraphim_orchestrator/src/scope.rs:336:    /// Create a worktree manager with a custom base directory.
crates/terraphim_orchestrator/src/scope.rs:338:    /// Worktrees will be created under `<worktree_base>/<name>`.
crates/terraphim_orchestrator/src/scope.rs:339:    pub fn with_base(repo_path: impl AsRef<Path>, worktree_base: impl AsRef<Path>) -> Self {
crates/terraphim_orchestrator/src/scope.rs:341:        let base = worktree_base.as_ref().to_path_buf();
crates/terraphim_orchestrator/src/scope.rs:342:        // Resolve relative worktree_base against repo_path to avoid CWD-dependent behaviour
crates/terraphim_orchestrator/src/scope.rs:350:            worktree_base: resolved_base,
crates/terraphim_orchestrator/src/scope.rs:354:    /// Get the base path where worktrees are created.
crates/terraphim_orchestrator/src/scope.rs:355:    pub fn worktree_base(&self) -> &Path {
crates/terraphim_orchestrator/src/scope.rs:356:        &self.worktree_base
crates/terraphim_orchestrator/src/scope.rs:364:    /// Create a new worktree.
crates/terraphim_orchestrator/src/scope.rs:366:    /// * `name` - Name of the worktree (used as directory name)
crates/terraphim_orchestrator/src/scope.rs:369:    /// Returns a `WorktreeGuard` that owns cleanup of the worktree.
crates/terraphim_orchestrator/src/scope.rs:371:    /// invokes `git worktree remove --force` against the repository
crates/terraphim_orchestrator/src/scope.rs:372:    /// (reconciling the `<repo>/.git/worktrees/<name>` admin entry)
crates/terraphim_orchestrator/src/scope.rs:374:    pub async fn create_worktree(
crates/terraphim_orchestrator/src/scope.rs:378:    ) -> Result<WorktreeGuard, std::io::Error> {
crates/terraphim_orchestrator/src/scope.rs:379:        let worktree_path = self.worktree_base.join(name);
crates/terraphim_orchestrator/src/scope.rs:382:        if let Some(parent) = worktree_path.parent() {
crates/terraphim_orchestrator/src/scope.rs:388:            worktree_path = %worktree_path.display(),
crates/terraphim_orchestrator/src/scope.rs:390:            "creating git worktree"
crates/terraphim_orchestrator/src/scope.rs:396:            .arg("worktree")
crates/terraphim_orchestrator/src/scope.rs:398:            .arg(&worktree_path)
crates/terraphim_orchestrator/src/scope.rs:406:            error!(name = %name, stderr = %stderr, "git worktree add failed");
crates/terraphim_orchestrator/src/scope.rs:408:                "Failed to create worktree '{}': {}",
crates/terraphim_orchestrator/src/scope.rs:413:        info!(name = %name, path = %worktree_path.display(), "worktree created");
crates/terraphim_orchestrator/src/scope.rs:416:        // worktree as ADF-managed.  Best-effort; a missing or invalid
crates/terraphim_orchestrator/src/scope.rs:417:        // manifest inhibits cleanup rather than breaking the worktree.
crates/terraphim_orchestrator/src/scope.rs:418:        let manifest = WorktreeManifest {
crates/terraphim_orchestrator/src/scope.rs:419:            version: WorktreeManifest::CURRENT_VERSION,
crates/terraphim_orchestrator/src/scope.rs:421:            worktree_path: worktree_path.to_string_lossy().to_string(),
crates/terraphim_orchestrator/src/scope.rs:427:        if let Err(e) = manifest.write_to_dir(&worktree_path) {
crates/terraphim_orchestrator/src/scope.rs:429:                path = %worktree_path.display(),
crates/terraphim_orchestrator/src/scope.rs:431:                "failed to write worktree manifest; cleanup will skip this entry"
crates/terraphim_orchestrator/src/scope.rs:435:        Ok(WorktreeGuard::for_managed(&self.repo_path, worktree_path))
crates/terraphim_orchestrator/src/scope.rs:438:    /// Remove a worktree.
crates/terraphim_orchestrator/src/scope.rs:440:    /// * `name` - Name of the worktree to remove
crates/terraphim_orchestrator/src/scope.rs:441:    pub async fn remove_worktree(&self, name: &str) -> Result<(), std::io::Error> {
crates/terraphim_orchestrator/src/scope.rs:442:        let worktree_path = self.worktree_base.join(name);
crates/terraphim_orchestrator/src/scope.rs:444:        if !worktree_path.exists() {
crates/terraphim_orchestrator/src/scope.rs:445:            warn!(name = %name, path = %worktree_path.display(), "worktree does not exist");
crates/terraphim_orchestrator/src/scope.rs:449:        info!(name = %name, "removing git worktree");
crates/terraphim_orchestrator/src/scope.rs:454:            .arg("worktree")
crates/terraphim_orchestrator/src/scope.rs:456:            .arg(&worktree_path)
crates/terraphim_orchestrator/src/scope.rs:466:                .arg("worktree")
crates/terraphim_orchestrator/src/scope.rs:469:                .arg(&worktree_path)
crates/terraphim_orchestrator/src/scope.rs:476:                error!(name = %name, stderr = %stderr, "git worktree remove failed");
crates/terraphim_orchestrator/src/scope.rs:478:                    "Failed to remove worktree '{}': {}",
crates/terraphim_orchestrator/src/scope.rs:485:        if let Some(parent) = worktree_path.parent() {
crates/terraphim_orchestrator/src/scope.rs:489:        info!(name = %name, "worktree removed");
crates/terraphim_orchestrator/src/scope.rs:493:    /// Remove all worktrees managed by this manager.
crates/terraphim_orchestrator/src/scope.rs:495:    /// Returns the number of worktrees removed.
crates/terraphim_orchestrator/src/scope.rs:497:        let worktrees = self.list_worktrees()?;
crates/terraphim_orchestrator/src/scope.rs:500:        for name in &worktrees {
crates/terraphim_orchestrator/src/scope.rs:501:            if let Err(e) = self.remove_worktree(name).await {
crates/terraphim_orchestrator/src/scope.rs:502:                error!(name = %name, error = %e, "failed to remove worktree during cleanup");
crates/terraphim_orchestrator/src/scope.rs:508:        info!(count = count, "cleaned up all worktrees");
crates/terraphim_orchestrator/src/scope.rs:512:    /// List all worktrees managed by this manager.
crates/terraphim_orchestrator/src/scope.rs:514:    /// Returns a list of worktree names (directory names, not full paths).
crates/terraphim_orchestrator/src/scope.rs:515:    pub fn list_worktrees(&self) -> Result<Vec<String>, std::io::Error> {
crates/terraphim_orchestrator/src/scope.rs:516:        if !self.worktree_base.exists() {
crates/terraphim_orchestrator/src/scope.rs:520:        let mut worktrees = Vec::new();
crates/terraphim_orchestrator/src/scope.rs:522:        for entry in std::fs::read_dir(&self.worktree_base)? {
crates/terraphim_orchestrator/src/scope.rs:527:                // Verify this is actually a git worktree by checking for .git file or directory
crates/terraphim_orchestrator/src/scope.rs:530:                        worktrees.push(name.to_string());
crates/terraphim_orchestrator/src/scope.rs:536:        Ok(worktrees)
crates/terraphim_orchestrator/src/scope.rs:539:    /// Check if a worktree exists.
crates/terraphim_orchestrator/src/scope.rs:540:    pub fn worktree_exists(&self, name: &str) -> bool {
crates/terraphim_orchestrator/src/scope.rs:541:        self.worktree_base.join(name).join(".git").exists()
crates/terraphim_orchestrator/src/scope.rs:544:    /// Sweep stale worktree residue left by a previous orchestrator
crates/terraphim_orchestrator/src/scope.rs:550:    /// is spawned. This is Layer 2 of the worktree lifecycle defence
crates/terraphim_orchestrator/src/scope.rs:553:    /// - Layer 1 (`WorktreeGuard::Drop`) handles the happy / cancelled
crates/terraphim_orchestrator/src/scope.rs:562:    /// 1. Walk `self.worktree_base` direct children whose name starts
crates/terraphim_orchestrator/src/scope.rs:563:    ///    with [`WORKTREE_REVIEW_PREFIX`] and attempt removal.
crates/terraphim_orchestrator/src/scope.rs:565:    ///    prefix filter -- the per-agent `/tmp/adf-worktrees` root
crates/terraphim_orchestrator/src/scope.rs:567:    /// 3. Removal tries `git worktree remove --force` first so the
crates/terraphim_orchestrator/src/scope.rs:568:    ///    `<repo>/.git/worktrees/<name>` admin entry is reconciled,
crates/terraphim_orchestrator/src/scope.rs:574:    /// 5. After walking, runs `git worktree prune --verbose` to drop
crates/terraphim_orchestrator/src/scope.rs:587:        if self.worktree_base.is_dir() {
crates/terraphim_orchestrator/src/scope.rs:588:            match std::fs::read_dir(&self.worktree_base) {
crates/terraphim_orchestrator/src/scope.rs:595:                        if !name.starts_with(WORKTREE_REVIEW_PREFIX) {
crates/terraphim_orchestrator/src/scope.rs:603:                        path = %self.worktree_base.display(),
crates/terraphim_orchestrator/src/scope.rs:605:                        "sweep_stale could not enumerate worktree base"
crates/terraphim_orchestrator/src/scope.rs:611:        // 2. Extra roots (typically `/tmp/adf-worktrees`): every
crates/terraphim_orchestrator/src/scope.rs:633:        // 3. Reconcile git's admin registry so half-killed worktree
crates/terraphim_orchestrator/src/scope.rs:634:        //    metadata under `<repo>/.git/worktrees/` is dropped.
crates/terraphim_orchestrator/src/scope.rs:638:            .arg("worktree")
crates/terraphim_orchestrator/src/scope.rs:647:                warn!(stderr = %stderr, "git worktree prune failed during sweep");
crates/terraphim_orchestrator/src/scope.rs:650:            warn!(error = %e, "git worktree prune could not be invoked during sweep");
crates/terraphim_orchestrator/src/scope.rs:662:            "worktree sweep_stale complete"
crates/terraphim_orchestrator/src/scope.rs:667:    /// Remove one worktree path, updating `report` in place.
crates/terraphim_orchestrator/src/scope.rs:669:    /// Before deletion, checks for a valid [`WorktreeManifest`]. If the
crates/terraphim_orchestrator/src/scope.rs:673:    /// the `review-` prefix or live under `/tmp/adf-worktrees`.
crates/terraphim_orchestrator/src/scope.rs:675:    /// Tries `git worktree remove --force` first so the git admin
crates/terraphim_orchestrator/src/scope.rs:682:        let manifest = match WorktreeManifest::read_valid(path, &self.repo_path) {
crates/terraphim_orchestrator/src/scope.rs:702:            .arg("worktree")
crates/terraphim_orchestrator/src/scope.rs:710:            // Git removed both the worktree directory and the admin
crates/terraphim_orchestrator/src/scope.rs:712:            // worktree was already corrupt; tidy that up best-effort.
crates/terraphim_orchestrator/src/scope.rs:720:        // Git refused (or the path was never a registered worktree to
crates/terraphim_orchestrator/src/scope.rs:721:        // begin with -- common for residue under `/tmp/adf-worktrees`).
crates/terraphim_orchestrator/src/scope.rs:728:                    "sweep_stale skipping root-owned worktree -- Layer 3 will clean"
crates/terraphim_orchestrator/src/scope.rs:741:                    "sweep_stale failed to remove worktree residue"
crates/terraphim_orchestrator/src/scope.rs:749:/// Summary of a [`WorktreeManager::sweep_stale`] invocation.
crates/terraphim_orchestrator/src/scope.rs:756:    /// Number of worktree directories successfully removed (either
crates/terraphim_orchestrator/src/scope.rs:757:    /// via `git worktree remove --force` or filesystem fallback).
crates/terraphim_orchestrator/src/scope.rs:766:    /// Number of entries skipped because no valid ADF worktree
crates/terraphim_orchestrator/src/scope.rs:770:    /// Whether `git worktree prune --verbose` exited zero. False
crates/terraphim_orchestrator/src/scope.rs:771:    /// implies the git admin registry under `<repo>/.git/worktrees`
crates/terraphim_orchestrator/src/scope.rs:976:    // ==================== WorktreeManager Tests ====================
crates/terraphim_orchestrator/src/scope.rs:1037:    /// Write a valid ADF worktree manifest into `dir` for testing.
crates/terraphim_orchestrator/src/scope.rs:1039:    /// as ADF-managed worktrees.
crates/terraphim_orchestrator/src/scope.rs:1041:        let manifest = WorktreeManifest {
crates/terraphim_orchestrator/src/scope.rs:1042:            version: WorktreeManifest::CURRENT_VERSION,
crates/terraphim_orchestrator/src/scope.rs:1044:            worktree_path: dir.to_string_lossy().to_string(),
crates/terraphim_orchestrator/src/scope.rs:1054:    async fn test_create_worktree() {
crates/terraphim_orchestrator/src/scope.rs:1056:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1058:        let guard_result = manager.create_worktree("feature-branch", "HEAD").await;
crates/terraphim_orchestrator/src/scope.rs:1061:            "create_worktree failed: {:?}",
crates/terraphim_orchestrator/src/scope.rs:1075:    async fn test_remove_worktree() {
crates/terraphim_orchestrator/src/scope.rs:1077:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1079:        // Create worktree; keep the guard so the manual remove path
crates/terraphim_orchestrator/src/scope.rs:1081:        let guard = manager.create_worktree("to-remove", "HEAD").await.unwrap();
crates/terraphim_orchestrator/src/scope.rs:1082:        let path = manager.worktree_base().join("to-remove");
crates/terraphim_orchestrator/src/scope.rs:1086:        // Remove worktree
crates/terraphim_orchestrator/src/scope.rs:1087:        let result = manager.remove_worktree("to-remove").await;
crates/terraphim_orchestrator/src/scope.rs:1088:        assert!(result.is_ok(), "remove_worktree failed: {:?}", result.err());
crates/terraphim_orchestrator/src/scope.rs:1093:    async fn test_remove_nonexistent_worktree() {
crates/terraphim_orchestrator/src/scope.rs:1095:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1097:        // Should succeed (no-op) for non-existent worktree
crates/terraphim_orchestrator/src/scope.rs:1098:        let result = manager.remove_worktree("nonexistent").await;
crates/terraphim_orchestrator/src/scope.rs:1105:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1107:        // Create multiple worktrees; disarm each guard so cleanup_all
crates/terraphim_orchestrator/src/scope.rs:1110:        manager.create_worktree("wt1", "HEAD").await.unwrap().keep();
crates/terraphim_orchestrator/src/scope.rs:1111:        manager.create_worktree("wt2", "HEAD").await.unwrap().keep();
crates/terraphim_orchestrator/src/scope.rs:1112:        manager.create_worktree("wt3", "HEAD").await.unwrap().keep();
crates/terraphim_orchestrator/src/scope.rs:1114:        let worktrees = manager.list_worktrees().unwrap();
crates/terraphim_orchestrator/src/scope.rs:1115:        assert_eq!(worktrees.len(), 3);
crates/terraphim_orchestrator/src/scope.rs:1121:        let worktrees = manager.list_worktrees().unwrap();
crates/terraphim_orchestrator/src/scope.rs:1122:        assert!(worktrees.is_empty());
crates/terraphim_orchestrator/src/scope.rs:1126:    async fn test_list_worktrees() {
crates/terraphim_orchestrator/src/scope.rs:1128:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1131:        let worktrees = manager.list_worktrees().unwrap();
crates/terraphim_orchestrator/src/scope.rs:1132:        assert!(worktrees.is_empty());
crates/terraphim_orchestrator/src/scope.rs:1134:        // Create worktrees; keep the guards so the worktrees survive
crates/terraphim_orchestrator/src/scope.rs:1135:        // long enough for list_worktrees to enumerate them.
crates/terraphim_orchestrator/src/scope.rs:1136:        let _g_a = manager.create_worktree("wt-a", "HEAD").await.unwrap();
crates/terraphim_orchestrator/src/scope.rs:1137:        let _g_b = manager.create_worktree("wt-b", "HEAD").await.unwrap();
crates/terraphim_orchestrator/src/scope.rs:1142:        let worktrees = manager.list_worktrees().unwrap();
crates/terraphim_orchestrator/src/scope.rs:1143:        assert_eq!(worktrees.len(), 2);
crates/terraphim_orchestrator/src/scope.rs:1144:        assert!(worktrees.contains(&"wt-a".to_string()));
crates/terraphim_orchestrator/src/scope.rs:1145:        assert!(worktrees.contains(&"wt-b".to_string()));
crates/terraphim_orchestrator/src/scope.rs:1149:    async fn test_worktree_exists() {
crates/terraphim_orchestrator/src/scope.rs:1151:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1153:        assert!(!manager.worktree_exists("test-wt"));
crates/terraphim_orchestrator/src/scope.rs:1155:        let guard = manager.create_worktree("test-wt", "HEAD").await.unwrap();
crates/terraphim_orchestrator/src/scope.rs:1156:        assert!(manager.worktree_exists("test-wt"));
crates/terraphim_orchestrator/src/scope.rs:1159:        manager.remove_worktree("test-wt").await.unwrap();
crates/terraphim_orchestrator/src/scope.rs:1160:        assert!(!manager.worktree_exists("test-wt"));
crates/terraphim_orchestrator/src/scope.rs:1164:    fn test_worktree_paths() {
crates/terraphim_orchestrator/src/scope.rs:1166:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1169:        assert_eq!(manager.worktree_base(), repo_path.join(".worktrees"));
crates/terraphim_orchestrator/src/scope.rs:1173:    async fn test_create_duplicate_worktree_fails() {
crates/terraphim_orchestrator/src/scope.rs:1175:        let manager = WorktreeManager::new(&repo_path);
crates/terraphim_orchestrator/src/scope.rs:1177:        let guard = manager.create_worktree("duplicate", "HEAD").await.unwrap();
crates/terraphim_orchestrator/src/scope.rs:1180:        let result = manager.create_worktree("duplicate", "HEAD").await;
crates/terraphim_orchestrator/src/scope.rs:1192:        let base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:1194:        let manager = WorktreeManager::with_base(&repo_path, &base);
crates/terraphim_orchestrator/src/scope.rs:1208:            WorktreeManager::with_base(&repo_path, repo_path.join("does-not-exist-anywhere"));
crates/terraphim_orchestrator/src/scope.rs:1220:        // worktrees -- sweep_one falls back to remove_dir_all when git
crates/terraphim_orchestrator/src/scope.rs:1225:        let base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:1229:            let dir = base.join(format!("{}{}", WORKTREE_REVIEW_PREFIX, i));
crates/terraphim_orchestrator/src/scope.rs:1235:        let manager = WorktreeManager::with_base(&repo_path, &base);
crates/terraphim_orchestrator/src/scope.rs:1244:            let dir = base.join(format!("{}{}", WORKTREE_REVIEW_PREFIX, i));
crates/terraphim_orchestrator/src/scope.rs:1252:        // swept from worktree_base.  `keep-me` lacks BOTH a review
crates/terraphim_orchestrator/src/scope.rs:1255:        let base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:1258:        let review_dir = base.join(format!("{}victim", WORKTREE_REVIEW_PREFIX));
crates/terraphim_orchestrator/src/scope.rs:1265:        let manager = WorktreeManager::with_base(&repo_path, &base);
crates/terraphim_orchestrator/src/scope.rs:1283:        let base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:1285:        let manager = WorktreeManager::with_base(&repo_path, &base);
crates/terraphim_orchestrator/src/scope.rs:1300:        let base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:1303:        // Unique temp dir to mimic `/tmp/adf-worktrees` without
crates/terraphim_orchestrator/src/scope.rs:1305:        let extra = std::env::temp_dir().join(format!("adf-worktrees-test-{}", Uuid::new_v4()));
crates/terraphim_orchestrator/src/scope.rs:1312:        let manager = WorktreeManager::with_base(&repo_path, &base);
crates/terraphim_orchestrator/src/scope.rs:1347:        let base = repo_path.join(".worktrees");
crates/terraphim_orchestrator/src/scope.rs:1350:        let review_dir = base.join(format!("{}root-owned", WORKTREE_REVIEW_PREFIX));
crates/terraphim_orchestrator/src/scope.rs:1363:        let report = WorktreeManager::with_base(&repo_path, &base).sweep_stale(&[]);
crates/terraphim_orchestrator/src/agent_run_command.rs:145:            working_dir: config.working_dir_for_agent(agent).display().to_string(),
crates/terraphim_orchestrator/src/agent_run_command.rs:146:            repo_ok: config.working_dir_for_agent(agent).is_dir(),
crates/terraphim_orchestrator/src/agent_run_command.rs:452:        working_dir: ".".to_string(),
crates/terraphim_orchestrator/src/agent_run_command.rs:478:            println!("Working Dir: {}", report.working_dir);
crates/terraphim_orchestrator/src/agent_run_command.rs:733:            working_dir: tmp.path().to_path_buf(),
crates/terraphim_orchestrator/src/meta_coordinator.rs:49:    pub working_dir: std::path::PathBuf,
crates/terraphim_orchestrator/src/meta_coordinator.rs:61:            .field("working_dir", &self.working_dir)
crates/terraphim_orchestrator/src/meta_coordinator.rs:147:                working_dir: project.working_dir.clone(),
crates/terraphim_orchestrator/src/pr_dispatch.rs:4://! pre-check, persona composition, worktrees). Everything in this module is
crates/terraphim_orchestrator/src/pr_dispatch.rs:15:use terraphim_spawner::SpawnContext;
crates/terraphim_orchestrator/src/pr_dispatch.rs:76:/// [`SpawnContext`] without clobbering existing keys the orchestrator already
crates/terraphim_orchestrator/src/pr_dispatch.rs:78:pub fn layer_pr_env(mut base: SpawnContext, req: &ReviewPrRequest) -> SpawnContext {
crates/terraphim_orchestrator/src/pr_dispatch.rs:102:working_dir = "/tmp/pr-dispatch-tests"
crates/terraphim_orchestrator/src/pr_dispatch.rs:112:working_dir = "/tmp/alpha"
crates/terraphim_orchestrator/src/pr_dispatch.rs:116:working_dir = "/tmp/beta"
crates/terraphim_orchestrator/src/pr_dispatch.rs:171:        let base = SpawnContext::default()
crates/terraphim_orchestrator/src/lib.rs:78:pub mod worktree_guard;
crates/terraphim_orchestrator/src/lib.rs:132:pub use worktree_guard::{with_worktree_guard, with_worktree_guard_async, WorktreeGuard};
crates/terraphim_orchestrator/src/lib.rs:145:use terraphim_spawner::{AgentHandle, AgentSpawner, ResourceLimits, SpawnContext, SpawnRequest};
crates/terraphim_orchestrator/src/lib.rs:182:    worktree_path: Option<PathBuf>,
crates/terraphim_orchestrator/src/lib.rs:196:    /// Worktree guard for automatic cleanup on agent crash.
crates/terraphim_orchestrator/src/lib.rs:198:    worktree_guard: Option<crate::worktree_guard::WorktreeGuard>,
crates/terraphim_orchestrator/src/lib.rs:264:    /// very next tick, producing a worktree storm (#1562).
crates/terraphim_orchestrator/src/lib.rs:612:                    working_dir: p.working_dir.clone(),
crates/terraphim_orchestrator/src/lib.rs:621:/// Build a [`SpawnContext`] for an agent, resolving per-project working_dir,
crates/terraphim_orchestrator/src/lib.rs:623:/// environment (`ADF_PROJECT_ID`, `ADF_WORKING_DIR`, `GITEA_OWNER`,
crates/terraphim_orchestrator/src/lib.rs:624:/// `GITEA_REPO`). Legacy (project-less) agents use [`SpawnContext::global()`].
crates/terraphim_orchestrator/src/lib.rs:635:) -> SpawnContext {
crates/terraphim_orchestrator/src/lib.rs:637:        return SpawnContext::global();
crates/terraphim_orchestrator/src/lib.rs:640:        return SpawnContext::global();
crates/terraphim_orchestrator/src/lib.rs:642:    let working_dir_str = project.working_dir.to_string_lossy().into_owned();
crates/terraphim_orchestrator/src/lib.rs:643:    let mut ctx = SpawnContext::with_working_dir(project.working_dir.clone())
crates/terraphim_orchestrator/src/lib.rs:645:        .with_env("ADF_WORKING_DIR", working_dir_str);
crates/terraphim_orchestrator/src/lib.rs:748:        // Set CARGO_TARGET_DIR so worktree agents share the main build cache,
crates/terraphim_orchestrator/src/lib.rs:749:        // and RUSTC_WRAPPER=sccache for cross-worktree compilation caching.
crates/terraphim_orchestrator/src/lib.rs:751:        let target_dir = config.working_dir.join("target");
crates/terraphim_orchestrator/src/lib.rs:764:            info!("sccache detected, enabling shared compilation cache for worktrees");
crates/terraphim_orchestrator/src/lib.rs:767:            .with_working_dir(&config.working_dir)
crates/terraphim_orchestrator/src/lib.rs:777:        // Reconcile any worktree residue left by a previous instance
crates/terraphim_orchestrator/src/lib.rs:782:        // `extra_roots` mirrors the per-agent worktree convention
crates/terraphim_orchestrator/src/lib.rs:789:        // `git worktree prune --verbose` races against
crates/terraphim_orchestrator/src/lib.rs:791:        // `git worktree add` on that shared real repo's
crates/terraphim_orchestrator/src/lib.rs:792:        // `.git/worktrees/` admin registry. The production wiring is
crates/terraphim_orchestrator/src/lib.rs:800:            let sweep_report = compound_workflow.worktree_manager().sweep_stale(&[]);
crates/terraphim_orchestrator/src/lib.rs:806:                    "large worktree backlog at startup -- prior crash storm likely"
crates/terraphim_orchestrator/src/lib.rs:812:        let handoff_ledger = HandoffLedger::new(config.working_dir.join("handoff-ledger.jsonl"));
crates/terraphim_orchestrator/src/lib.rs:986:                    config.working_dir.join("logs").join("agents")
crates/terraphim_orchestrator/src/lib.rs:1618:            .working_dir
crates/terraphim_orchestrator/src/lib.rs:2296:        // Create isolated git worktree for implementation-tier agents that modify code.
crates/terraphim_orchestrator/src/lib.rs:2300:        // Resolve the git repo directory for worktree operations. Project-bound
crates/terraphim_orchestrator/src/lib.rs:2301:        // agents need a worktree from their own repo, not the orchestrator's.
crates/terraphim_orchestrator/src/lib.rs:2304:                Some(p) => p.working_dir.as_path(),
crates/terraphim_orchestrator/src/lib.rs:2309:                        fallback = %self.config.working_dir.display(),
crates/terraphim_orchestrator/src/lib.rs:2310:                        "project_by_id returned None, falling back to orchestrator working_dir"
crates/terraphim_orchestrator/src/lib.rs:2312:                    &self.config.working_dir
crates/terraphim_orchestrator/src/lib.rs:2316:            &self.config.working_dir
crates/terraphim_orchestrator/src/lib.rs:2319:        let (worktree_path, worktree_guard) = if needs_isolation {
crates/terraphim_orchestrator/src/lib.rs:2320:            if let Some(path) = self.create_agent_worktree(&def.name, repo_dir).await {
crates/terraphim_orchestrator/src/lib.rs:2321:                let guard = crate::worktree_guard::WorktreeGuard::for_managed(repo_dir, &path);
crates/terraphim_orchestrator/src/lib.rs:2329:        let agent_working_dir = worktree_path.as_deref().unwrap_or(repo_dir).to_path_buf();
crates/terraphim_orchestrator/src/lib.rs:2338:                working_dir: agent_working_dir.clone(),
crates/terraphim_orchestrator/src/lib.rs:2354:                    working_dir: agent_working_dir.clone(),
crates/terraphim_orchestrator/src/lib.rs:2443:                worktree_path,
crates/terraphim_orchestrator/src/lib.rs:2444:                worktree_guard,
crates/terraphim_orchestrator/src/lib.rs:2493:    /// chain injection, and worktree creation. The pr-reviewer is review-tier
crates/terraphim_orchestrator/src/lib.rs:2701:                working_dir: self.config.working_dir_for_agent(&def),
crates/terraphim_orchestrator/src/lib.rs:2716:                    working_dir: self.config.working_dir_for_agent(&def),
crates/terraphim_orchestrator/src/lib.rs:2785:                worktree_path: None,
crates/terraphim_orchestrator/src/lib.rs:2786:                worktree_guard: None,
crates/terraphim_orchestrator/src/lib.rs:2913:                working_dir: self.config.working_dir_for_agent(&def),
crates/terraphim_orchestrator/src/lib.rs:2988:                worktree_path: None,
crates/terraphim_orchestrator/src/lib.rs:2989:                worktree_guard: None,
crates/terraphim_orchestrator/src/lib.rs:3232:                working_dir: self.config.working_dir_for_agent(&def),
crates/terraphim_orchestrator/src/lib.rs:3302:                worktree_path: None,
crates/terraphim_orchestrator/src/lib.rs:3303:                worktree_guard: None,
crates/terraphim_orchestrator/src/lib.rs:3382:                .current_dir(&self.config.working_dir)
crates/terraphim_orchestrator/src/lib.rs:3432:                .current_dir(&self.config.working_dir)
crates/terraphim_orchestrator/src/lib.rs:3540:                    .current_dir(&self.config.working_dir)
crates/terraphim_orchestrator/src/lib.rs:3590:                .current_dir(&self.config.working_dir)
crates/terraphim_orchestrator/src/lib.rs:5065:    /// project's `working_dir` as `repo_root`, constructs the [`post_merge_gate::GateConfig`]
crates/terraphim_orchestrator/src/lib.rs:5114:        // Legacy mode uses the top-level `working_dir` and `gitea`.
crates/terraphim_orchestrator/src/lib.rs:5124:            (self.config.working_dir.clone(), self.config.gitea.clone())
crates/terraphim_orchestrator/src/lib.rs:5127:                Some(p) => (p.working_dir.clone(), p.gitea.clone()),
crates/terraphim_orchestrator/src/lib.rs:5687:    /// Create a git worktree for an agent to work in isolation.
crates/terraphim_orchestrator/src/lib.rs:5689:    /// `repo_dir` is the git repository root where `git worktree add` runs.
crates/terraphim_orchestrator/src/lib.rs:5690:    /// For project-bound agents this is the project's working_dir; otherwise
crates/terraphim_orchestrator/src/lib.rs:5691:    /// it is the orchestrator's top-level working_dir.
crates/terraphim_orchestrator/src/lib.rs:5693:    /// Returns the worktree path if successful, None if worktree creation fails
crates/terraphim_orchestrator/src/lib.rs:5694:    /// (fail-open: agent uses shared working_dir instead).
crates/terraphim_orchestrator/src/lib.rs:5695:    async fn create_agent_worktree(&self, agent_name: &str, repo_dir: &Path) -> Option<PathBuf> {
crates/terraphim_orchestrator/src/lib.rs:5696:        let worktree_root = repo_dir.join(".worktrees");
crates/terraphim_orchestrator/src/lib.rs:5697:        if let Err(e) = tokio::fs::create_dir_all(&worktree_root).await {
crates/terraphim_orchestrator/src/lib.rs:5698:            warn!(agent = %agent_name, error = %e, "failed to create worktree root");
crates/terraphim_orchestrator/src/lib.rs:5703:        let worktree_path = worktree_root.join(format!("{agent_name}-{id}"));
crates/terraphim_orchestrator/src/lib.rs:5707:                "worktree",
crates/terraphim_orchestrator/src/lib.rs:5710:                &worktree_path.to_string_lossy(),
crates/terraphim_orchestrator/src/lib.rs:5721:                    path = %worktree_path.display(),
crates/terraphim_orchestrator/src/lib.rs:5723:                    "created isolated git worktree"
crates/terraphim_orchestrator/src/lib.rs:5725:                Some(worktree_path)
crates/terraphim_orchestrator/src/lib.rs:5732:                    "git worktree add failed, using shared working_dir"
crates/terraphim_orchestrator/src/lib.rs:5737:                warn!(agent = %agent_name, error = %e, "git worktree command failed");
crates/terraphim_orchestrator/src/lib.rs:5743:    /// Remove a git worktree after an agent finishes.
crates/terraphim_orchestrator/src/lib.rs:5744:    async fn remove_agent_worktree(&self, agent_name: &str, worktree_path: &Path, repo_dir: &Path) {
crates/terraphim_orchestrator/src/lib.rs:5749:                "worktree",
crates/terraphim_orchestrator/src/lib.rs:5752:                &worktree_path.to_string_lossy(),
crates/terraphim_orchestrator/src/lib.rs:5762:                    path = %worktree_path.display(),
crates/terraphim_orchestrator/src/lib.rs:5763:                    "removed agent worktree"
crates/terraphim_orchestrator/src/lib.rs:5770:                    path = %worktree_path.display(),
crates/terraphim_orchestrator/src/lib.rs:5772:                    "git worktree remove failed"
crates/terraphim_orchestrator/src/lib.rs:5776:                warn!(agent = %agent_name, error = %e, "git worktree remove command failed");
crates/terraphim_orchestrator/src/lib.rs:5786:    async fn try_commit_agent_work(&self, agent_name: &str, working_dir: &Path) {
crates/terraphim_orchestrator/src/lib.rs:5790:            .current_dir(working_dir)
crates/terraphim_orchestrator/src/lib.rs:5802:            .current_dir(working_dir)
crates/terraphim_orchestrator/src/lib.rs:5821:            .current_dir(working_dir)
crates/terraphim_orchestrator/src/lib.rs:5834:            .current_dir(working_dir)
crates/terraphim_orchestrator/src/lib.rs:7023:        // Capture worktree_path before removing so we can commit + clean up.
crates/terraphim_orchestrator/src/lib.rs:7025:            let (worktree_path, commit_status_post) = {
crates/terraphim_orchestrator/src/lib.rs:7028:                    agent.and_then(|m| m.worktree_path.clone()),
crates/terraphim_orchestrator/src/lib.rs:7050:            // Disarm worktree guard on success so it doesn't conflict with
crates/terraphim_orchestrator/src/lib.rs:7054:                    if let Some(guard) = agent.worktree_guard.take() {
crates/terraphim_orchestrator/src/lib.rs:7185:            // Auto-commit in the agent's working directory (worktree or shared)
crates/terraphim_orchestrator/src/lib.rs:7190:                .map(|p| p.working_dir.as_path())
crates/terraphim_orchestrator/src/lib.rs:7191:                .unwrap_or(&self.config.working_dir);
crates/terraphim_orchestrator/src/lib.rs:7192:            let commit_dir = worktree_path.as_deref().unwrap_or(project_repo);
crates/terraphim_orchestrator/src/lib.rs:7197:            // Clean up the worktree after committing
crates/terraphim_orchestrator/src/lib.rs:7198:            if let Some(ref wt) = worktree_path {
crates/terraphim_orchestrator/src/lib.rs:7199:                self.remove_agent_worktree(&name, wt, project_repo).await;
crates/terraphim_orchestrator/src/lib.rs:7979:                let working_dir = self.config.compound_review.repo_path.clone();
crates/terraphim_orchestrator/src/lib.rs:7987:                    let executor = flow::executor::FlowExecutor::new(working_dir, flow_state_dir)
crates/terraphim_orchestrator/src/lib.rs:8322:            working_dir: std::path::PathBuf::from("/tmp/test-orchestrator"),
crates/terraphim_orchestrator/src/lib.rs:8332:                worktree_root: std::path::PathBuf::from("/tmp/test-orchestrator/.worktrees"),
crates/terraphim_orchestrator/src/lib.rs:8606:        // Use empty groups to avoid git worktree operations during test.
crates/terraphim_orchestrator/src/lib.rs:8607:        // Worktree creation fails when git index is locked (e.g. pre-commit hooks).
crates/terraphim_orchestrator/src/lib.rs:8645:            worktree_root: std::path::PathBuf::from("/tmp/test-orchestrator/.worktrees"),
crates/terraphim_orchestrator/src/lib.rs:8675:    /// fire", spawning a new worktree every tick (the bigbox storm).
crates/terraphim_orchestrator/src/lib.rs:8679:        // workflow has no review groups -- it still creates a worktree
crates/terraphim_orchestrator/src/lib.rs:8683:        let tmp_worktree = TempDir::new().expect("tempdir");
crates/terraphim_orchestrator/src/lib.rs:8684:        config.compound_review.worktree_root = tmp_worktree.path().to_path_buf();
crates/terraphim_orchestrator/src/lib.rs:8693:        // worktree creation/removal. The orchestrator's
crates/terraphim_orchestrator/src/lib.rs:8699:            worktree_root: tmp_worktree.path().to_path_buf(),
crates/terraphim_orchestrator/src/lib.rs:8745:working_dir = "/tmp"
crates/terraphim_orchestrator/src/lib.rs:8819:            working_dir: std::path::PathBuf::from("/tmp"),
crates/terraphim_orchestrator/src/lib.rs:8829:                worktree_root: std::path::PathBuf::from("/tmp/.worktrees"),
crates/terraphim_orchestrator/src/lib.rs:9850:        let working_dir = tmp.path().to_path_buf();
crates/terraphim_orchestrator/src/lib.rs:9852:            working_dir: working_dir.clone(),
crates/terraphim_orchestrator/src/lib.rs:9860:                repo_path: working_dir.clone(),
crates/terraphim_orchestrator/src/lib.rs:9862:                worktree_root: working_dir.join(".worktrees"),
crates/terraphim_orchestrator/src/lib.rs:9916:                working_dir: working_dir.clone(),
crates/terraphim_orchestrator/src/lib.rs:11904:    /// (worktrees, real spawning, output capture) disproportionate to a single
crates/terraphim_orchestrator/src/project_adf.rs:211:            working_dir: cfg
crates/terraphim_orchestrator/src/project_adf.rs:559:            working_dir: adf.discovered_path.parent().unwrap().to_path_buf(),
crates/terraphim_orchestrator/src/config.rs:48:    /// Per-project working directory (overrides top-level working_dir for this project's agents).
crates/terraphim_orchestrator/src/config.rs:49:    pub working_dir: PathBuf,
crates/terraphim_orchestrator/src/config.rs:80:    pub working_dir: PathBuf,
crates/terraphim_orchestrator/src/config.rs:939:    /// Root directory for worktrees.
crates/terraphim_orchestrator/src/config.rs:940:    #[serde(default = "default_worktree_root")]
crates/terraphim_orchestrator/src/config.rs:941:    pub worktree_root: PathBuf,
crates/terraphim_orchestrator/src/config.rs:975:fn default_worktree_root() -> PathBuf {
crates/terraphim_orchestrator/src/config.rs:976:    PathBuf::from(".worktrees")
crates/terraphim_orchestrator/src/config.rs:994:            worktree_root: default_worktree_root(),
crates/terraphim_orchestrator/src/config.rs:1397:    /// `working_dir` if the agent has a `project` and it matches a known
crates/terraphim_orchestrator/src/config.rs:1398:    /// project, else the top-level working_dir.
crates/terraphim_orchestrator/src/config.rs:1399:    pub fn working_dir_for_agent(&self, agent: &AgentDefinition) -> PathBuf {
crates/terraphim_orchestrator/src/config.rs:1404:            .map(|p| p.working_dir.clone())
crates/terraphim_orchestrator/src/config.rs:1405:            .unwrap_or_else(|| self.working_dir.clone())
crates/terraphim_orchestrator/src/config.rs:1722:working_dir = "/tmp/terraphim"
crates/terraphim_orchestrator/src/config.rs:1747:working_dir = "/Users/alex/projects/terraphim/terraphim-ai"
crates/terraphim_orchestrator/src/config.rs:1822:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:1858:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:1882:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:1923:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:1944:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2006:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2045:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2077:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2100:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2122:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2171:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2203:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2233:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2257:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2279:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2304:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2335:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2355:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2384:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2410:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2440:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2471:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2489:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2517:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2547:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2613:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:2732:working_dir = "/tmp/pr-dispatch-default-test"
crates/terraphim_orchestrator/src/config.rs:2761:working_dir = "/tmp/pr-dispatch-accessor-test"
crates/terraphim_orchestrator/src/config.rs:2783:working_dir = "/tmp/pr-dispatch-accessor-configured"
crates/terraphim_orchestrator/src/config.rs:2812:working_dir = "/tmp/per-project"
crates/terraphim_orchestrator/src/config.rs:2865:working_dir = "/tmp/pr-dispatch-parse-test"
crates/terraphim_orchestrator/src/config.rs:2902:working_dir = "/tmp/terraphim"
crates/terraphim_orchestrator/src/config.rs:2949:working_dir = "/tmp/alpha"
crates/terraphim_orchestrator/src/config.rs:2965:working_dir = "/tmp/beta"
crates/terraphim_orchestrator/src/config.rs:2979:working_dir = "/tmp/o"
crates/terraphim_orchestrator/src/config.rs:3044:working_dir = "/tmp/o"
crates/terraphim_orchestrator/src/config.rs:3066:working_dir = "/tmp/terraphim"
crates/terraphim_orchestrator/src/config.rs:3091:working_dir = "/tmp/terraphim"
crates/terraphim_orchestrator/src/config.rs:3158:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3184:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3210:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3231:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3257:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3283:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3304:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3329:working_dir = "/tmp"
crates/terraphim_orchestrator/src/config.rs:3350:working_dir = "/tmp"
crates/terraphim_orchestrator/src/worktree_guard.rs:1://! RAII worktree guard for automatic cleanup on agent crash or panic.
crates/terraphim_orchestrator/src/worktree_guard.rs:3://! Ensures that agent worktrees are cleaned up even when the agent process
crates/terraphim_orchestrator/src/worktree_guard.rs:9://! use terraphim_orchestrator::worktree_guard::WorktreeGuard;
crates/terraphim_orchestrator/src/worktree_guard.rs:12://!     let guard = WorktreeGuard::new("/tmp/agent-worktree-123");
crates/terraphim_orchestrator/src/worktree_guard.rs:14://! } // worktree automatically cleaned up here
crates/terraphim_orchestrator/src/worktree_guard.rs:21:/// RAII guard that removes a worktree directory when dropped.
crates/terraphim_orchestrator/src/worktree_guard.rs:24:/// you want to preserve the worktree for inspection).
crates/terraphim_orchestrator/src/worktree_guard.rs:26:pub struct WorktreeGuard {
crates/terraphim_orchestrator/src/worktree_guard.rs:29:    /// When `Some`, `Drop` runs `git -C <repo_path> worktree remove
crates/terraphim_orchestrator/src/worktree_guard.rs:37:impl WorktreeGuard {
crates/terraphim_orchestrator/src/worktree_guard.rs:38:    /// Create a new worktree guard for the given path.
crates/terraphim_orchestrator/src/worktree_guard.rs:42:    /// cleanup; use `for_managed` for git-aware cleanup of worktrees
crates/terraphim_orchestrator/src/worktree_guard.rs:43:    /// created via `WorktreeManager::create_worktree`.
crates/terraphim_orchestrator/src/worktree_guard.rs:46:        debug!(path = %path.display(), "worktree guard created");
crates/terraphim_orchestrator/src/worktree_guard.rs:54:    /// Create a managed guard whose `Drop` invokes `git worktree
crates/terraphim_orchestrator/src/worktree_guard.rs:58:    /// Use this when the worktree was created via
crates/terraphim_orchestrator/src/worktree_guard.rs:59:    /// `WorktreeManager::create_worktree` so the git admin registry
crates/terraphim_orchestrator/src/worktree_guard.rs:60:    /// at `<repo>/.git/worktrees/<name>` is reconciled along with the
crates/terraphim_orchestrator/src/worktree_guard.rs:62:    pub fn for_managed<R: AsRef<Path>, P: AsRef<Path>>(repo_path: R, worktree_path: P) -> Self {
crates/terraphim_orchestrator/src/worktree_guard.rs:63:        let path = worktree_path.as_ref().to_path_buf();
crates/terraphim_orchestrator/src/worktree_guard.rs:67:            worktree_path = %path.display(),
crates/terraphim_orchestrator/src/worktree_guard.rs:68:            "managed worktree guard created"
crates/terraphim_orchestrator/src/worktree_guard.rs:79:    /// Call this when the agent succeeds and you want to keep the worktree.
crates/terraphim_orchestrator/src/worktree_guard.rs:82:        debug!(path = %self.path.display(), "worktree guard disarmed");
crates/terraphim_orchestrator/src/worktree_guard.rs:85:    /// Get the worktree path.
crates/terraphim_orchestrator/src/worktree_guard.rs:97:            debug!(path = %self.path.display(), "worktree already removed");
crates/terraphim_orchestrator/src/worktree_guard.rs:101:        // Managed path: try `git worktree remove --force` first so the
crates/terraphim_orchestrator/src/worktree_guard.rs:102:        // git admin entry at `<repo>/.git/worktrees/<name>` is
crates/terraphim_orchestrator/src/worktree_guard.rs:104:        // Drop cannot be async, and git worktree remove is sub-second.
crates/terraphim_orchestrator/src/worktree_guard.rs:110:                .arg("worktree")
crates/terraphim_orchestrator/src/worktree_guard.rs:122:                        "worktree cleaned up via git"
crates/terraphim_orchestrator/src/worktree_guard.rs:130:                        "git worktree remove failed, falling back to fs"
crates/terraphim_orchestrator/src/worktree_guard.rs:146:                info!(path = %self.path.display(), "worktree cleaned up");
crates/terraphim_orchestrator/src/worktree_guard.rs:149:                warn!(path = %self.path.display(), error = %e, "failed to remove worktree");
crates/terraphim_orchestrator/src/worktree_guard.rs:152:                    debug!(path = %self.path.display(), error = %e2, "failed to remove worktree dir");
crates/terraphim_orchestrator/src/worktree_guard.rs:159:impl Drop for WorktreeGuard {
crates/terraphim_orchestrator/src/worktree_guard.rs:165:/// Scoped worktree guard that wraps a closure and ensures cleanup.
crates/terraphim_orchestrator/src/worktree_guard.rs:169:pub fn with_worktree_guard<F, T, P: AsRef<Path>>(path: P, f: F) -> T
crates/terraphim_orchestrator/src/worktree_guard.rs:171:    F: FnOnce(&WorktreeGuard) -> T,
crates/terraphim_orchestrator/src/worktree_guard.rs:173:    let guard = WorktreeGuard::new(path);
crates/terraphim_orchestrator/src/worktree_guard.rs:177:/// Async version of `with_worktree_guard`.
crates/terraphim_orchestrator/src/worktree_guard.rs:178:pub async fn with_worktree_guard_async<F, T, P: AsRef<Path>>(path: P, f: F) -> T
crates/terraphim_orchestrator/src/worktree_guard.rs:182:    let _guard = WorktreeGuard::new(path);
crates/terraphim_orchestrator/src/worktree_guard.rs:195:    fn test_worktree_guard_cleanup() {
crates/terraphim_orchestrator/src/worktree_guard.rs:197:        let worktree = temp_dir.path().join("worktree-123");
crates/terraphim_orchestrator/src/worktree_guard.rs:198:        std::fs::create_dir(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:199:        File::create(worktree.join("file.txt")).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:201:        assert!(worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:204:            let _guard = WorktreeGuard::new(&worktree);
crates/terraphim_orchestrator/src/worktree_guard.rs:206:            assert!(worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:210:        assert!(!worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:214:    fn test_worktree_guard_keep() {
crates/terraphim_orchestrator/src/worktree_guard.rs:216:        let worktree = temp_dir.path().join("worktree-456");
crates/terraphim_orchestrator/src/worktree_guard.rs:217:        std::fs::create_dir(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:220:            let guard = WorktreeGuard::new(&worktree);
crates/terraphim_orchestrator/src/worktree_guard.rs:225:        assert!(worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:229:    fn test_worktree_guard_already_removed() {
crates/terraphim_orchestrator/src/worktree_guard.rs:231:        let worktree = temp_dir.path().join("worktree-789");
crates/terraphim_orchestrator/src/worktree_guard.rs:232:        std::fs::create_dir(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:235:            let _guard = WorktreeGuard::new(&worktree);
crates/terraphim_orchestrator/src/worktree_guard.rs:237:            std::fs::remove_dir_all(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:241:        assert!(!worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:245:    fn test_with_worktree_guard() {
crates/terraphim_orchestrator/src/worktree_guard.rs:247:        let worktree = temp_dir.path().join("worktree-scoped");
crates/terraphim_orchestrator/src/worktree_guard.rs:248:        std::fs::create_dir(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:250:        let result = with_worktree_guard(&worktree, |guard| {
crates/terraphim_orchestrator/src/worktree_guard.rs:256:        assert!(!worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:293:        let worktree = repo.path().join(".worktrees/managed-remove");
crates/terraphim_orchestrator/src/worktree_guard.rs:295:        // Use real git worktree add so the admin entry exists.
crates/terraphim_orchestrator/src/worktree_guard.rs:299:            .arg("worktree")
crates/terraphim_orchestrator/src/worktree_guard.rs:301:            .arg(&worktree)
crates/terraphim_orchestrator/src/worktree_guard.rs:305:            .expect("git worktree add");
crates/terraphim_orchestrator/src/worktree_guard.rs:306:        assert!(status.success(), "git worktree add failed");
crates/terraphim_orchestrator/src/worktree_guard.rs:307:        assert!(worktree.exists());
crates/terraphim_orchestrator/src/worktree_guard.rs:309:        let admin = repo.path().join(".git/worktrees/managed-remove");
crates/terraphim_orchestrator/src/worktree_guard.rs:313:            let _guard = WorktreeGuard::for_managed(repo.path(), &worktree);
crates/terraphim_orchestrator/src/worktree_guard.rs:317:            !worktree.exists(),
crates/terraphim_orchestrator/src/worktree_guard.rs:318:            "managed guard should remove worktree dir"
crates/terraphim_orchestrator/src/worktree_guard.rs:328:        // Point repo_path at a non-git directory so `git worktree
crates/terraphim_orchestrator/src/worktree_guard.rs:334:        let worktree = temp_dir.path().join("orphan-worktree");
crates/terraphim_orchestrator/src/worktree_guard.rs:335:        std::fs::create_dir(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:336:        File::create(worktree.join("payload.txt")).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:339:            let _guard = WorktreeGuard::for_managed(&not_a_repo, &worktree);
crates/terraphim_orchestrator/src/worktree_guard.rs:343:            !worktree.exists(),
crates/terraphim_orchestrator/src/worktree_guard.rs:344:            "fallback fs removal should remove worktree dir"
crates/terraphim_orchestrator/src/worktree_guard.rs:353:        let worktree = temp_dir.path().join("kept-worktree");
crates/terraphim_orchestrator/src/worktree_guard.rs:354:        std::fs::create_dir(&worktree).unwrap();
crates/terraphim_orchestrator/src/worktree_guard.rs:356:        let guard = WorktreeGuard::for_managed(&fake_repo, &worktree);
crates/terraphim_orchestrator/src/worktree_guard.rs:360:            worktree.exists(),
crates/terraphim_orchestrator/src/control_plane/routing.rs:108:            working_dir: PathBuf::from(
crates/terraphim_orchestrator/src/control_plane/routing.rs:999:                    working_dir: PathBuf::from("/tmp"),
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:966:        format!("ls .worktrees/ 2>/dev/null | grep '^{}-'", validated_name)
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:969:            "ls /tmp/adf-worktrees/ 2>/dev/null | grep '^{}-'",
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:973:    let (worktrees, _, _) = if local {
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:990:    if worktrees.trim().is_empty() && procs.trim().is_empty() {
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:992:            "No active worktrees or agent CLI processes found for '{}'.",
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:998:    if !worktrees.trim().is_empty() {
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:999:        println!("Active worktrees for '{}':", validated_name);
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:1000:        for wt in worktrees.lines() {
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:1002:                println!("  .worktrees/{}", wt.trim());
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:1004:                println!("  /tmp/adf-worktrees/{}", wt.trim());
crates/terraphim_orchestrator/src/bin/adf-ctl.rs:1169:        working_dir: cwd.clone(),
crates/terraphim_orchestrator/src/bin/adf.rs:10:use terraphim_spawner::{AgentSpawner, ResourceLimits, SpawnContext};
crates/terraphim_orchestrator/src/bin/adf.rs:236:            let working_dir = adf_config
crates/terraphim_orchestrator/src/bin/adf.rs:245:                repo_path: working_dir.clone(),
crates/terraphim_orchestrator/src/bin/adf.rs:250:                working_dir,
crates/terraphim_orchestrator/src/bin/adf.rs:340:            let working_dir = adf_config
crates/terraphim_orchestrator/src/bin/adf.rs:349:                repo_path: working_dir.clone(),
crates/terraphim_orchestrator/src/bin/adf.rs:354:                working_dir,
crates/terraphim_orchestrator/src/bin/adf.rs:463:    let working_dir = adf_config
crates/terraphim_orchestrator/src/bin/adf.rs:472:        repo_path: working_dir.clone(),
crates/terraphim_orchestrator/src/bin/adf.rs:477:        working_dir,
crates/terraphim_orchestrator/src/bin/adf.rs:528:/// Build a SpawnContext for a local agent from an OrchestratorConfig and agent definition.
crates/terraphim_orchestrator/src/bin/adf.rs:533:) -> SpawnContext {
crates/terraphim_orchestrator/src/bin/adf.rs:536:        None => return SpawnContext::global(),
crates/terraphim_orchestrator/src/bin/adf.rs:540:        None => return SpawnContext::global(),
crates/terraphim_orchestrator/src/bin/adf.rs:542:    let working_dir_str = project.working_dir.to_string_lossy().into_owned();
crates/terraphim_orchestrator/src/bin/adf.rs:543:    let mut ctx = SpawnContext::with_working_dir(project.working_dir.clone())
crates/terraphim_orchestrator/src/bin/adf.rs:545:        .with_env("ADF_WORKING_DIR", working_dir_str);
crates/terraphim_orchestrator/src/bin/adf.rs:583:    let working_dir = adf_config
crates/terraphim_orchestrator/src/bin/adf.rs:592:        repo_path: working_dir.clone(),
crates/terraphim_orchestrator/src/bin/adf.rs:597:        working_dir,
crates/terraphim_orchestrator/src/bin/adf.rs:676:            working_dir: config.working_dir_for_agent(&def),
crates/terraphim_orchestrator/src/bin/adf.rs:690:            working_dir: config.working_dir_for_agent(&def),
crates/terraphim_orchestrator/src/control_plane/policy.rs:269:                    working_dir: PathBuf::from("/tmp"),

```
