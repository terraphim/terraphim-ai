# Design: tinyclaw cron surface (#3147) + jmap relocation (#3198)

Date: 2026-08-12. Discipline: `disciplined-design`. Research: `research-tinyclaw-cron-and-jmap.md`.

## Design goals

1. **#3198**: eliminate the only workspace-escaping path dep; consume
   `haystack_jmap` from the terraphim registry; move the crate out of
   terraphim-private. Zero behaviour change to the email channel (only the
   constructor signature + search limit).
2. **#3147**: give users (human CLI + agent loop + skills) a scheduling
   surface over the existing `CronStore`, without new dependencies and
   without coupling to the down orchestrator.

## #3198 — jmap relocation

### Cargo.toml (tinyclaw)

```toml
# BEFORE
jmap_client = { path = "../../../terraphim-private/crates/haystack_jmap" }

# AFTER (replace the comment block + line)
haystack_jmap = { version = "1.20.2", registry = "terraphim" }
```

### email.rs changes

```rust
use haystack_jmap::{Email, JMAPClient};        // was use jmap_client::…
pub use haystack_jmap::{BodyValue, Email as JmapEmail, EmailAddress as JmapEmailAddress};

pub struct EmailConfig {
    pub jmap_access_token: String,
    pub jmap_session_url: String,   // NEW; empty => connect() errors clearly
    pub smtp_host: String,
    pub from_address: String,
    pub allow_from: Vec<String>,
}

// connect()
let client = JMAPClient::new(
    self.config.jmap_access_token.clone(),
    &self.config.jmap_session_url,
).await?;
if self.config.jmap_session_url.is_empty() {
    anyhow::bail!("jmap_session_url is required to connect");
}

// search_emails — bounded
const SEARCH_LIMIT: u32 = 20;
Some(client) => Ok(client.search_emails(query, SEARCH_LIMIT).await?),
```

Tests: struct literals unchanged (field-identical). Add one test for
`connect()` failing on empty session URL (no network).

### terraphim-private

`git rm -r crates/haystack_jmap` + commit + push. `haystack_core` stays
(consumed by atlassian/discourse/grepapp in private).

## #3147 — scheduling surface

### Components

1. **`ScheduleTool`** (`src/tools/scheduler.rs`, registered as `"schedule"`):

   - `create {prompt, schedule, skills?, deliver?, model?}` →
     `Schedule::parse` (rejects invalid cron with clear message) → build
     `CronJob::new(prompt, schedule)` + optional fields → store. Returns
     `{op, id, schedule, status: "created"}`.
   - `list` → `CronStore::load_all` → `{op, count, jobs:[…]}`.
   - `delete {id}` → remove by id → `{op, id, status: "deleted"}`.
   - Store key `"tinyclaw_schedules"`; `DeviceStorage::arc_instance()` at
     construction (`with_storage(storage, key)` builder for tests;
     `from_config` uses `arc_instance()` with graceful degradation).
   - Registered in `create_default_registry_with_parity` when
     `[scheduler] enabled = true` (config section `SchedulerConfig { enabled,
     store_key }`).

2. **CLI** (`main.rs`):

   ```rust
   enum Commands { … Schedule { command: ScheduleCommands } }
   enum ScheduleCommands {
       Create { prompt: String, schedule: String,
                #[arg(long)] skill: Option<String>,
                #[arg(long)] deliver: Option<String> },
       List,
       Delete { id: String },
   }
   ```

   Handlers build `CronStore` from `DeviceStorage::arc_instance()` (await),
   reuse the same logic as `ScheduleTool` (shared helper functions so CLI and
   tool can't drift: `create_job(store, prompt, schedule, …)`,
   `list_jobs(store)`, `delete_job(store, id)` in `src/tools/scheduler.rs`).

3. **`SkillStep::Schedule`** (`src/skills/types.rs`):

   ```rust
   #[serde(rename = "schedule")]
   Schedule { cron: String, skill: String, inputs: serde_json::Value },
   ```

   Executor (`src/skills/executor.rs`):
   - new field `cron_store: Option<CronStore>` + `with_cron_store()` builder.
   - dispatch: build `CronJob` whose `prompt` is
     `format!("run skill {skill} with inputs {inputs}")` (or better: store
     `skill` + `inputs` in the job's `skills` vec + prompt text), schedule
     from `Schedule::parse(cron)`, persist. Error if store unset.
   - `step_type_name` + tests updated.

### Wiring

- `main.rs` agent/gateway path: construct `CronStore` once (like dashboard
  does) and pass to both `ScheduleTool` (via registry factory) and
  `SkillExecutor::with_cron_store`.
- `create_default_registry` / `_with_parity` signatures: `ScheduleTool` needs
  the storage — add `storage: Option<Arc<DeviceStorage>>` param to the
  parity registry factory (or construct from config inside; keep the factory
  async already). Decision: `create_default_registry_with_parity` gains
  `storage: Option<Arc<DeviceStorage>>`; `from_config` path calls
  `arc_instance()` itself. Minimal churn: current callers pass `None`.

### Test plan

- `tests/scheduler_contracts.rs`: create (valid cron + invalid cron rejected),
  list round-trip, delete, persistence across store re-creation
  (memory-only DeviceStorage).
- Executor: `SkillStep::Schedule` with store → job appears in store;
  without store → clear error.
- CLI: covered via the shared helpers (no subprocess tests; keep CLI thin).

## Acceptance mapping

| #3147 criterion | How met |
|---|---|
| "schedule `daily-report` skill every day at 09:00" → schedule ID | `schedule create --prompt … --schedule "0 9 * * *"` returns `{id}` |
| `schedule list` shows active schedules | `ScheduleTool list` + CLI |
| `schedule delete <id>` removes | `delete` |
| Invalid cron rejected with clear message | `Schedule::parse` error |
| Integration test create/list/delete | `scheduler_contracts.rs` |
| Orchestrator not running: fails fast | n/a — we persist locally (documented deviation) |
| Duplicate name / timezone | cron is UTC; no names in v1 (id-based), documented |

| #3198 criterion | How met |
|---|---|
| No workspace-escaping path dep | `haystack_jmap` from terraphim registry |
| Fresh clone builds | registry dep; terraphim-private not needed |
| Crate removed from private | `git rm crates/haystack_jmap` in terraphim-private |
