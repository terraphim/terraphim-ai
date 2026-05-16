# Design & Implementation Plan: Synthetic Time Testing for ADF Cron Scheduler

## 1. Summary of Target Behavior

Enable synthetic time testing for the ADF orchestrator's cron scheduling logic to verify that agents fire exactly once per schedule occurrence and do not re-trigger rapidly within the same schedule window.

After implementation:
- Tests can manipulate `last_tick_time` directly to simulate tick boundaries
- Tests can verify `last_cron_fire` correctly prevents re-triggering
- The cron scheduling fix can be validated without waiting hours for real-time verification

## 2. Key Invariants and Acceptance Criteria

### Invariants
1. `last_cron_fire` is set AFTER agent spawn, never before
2. `last_tick_time` represents the tick boundary used for cron comparison
3. `schedule.after(&last_tick_time).next()` returns the next fire time at or after last_tick
4. An agent is skipped if `next_fire <= last_fire_time` for that agent

### Acceptance Criteria

| ID | Criterion | Test Type | Test Location |
|----|-----------|-----------|---------------|
| AC1 | Agent fires when `last_tick_time` is just before schedule fire time and agent is inactive | Unit | scheduler_tests.rs |
| AC2 | Agent does NOT fire again within same schedule occurrence (re-trigger prevention) | Unit | scheduler_tests.rs |
| AC3 | Agent fires again at next schedule occurrence after `last_tick_time` advances past previous fire | Unit | scheduler_tests.rs |
| AC4 | Agent is skipped if already in `active_agents` | Unit | scheduler_tests.rs |
| AC5 | `set_last_tick_time` helper correctly updates internal state | Unit | scheduler_tests.rs |

## 3. High-Level Design and Boundaries

### Approach: Direct Field Manipulation (Test-Only Helpers)

```
┌─────────────────────────────────────────────────────────────┐
│                    Test Context Only                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ TestOrchestrator (wraps AgentOrchestrator)           │   │
│  │  - set_last_tick_time(time)                           │   │
│  │  - set_last_cron_fire(agent, time)                    │   │
│  │  - check_cron_schedules() -> Vec<AgentDefinition>     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Uses existing
                              ▼
┌─────────────────────────────────────────────────────────────┐
│               Production: AgentOrchestrator                 │
│  - last_tick_time: DateTime<Utc>                           │
│  - last_cron_fire: HashMap<String, DateTime<Utc>>          │
│  - check_cron_schedules()                                  │
└─────────────────────────────────────────────────────────────┘
```

### Boundaries
- **Changes inside existing components**: Add `#[cfg(test)]` helper method only
- **No new components**: Use existing test infrastructure
- **No production code changes**: All synthetic time handling is test-only

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | No `set_last_tick_time` helper | Add `#[cfg(test)] set_last_tick_time(&mut self, time: DateTime<Utc>)` | chrono |
| `crates/terraphim_orchestrator/tests/scheduler_tests.rs` | Modify | 3 scheduler tests | Add 4-5 cron scheduling tests using synthetic time | TestModTimeScheduler |

### State Changes

**`src/lib.rs` - AgentOrchestrator:**
- Add line ~7625: `#[cfg(test)] pub fn set_last_tick_time(&mut self, time: chrono::DateTime<chrono::Utc>)`
- Sets `self.last_tick_time` for test control

**`tests/scheduler_tests.rs`:**
- Create `TestModTimeScheduler` struct wrapping `TimeScheduler`
- Add helper methods for time manipulation
- Add test cases for AC1-AC5

## 5. Step-by-Step Implementation Sequence

### Step 1: Add test helper `set_last_tick_time`
**Purpose**: Allow tests to control `last_tick_time` field
**Deployable State**: Yes - purely additive, #[cfg(test)]

```rust
#[cfg(test)]
/// Test helper: set last_tick_time for synthetic time testing.
pub fn set_last_tick_time(&mut self, time: chrono::DateTime<chrono::Utc>) {
    self.last_tick_time = time;
}
```

### Step 2: Create TestModTimeScheduler wrapper
**Purpose**: Provide controlled time environment for scheduler tests
**Deployable State**: Yes - new test infrastructure

