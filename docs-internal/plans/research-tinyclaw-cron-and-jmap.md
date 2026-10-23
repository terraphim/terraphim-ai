# Research: tinyclaw cron surface (#3147) + jmap crate relocation (#3198)

Date: 2026-08-12. Discipline: `disciplined-research`.

## #3147 — cron scheduling surface for TinyClaw

### Existing state (already built in Wave 3, epic #3160)

`crates/terraphim_tinyclaw/src/cron/` is a **complete cron subsystem**:

- `job.rs` — `CronJob` (id, name, prompt, `Schedule`, skills, deliver, repeat,
  state, enabled, next/last run, model/provider overrides, script), `Schedule`
  enum with `parse()` accepting 4 formats: `every <dur>`, RFC3339 timestamp,
  5/6-field cron (padded with leading `0` seconds for the `cron` crate),
  relative delays (`30m`, `2h`, `1d`). Invalid cron → `CronError::InvalidSchedule`.
- `store.rs` — `CronStore` over `terraphim_persistence::DeviceStorage`
  (`fastest_op`, opendal), same pattern as the subagent registry.
- `scheduler.rs` — `CronScheduler` with `JobExecutor` trait, `tick()`, pause,
  repeat exhaustion. `CronError` enum.

The **dashboard** (`src/dashboard/`) already exposes the Hermes cron REST
contract: `POST /api/cron/fire`, `GET/POST /api/cron/jobs`,
`GET/DELETE /api/cron/jobs/{id}` — ported from `web_server.py` + `cron/jobs.py`.
`DashboardState` owns a `CronStore` (memory-only for hermetic tests).

### What #3147 actually needs (delta)

The issue text predates the Wave-3 cron subsystem and assumes nothing exists.
What's genuinely missing:

1. **CLI**: `terraphim-tinyclaw schedule create/list/delete` — no `Schedule`
   subcommand in `Commands` (main.rs) today.
2. **Agent-loop tool**: `ScheduleTool` (`src/tools/scheduler.rs`) so the model
   can create recurring schedules in conversation.
3. **`SkillStep::Schedule`** variant (`src/skills/types.rs`) + executor dispatch.

### Orchestrator coupling — decision

The issue says "validate with `terraphim_orchestrator::is_cron_schedule_valid`"
and "persist via `terraphim_orchestrator` cron mechanism". Findings:

- `terraphim_orchestrator` is **excluded from the workspace** (`Cargo.toml`
  exclude list, line ~30) and consumed from the `terraphim` registry. The local
  `crates/terraphim_orchestrator/` dir is a residual (no workspace root;
  `cargo check` there fails "failed to find a workspace root"). It also has
  **two versions in Cargo.lock** (1.20.2 terraphim registry + 1.20.3 crates.io)
  — `cargo check -p terraphim_orchestrator` is ambiguous. Adding it as a
  tinyclaw dep is a heavyweight, ambiguous dependency for one function.
- `is_cron_schedule_valid` wraps `parse_cron` (cron-crate `Schedule` parse,
  5/6/7-field). TinyClaw's own `Schedule::parse` already validates cron via the
  same `cron = "0.13"` crate and is **strictly more useful** (4 formats).
- The orchestrator **process is down** on bigbox; its cron persistence is a
  config file / in-memory `last_cron_fire` — nothing durable TinyClaw could
  push to right now.

**Decision**: implement the surface over TinyClaw's own `CronStore`
(`terraphim_persistence`-backed, same storage as dashboard), validate with
`Schedule::parse`, and document the deviation. The dashboard REST surface
(`/api/cron/jobs`) is the persistence/management API; CLI + tool + skill step
are clients of the same store. This matches the established tinyclaw pattern
(dashboard cron was already built this way) and needs **zero new deps**.

## #3198 — jmap crate relocation

### The problem

`crates/terraphim_tinyclaw/Cargo.toml` line ~116:
`jmap_client = { path = "../../../terraphim-private/crates/haystack_jmap" }`
— the **only** path dep escaping the workspace into `terraphim-private`.
Breaks fresh clones / CI / Docker (private repo not present).

### Findings

- **The separate crates repository already exists**: `terraphim-service`
  (Gitea + github) — "Service + middleware + haystack layer — extracted from
  terraphim". It contains `crates/haystack_jmap` AND `crates/haystack_core`,
  both workspace members.
