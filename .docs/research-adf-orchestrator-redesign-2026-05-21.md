# Research Document: ADF Orchestrator Redesign

## 1. Problem Restatement and Scope

The current ADF orchestrator has grown beyond the original "thin controller loop" idea. It now owns project-source loading, cron scheduling, mention dispatch, PR gate reconciliation, provider probing, cost and provider budgets, Quickwit shipping, worktree lifecycle, fallback routing, learning/evolution, output posting, drift checks, flow execution, and multi-project policy enforcement. These responsibilities are executed through one large `AgentOrchestrator` and a sequential `reconcile_tick` path.

The immediate request to raise `reconcile_tick` to 250 seconds is a stabilisation action, not the architectural fix. The deeper problem is that ADF is carrying multiple orchestration styles at once: Kubernetes-style reconciliation, Symphony-style issue dispatch/retry state, OTP-style supervision, Gitea-mediated agent coordination, and CI/PR gate automation. The current crate no longer has clear enough boundaries for the upcoming features such as structured RPC envelopes, Agent SDK integration, session persistence, activity logs, approvals, and native pi-rust agent runtime.

IN scope:

- Local ADF orchestrator code and related modules.
- Symphony orchestrator code and documented invariants.
- OTP-inspired supervisor and messaging capabilities already present in the workspace.
- ADF logs from bigbox showing current operational failure modes.
- GitHub and Gitea tickets/PRs that describe upcoming ADF features.
- Existing research/design documents about ADF, Symphony, project sources, stability, fallback, and inter-agent orchestration.

OUT of scope:

- Full Phase 2 implementation design.
- Removing or rewriting current production ADF immediately.
- Replacing Gitea task tracking.
- Choosing exact protocol schema beyond documenting existing requirements.
- Designing distributed multi-host orchestration; current deployment remains single bigbox.

## 2. User & Business Outcomes

Desired outcomes:

- ADF remains stable under slow providers, Gitea API failures, and long-running agents.
- ADF can adopt upcoming capabilities without more responsibilities being added to one monolithic tick loop.
- Project-local ADF configuration remains versioned in each repo while fleet-level policy remains centralised.
- Agent runs become more observable, resumable, and governable.
- PR gates and task automation stop depending on fragile text parsing and one sequential control loop.

Business value:

- Fewer production-only emergency fixes on bigbox.
- Lower risk of duplicate agent work and blocked PR queues.
- Clearer ownership boundaries for new features: runtime, scheduling, supervision, governance, and project policy.
- Better fit for future Agent SDK, pi-rust, and JSONL envelope work.

## 3. System Elements and Dependencies