```rust
struct TestModTimeScheduler {
    scheduler: TimeScheduler,
    last_tick_time: chrono::DateTime<chrono::Utc>,
}

impl TestModTimeScheduler {
    fn new(agents: &[AgentDefinition], last_tick_time: chrono::DateTime<chrono::Utc>) -> Self { ... }
    fn set_tick_time(&mut self, time: chrono::DateTime<chrono::Utc>) { ... }
    fn get_scheduled_fire_time(&self, agent_name: &str) -> Option<DateTime<Utc>> { ... }
}
```

### Step 3: Add AC1 - Agent fires when time is right
**Purpose**: Verify basic cron firing works
**Deployable State**: Yes - new test

```rust
#[test]
fn test_cron_fires_when_time_matches_schedule() {
    // Set last_tick_time to T-31s where fire is at T
    // Call check logic
    // Assert agent is in to_spawn
}
```

### Step 4: Add AC2 - Re-trigger prevention
**Purpose**: Verify `last_cron_fire` prevents re-trigger
**Deployable State**: Yes - core fix verification

```rust
#[test]
fn test_cron_no_retrigger_same_occurrence() {
    // First call: agent fires, last_cron_fire is set
    // Second call with same time window: agent should NOT be in to_spawn
}
```

### Step 5: Add AC3 - Fires at next occurrence
**Purpose**: Verify agent can fire again at next schedule
**Deployable State**: Yes - regression prevention

```rust
#[test]
fn test_cron_fires_next_occurrence() {
    // First occurrence fires, last_cron_fire = T1
    // Advance last_tick_time past T1 (to T2 where next fire occurs)
    // Call check: agent should fire again
}
```

### Step 6: Add AC4 - Skip active agents
**Purpose**: Verify existing active agent check still works
**Deployable State**: Yes - existing behavior regression test

### Step 7: Add AC5 - Helper method validation
**Purpose**: Ensure test helper correctly updates state
**Deployable State**: Yes - infrastructure validation

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location | Test Name |
|---------------------|-----------|--------------|-----------|
| AC1 | Unit | scheduler_tests.rs | `test_cron_fires_when_time_matches_schedule` |
| AC2 | Unit | scheduler_tests.rs | `test_cron_no_retrigger_same_occurrence` |
| AC3 | Unit | scheduler_tests.rs | `test_cron_fires_next_occurrence` |
| AC4 | Unit | scheduler_tests.rs | `test_cron_skips_active_agents` |
| AC5 | Unit | scheduler_tests.rs | `test_set_last_tick_time_helper` |

### Test Execution
```bash
cargo test -p terraphim_orchestrator scheduler_tests
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Test doesn't reflect production | Use actual cron expressions from terraphim.toml | Low - expressions are real |
| last_tick_time timing semantics unclear | Add detailed comments | Low - code review |
| Async spawn complications | Test only checks to_spawn, not actual spawn | Low - isolated test |

### Complexity Assessment
- **Cyclomatic complexity**: Low - each test is straightforward sequence
- **Coupling**: Low - uses existing scheduler infrastructure
- **Lines of change**: ~100-150 total (2-3 helpers + 5 tests)

## 8. Open Questions / Decisions for Human Review

1. **Test cron expressions**: Use production schedules (`30 0-10 * * *`) or simplified test schedules (`0 * * * *`)?

2. **Integration with existing scheduler_tests.rs**: Add to existing file or create new `cron_schedule_tests.rs`?

3. **Test spawn vs. to_spawn**: Should tests verify the actual `to_spawn` list (current approach) or mock full spawn?

4. **Compound schedule testing**: Should the compound review schedule (`0 2 * * *`) also have synthetic time tests?

5. **Edge case coverage**: Should we test what happens if `last_tick_time` is set to exactly the fire time?

## Implementation Notes

### Location of set_last_tick_time in lib.rs
After existing test helpers (~line 7620):
```rust
/// Test helper: set last_tick_time for synthetic time testing.
#[cfg(test)]
pub fn set_last_tick_time(&mut self, time: chrono::DateTime<chrono::Utc>) {
    self.last_tick_time = time;
}
```

### Test Data
Use actual schedules from `/opt/ai-dark-factory/conf.d/terraphim.toml`:
- spec-validator: `30 0-10 * * *` (fires at XX:30)
- test-guardian: `35 0-10 * * *` (fires at XX:35)
- implementation-swarm: `45 0-10 * * *` (fires at XX:45)