- **Both are already published**:
  - terraphim registry: `haystack_jmap 1.20.2`, `haystack_core 1.20.3`
    (verified via Gitea packages API + local sparse-index cache).
  - crates.io: `haystack_jmap 1.20.4`, `haystack_core 1.20.3`.
- The terraphim-registry `haystack_jmap 1.20.2` manifest resolves
  `haystack_core ^1.19.3` + `terraphim_types` from the **same registry**
  (sparse index cache dump verified — `registry: None` entries, i.e. same
  registry), rest from crates.io.
- **API drift** between the private copy (lib `jmap_client`, v1.0.0) and the
  crates-repo line (lib `haystack_jmap`):
  - `JMAPClient::new(access_token)` (private) vs `new(access_token, session_url)`
  - `search_emails(query)` vs `search_emails(query, limit)`
  - structs `Email`/`EmailAddress`/`BodyValue`/`BodyPart` are **identical**;
    service adds `email_to_document()`.
  - lib/package rename: `jmap_client` → `haystack_jmap`.
- tinyclaw usage (src/channels/email.rs): `Email`, `JMAPClient` imports;
  `EmailConfig { jmap_access_token, smtp_host, from_address, allow_from }`;
  `connect()` calls `JMAPClient::new(token)`; `search_emails(query)`;
  re-exports `BodyValue`, `Email as JmapEmail`, `EmailAddress as JmapEmailAddress`;
  tests construct `Email`/`EmailAddress`/`BodyValue` literals directly
  (field-identical, so they survive the rename).
- terraphim-private consumers: only `haystack_jmap` itself references it
  (grep of crates/*/Cargo.toml). `haystack_core` is used by atlassian,
  discourse, grepapp, jmap in private — so **haystack_core stays** in private
  (it's published separately; private's haystack_atlassian/discourse still
  need the path dep). Only `haystack_jmap` moves out.

### Plan

1. tinyclaw Cargo.toml: replace the private path dep with
   `haystack_jmap = { version = "1.20.2", registry = "terraphim" }`
   (registry-cached; verified resolvable). Remove the private-path comment.
2. Update `src/channels/email.rs`:
   - `use haystack_jmap::{Email, JMAPClient}` + re-exports from `haystack_jmap`.
   - `EmailConfig` gains `jmap_session_url: String` (default
     `"https://api.fastmail.com/jmap/session"`? — must be configurable; default
     empty is fine, connect errors on empty).
   - `connect()`: `JMAPClient::new(token, &session_url)`.
   - `search_emails(query)`: add limit — keep the channel method signature
     `(query)` but call `client.search_emails(query, 20)` (bounded default);
     or thread a limit through. Keep simple: constant `SEARCH_LIMIT: u32 = 20`.
3. Remove `crates/haystack_jmap` from terraphim-private (git rm + commit +
   push) — the "move". `haystack_core` stays (private haystack consumers).
4. Verify: `cargo check -p terraphim_tinyclaw` + full test suite; the
   email.rs tests build struct literals — should compile unchanged.
5. Update the Cargo.toml comment block (lines ~108-116) to point at the
   terraphim registry instead of the private path.
6. Close #3198 with verification comment.

### Risks

- `jmap_session_url` is a new required-ish config field; keep `Default` empty
  so existing configs still parse, and `connect()` returns a clear error if
  empty (channel is not enabled by default).
- Version pin: `1.20.2` is the terraphim-registry version; crates.io has
  1.20.4 but the terraphim registry is the internal source of truth used by
  all other workspace crates (terraphim_types = registry terraphim). Stick
  with registry 1.20.2; bump later if the service repo publishes 1.20.4
  there.

## Open items for #3147 implementation

- `ScheduleTool` ops: `create {prompt, schedule, skills?, deliver?, model?}`,
  `list`, `delete {id}` — mirrors dashboard cron CRUD. Store key:
  `"tinyclaw_schedules"` (dashboard uses `"dashboard_cron_jobs"` — separate
  keyspace, same store type).
- CLI: `Commands::Schedule { command: ScheduleCommands }` with
  `Create {prompt, schedule, skill?, deliver?}`, `List`, `Delete {id}`.
- SkillStep::Schedule `{cron, skill, inputs}` — executor needs a
  `CronStore`; currently `SkillExecutor` has only storage_dir + tool_registry.
  Add optional `cron_store: Option<CronStore>` + `with_cron_store()` builder;
  `execute` returns a clear error when unset ("scheduler not configured").
  Executor is constructed in main.rs / agent_loop — wire the store there.