| Element | Location | Current Responsibility | Dependencies / Concerns |
|---------|----------|------------------------|--------------------------|
| `AgentOrchestrator` | `crates/terraphim_orchestrator/src/lib.rs` | Main production ADF loop, lifecycle, spawn, dispatch, output, gates, schedules, probes | Large shared state, sequential tick, many subsystem calls |
| Runtime config | `crates/terraphim_orchestrator/src/config.rs` | Global fleet config, includes, project sources, agents, workflows, budgets, PR dispatch | Increasingly broad schema; fleet and project concerns coexist |
| Project-source loader | `config.rs`, `project_adf.rs`, `.terraphim/adf.toml` | Loads repo-local agent/project config | Good direction, but still merged into monolithic runtime config |
| Provider probe | `provider_probe.rs` | Health checks and Quickwit shipment for providers/models | Slow external subprocess calls can dominate tick latency |
| Worktree management | `scope.rs`, `worktree_guard.rs` | Isolated agent worktrees and cleanup | Logs show repeated stale/no-manifest worktrees and prior corrupt repo failure |
| Dispatcher queue | `dispatcher.rs` | Priority queue and project fairness for dispatch tasks | Better boundary than direct cron spawning, but still drained by main tick |
| PR gates | `pr_gate.rs`, `pr_review.rs`, `post_merge_gate.rs` | Branch-protection reconciliation and PR statuses | Logs show repeated 404/403 branch-protection failures and blocked checks |
| Mention system | `mention.rs`, `mention_chain.rs`, `output_poster.rs` | Gitea comment polling, `@adf:` parsing, output posting | Current coordination is human-readable but not yet a structured workflow protocol |
| Symphony orchestrator | `crates/terraphim_symphony/src/orchestrator/*` | Issue dispatch/retry/reconcile with serialised runtime state | Clearer state model: running, claimed, retrying, completed |
| OTP supervisor | `crates/terraphim_agent_supervisor/src/*` | Fault-tolerant supervision trees and restart strategies | Exists but ADF production lifecycle mostly uses custom management paths |
| OTP messaging | `crates/terraphim_agent_messaging/src/*` | Message routing, mailboxes, delivery guarantees | Exists but ADF agents still mainly use stdout/stderr and Gitea comments |
| Spawner | `crates/terraphim_spawner/src/*` | CLI process spawn, output capture, health, circuit breaker primitives | ADF needs typed runtime events rather than ad-hoc parsing |
| Quickwit/logging | `quickwit.rs`, `quickwit_bulk.rs`, logs | Observability and provider probe results | Current logs are valuable but not a durable activity/state model |
| GitHub tickets | #688, #689, #641, #642 | JSONL RPC envelope, Agent SDK migration, activity log, approvals | These require explicit protocol and governance boundaries |
| Gitea issue/PR | `terraphim-ai#1769`, PR #1782, issue #1785 | Project-source loader, pi-rust bridge, native runtime integration | Evidence that features are converging on a runtime abstraction layer |

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Bigbox is production infrastructure | ADF currently drives real PRs, tickets, and automation | Migration must be incremental and reversible |
| Agents are mostly CLI processes today | Claude/opencode/codex/pi-rust do not share a native API | Runtime abstraction must support subprocess and SDK-backed runners |
| Gitea is the task source of truth | Issues, comments, and PR statuses are operational records | Orchestrator should keep Gitea integration, but isolate it from runtime supervision |
| Project-local config is desired | `project_sources` and `.terraphim/adf.toml` reduce production config drift | Project policy should not be flattened into one global agent namespace internally |
| Provider calls are slow/unreliable | Logs show repeated provider probe failures/timeouts | Provider health must be asynchronous and cached, not a blocking tick step |
| PR gate permissions differ by project | Logs show branch protection 404/403 for several projects | Gate reconcilers need per-project capability checks and backoff |
| Worktrees can become corrupt/stale | Logs show invalid HEAD, no-manifest worktrees, and a re-cloned bigbox repo | Workspace lifecycle needs stronger ownership and terminal-state cleanup semantics |
| Upcoming features need typed events | GitHub #688/#689/#641/#642 require RPC envelopes, SDK events, activity logs, approvals | Text parsing cannot remain the control plane contract |
| Single host, many subsystems | No distributed consensus needed | Prefer local actor/supervision boundaries over distributed architecture |

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Severity | Evidence | De-risking Step |
|------|----------|----------|-----------------|
| Monolithic tick continues to accumulate responsibilities | High | `AgentOrchestrator` owns many unrelated systems and tick failures affect all work | Split tick work into independent actors/jobs with bounded budgets |
| Provider probing starves control-plane work | High | Recent 90s timeout came from probe work exceeding tick budget | Move probes to independent background health actor with TTL cache |
| PR gate reconciliation floods logs and repeats impossible checks | Medium | Branch-protection 404/403 repeated for multiple projects | Add project capability discovery and exponential backoff |
| Worktree cleanup remains ambiguous | Medium | Stale/no-manifest worktrees repeatedly skipped | Make workspace ownership explicit and persist terminal run state |
| Upcoming JSONL/SDK work becomes another adapter inside `lib.rs` | High | GitHub #688/#689 require typed protocol and SDK bridging | Introduce runtime boundary before adding SDK/pi-rust support |
| Symphony and OTP code remain parallel unused solutions | Medium | Both exist with clearer models but are not the production ADF core | Decide whether ADF should adopt their state/supervision primitives |
| Longer 250s tick hides symptoms | Medium | Larger timeout prevents false continuation but allows slow work to remain hidden | Keep slow-step warnings and extract slow work next |

### Unknowns

- Whether every existing ADF feature is required in the same daemon process.
- Whether Symphony should become the issue-dispatch core or remain a reference implementation.
- Whether OTP supervisor/messaging crates are production-ready for the exact CLI-agent lifecycle ADF needs.
- Whether project-source configs should compile into isolated per-project controllers rather than a merged global config.
- Whether Quickwit should be the durable activity log or only a sink fed from a first-class activity journal.
- Whether PR gates should be owned by ADF or delegated to a smaller CI/gate service.

### Assumptions

- ASSUMPTION: Bigbox remains the production host for the next migration phase.
- ASSUMPTION: Gitea remains authoritative for issue state, PRs, comments, and task dependencies.
- ASSUMPTION: The current ADF must keep running while a new design is introduced.
- ASSUMPTION: Symphony's smaller state model is closer to the desired issue-runner core than the current ADF tick loop.
- ASSUMPTION: OTP supervision is valuable for lifecycle and restart policy, but not necessarily for every control-plane concern.

## 6. Context Complexity vs. Simplicity Opportunities

Complexity sources:

- One orchestrator struct owns fleet policy, project policy, execution runtime, provider health, governance, PR gates, output posting, and cleanup.
- Slow or failing external integrations are called in the same tick path as safety-critical lifecycle work.
- Agent identity is becoming project-scoped, but several operational paths still treat names and maps as global-ish concerns.
- Runtime communication is split across stdout/stderr parsing, Gitea comments, Quickwit logs, and ad-hoc files.
- Prior research identified multiple good patterns, but they were integrated by accretion rather than by enforcing architectural boundaries.

Simplification opportunities:

1. Treat ADF as a fleet control plane plus several supervised actors, not one reconcile function. Separate actors should own provider health, project dispatch, PR gates, workspace cleanup, and activity/governance.
2. Use Symphony's state model for issue/work dispatch: `running`, `claimed`, `retrying`, `completed`, explicit stall detection, and deterministic dispatch eligibility.
3. Use OTP supervision for runtime agents and background services: provider probe actor, PR gate actor, mention actor, workspace sweeper, and project controllers should fail/restart independently.
4. Introduce a typed runtime event boundary before adding Agent SDK and pi-rust. JSONL envelope events should feed activity logs, session persistence, cost tracking, and output posting.
5. Keep Gitea comments as the human audit surface, but stop treating comment text as the internal orchestration protocol.

Preliminary better-fit direction:

- **Fleet Kernel**: minimal process that loads fleet config, project sources, secrets, and starts supervised actors.
- **Project Controller**: one controller per project, owning project config, issue/PR queues, and per-project concurrency.
- **Run Supervisor**: OTP-style lifecycle owner for each agent run, handling spawn, timeout, restart, fallback, and workspace ownership.
- **Runtime Adapter**: pluggable subprocess/Claude SDK/pi-rust adapters that all emit the same JSONL/event envelope.
- **Background Actors**: provider health, PR gates, mention polling/webhook handling, workspace sweeps, and telemetry run independently with their own cadence/backoff.
- **Activity/Governance Journal**: append-only structured record that feeds Quickwit, Gitea comments, approvals, and human reports.

This direction preserves useful ADF work while making Symphony and OTP primitives first-class architectural pieces instead of parallel experiments.

## 7. Questions for Human Reviewer

1. Should Symphony become the canonical project issue dispatcher for ADF, or should ADF only borrow its state model?
2. Should provider health checks be completely decoupled from `reconcile_tick`, even if that means routing may use slightly stale health data?
3. Should PR gate reconciliation remain in ADF, or become a separate actor/service with its own project capability cache?
4. Should project-source loading produce independent project controllers rather than appending all agents into one `Vec<AgentDefinition>`?
5. Should JSONL envelope support be mandatory for new runtimes such as pi-rust, while legacy opencode/claude subprocesses remain compatibility adapters?
6. Should Gitea comments remain the agent-to-agent coordination surface, or should comments be generated from an internal structured activity/event stream?
7. What is the first migration slice: provider health actor, project controller split, runtime adapter boundary, or activity journal?
8. Should we freeze feature work in `terraphim_orchestrator/src/lib.rs` except urgent fixes until the new boundary is designed?
9. Do we still need one daemon to own all projects, or should each project controller be separately supervised and restartable?
10. What operational SLO matters most for the redesign: dispatch latency, successful PR completion, no duplicate work, or recovery from provider failure?

## Source Evidence

- Local code: `crates/terraphim_orchestrator/src/lib.rs`, `config.rs`, `provider_probe.rs`, `dispatcher.rs`, `scope.rs`, `pr_gate.rs`.
- Local code: `crates/terraphim_symphony/src/orchestrator/mod.rs`, `dispatch.rs`, `reconcile.rs`, `state.rs`.
- Local code: `crates/terraphim_agent_supervisor/src/*`, `crates/terraphim_agent_messaging/src/*`.
- Existing docs: `.docs/research-bigbox-adf-simplification.md`, `.docs/research-adf-stability-roadmap-2026-05-01.md`, `.docs/research-dark-factory-orchestration.md`, `.docs/research-tlaplus-symphony-validation.md`, `.docs/research-inter-agent-orchestration.md`, `.docs/research-adf-fallback-quota-v2.md`, `.docs/adf-architecture.md`.
- Bigbox logs: repeated `reconcile_tick exceeded timeout` at 90s before probe-timeout fix; provider probe timeouts/failures; repeated branch-protection 404/403; stale worktree no-manifest skips; prior invalid `HEAD` worktree creation failures.
- Gitea: issue `terraphim-ai#1769`, PR `terraphim-ai#1782`, issue `terraphim-ai#1785`.
- GitHub: issues `#688` JSONL RPC envelope, `#689` Agent SDK migration, `#641` structured activity log, `#642` file-based approval gates.
