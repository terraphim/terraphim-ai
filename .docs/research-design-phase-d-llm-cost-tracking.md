# Research & Design: LLM Cost Tracking -- Phase D CLI Enhancements

## 1. Problem Restatement and Scope

### Problem
Phases A-C delivered the complete data pipeline: LLM calls record usage, pricing is computed, records persist to UsageStore, and providers fetch live data. However, the CLI commands don't leverage this data effectively:

- `usage show` only fetches live provider data (no execution history / spend aggregation)
- `usage history` lists raw executions but has no `--by model` grouping and uses `--since` not `--last Nd`
- `usage alert` only checks agent budgets from `AgentMetricsRecord`, not dollar-based budget thresholds
- `usage show` doesn't display today/week/month spend from persisted ExecutionRecords
- The `--provider` filter on `history` filters by agent_name, not by provider/model

### IN Scope
- Enhance `usage show` to aggregate spend from ExecutionRecords (today/week/month)
- Add `--last Nd` shorthand to `history` (e.g., `--last 7d`)
- Add `--by model` grouping to `history`
- Enhance `usage alert --budget N` to work with dollar budgets
- Fix `history --provider` to filter by provider/model fields

### OUT of Scope
- Web dashboard
- New CLI subcommands
- Changes to provider fetch logic

## 2. System Elements

| Element | Location | Status | Role |
|---|---|---|---|
| `UsageAction` enum | `cli.rs:20-54` | EXISTS | CLI subcommand definitions |
| `execute_show()` | `cli.rs:77-120` | PARTIAL | Only fetches provider data, no ExecutionRecord aggregation |
| `execute_history()` | `cli.rs:122-186` | PARTIAL | Lists raw records, no grouping |
| `execute_alert()` | `cli.rs:241-286` | PARTIAL | Only checks AgentMetricsRecord budgets |
| `UsageStore::query_executions()` | `store.rs:552-639` | WORKING | Returns Vec<ExecutionRecord> filtered by date+agent |
| `ExecutionRecord` | `store.rs:134-222` | WORKING | Has model, provider, cost_sub_cents, started_at |
| `PricingTable` | `pricing.rs` | WORKING | Used for cost calculation |
| `format_usage_text()` | `formatter.rs:4-50` | WORKING | Formats ProviderUsage for display |

## 3. Constraints

| Constraint | Implication |
|---|---|
| No new subcommands | Extend existing `Show`, `History`, `Alert` variants |
| Must work with persistence feature | All new logic behind `#[cfg(feature = "persistence")]` |
| ExecutionRecords stored per-agent | query_executions already supports agent_filter |
| No model/provider filter in query_executions | Must filter in-memory after fetch |

## 4. Design: Step-by-Step

### Step D1: Add `--last Nd` to History

**File**: `cli.rs:28-37` (UsageAction::History)

Change `--since` to optional, add `--last` argument:
```rust
History {
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    until: Option<String>,
    #[arg(long)]
    last: Option<String>,  // e.g., "7d", "30d"
    #[arg(short, long)]
    provider: Option<String>,
    #[arg(short, long)]
    model: Option<String>,
    #[arg(long)]
    by_model: bool,
    #[arg(short, long, default_value = "text")]
    format: String,
},
```

In `execute_history`, resolve `--last 7d` to `since = 7 days ago`. If neither `--since` nor `--last` given, default to `--last 7d`.

### Step D2: Add `--by model` Grouping to History

**File**: `cli.rs` (execute_history)

After fetching executions, add grouping logic:
```rust
if by_model {
    let mut grouped: BTreeMap<String, ModelAggregation> = BTreeMap::new();
    for exec in &executions {
        let key = exec.model.as_deref().unwrap_or("unknown");
        let entry = grouped.entry(key.to_string()).or_default();
        entry.total_tokens += exec.total_tokens;
        entry.total_cost += exec.cost_usd();
        entry.count += 1;
    }
    // Format as table
}
```

### Step D3: Enhance `usage show` with ExecutionRecord Aggregation

**File**: `cli.rs` (execute_show)

After fetching provider data, also query ExecutionRecords for today/week/month:
```rust
let today = Utc::now().format("%Y-%m-%d").to_string();
let week_ago = Utc::now().checked_sub_signed(Duration::days(7))...;
let month_start = /* first of month */;

let today_execs = store.query_executions(&today, None, None).await?;
let week_execs = store.query_executions(&week_ago_str, None, None).await?;
let month_execs = store.query_executions(&month_start_str, None, None).await?;
```

Append spend summary lines to output.

### Step D4: Enhance `usage alert --budget N`

**File**: `cli.rs` (execute_alert)

Currently alert only works with `AgentMetricsRecord.budget_monthly_cents`. Enhance to:
- Accept `--budget 50` (dollar amount)
- Query all ExecutionRecords for current month
- Sum total cost
- Compare against budget
- Display percentage and status

### Step D5: Add `--model` filter to History

Filter executions by model field in-memory after fetching from store.

## 5. File Change Summary

| File | Action | Purpose |
|---|---|---|
| `terraphim_usage/src/cli.rs` | MODIFY | Add --last, --by-model, --model, enhance show/alert |
| `terraphim_usage/src/cli.rs` | MODIFY | Add ModelAggregation struct |
| `terraphim_cli/src/main.rs` | MODIFY | Update UsageAction enum if needed |

## 6. Implementation Order

1. D1: --last Nd parsing (independent)
2. D2: --by model grouping (depends on D1)
3. D5: --model filter (independent)
4. D3: show enhancement with spend aggregation (independent)
5. D4: alert --budget N (independent)
6. Tests + clippy
