# Research Document: Synthetic Time Testing for ADF Cron Scheduler

## 1. Problem Restatement and Scope

### Problem Statement
The ADF orchestrator's cron scheduler was re-triggering agents excessively (130+ times in 12 hours instead of ~11 expected). A fix was implemented that tracks `last_cron_fire` per agent to prevent re-triggering within the same schedule window. This fix requires verification via synthetic time testing, as waiting for real-time would take hours.

### IN Scope
- Understanding how `check_cron_schedules` works with `last_cron_fire`
- Identifying how to create synthetic time tests for the cron scheduling logic
- Designing test helpers to manipulate time fields for testing
- Verifying the fix prevents rapid re-triggering when agent completes quickly

### OUT of Scope
- Modifying the cron crate or schedule parsing logic
- Changing production time handling (chrono::Utc::now usage)
- Adding full TimeProvider abstraction (too invasive)
- Integration testing with real agent spawning

## 2. User & Business Outcomes

### Visible Behavior After Fix
- Agents with cron schedules fire exactly once per schedule occurrence
- No rapid re-triggering when agents complete within tick interval
- Expected behavior: `50 0-10 * * *` fires once per hour (11 times/day), not every 90 seconds

### Testable Outcomes
1. When `check_cron_schedules` is called at T+30s and agent is inactive, agent spawns once
2. When same call happens again at T+60s, agent does NOT spawn again (protected by `last_cron_fire`)
3. When `check_cron_schedules` is called after next schedule occurrence (T+3600s), agent spawns again

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| `AgentOrchestrator` | lib.rs:202-280 | Main orchestrator holding time fields | scheduler, spawner, config |
| `last_tick_time` | lib.rs:219 | `chrono::DateTime<Utc>` - timestamp of last tick | Set at end of each tick |
| `last_cron_fire` | lib.rs:240 | `HashMap<String, DateTime<Utc>>` - per-agent last fire tracking | Set after each spawn |
| `check_cron_schedules()` | lib.rs:7038 | Async method checking and spawning due agents | Uses scheduler, active_agents, last_tick_time, now |
| `TimeScheduler` | scheduler.rs:32 | Parses cron expressions, provides scheduled agents | cron crate |
| `cron::Schedule` | cron crate | Generates fire time iterators via `after()` |chrono DateTime |
| `tick_interval_secs` | config.rs:107 | Tick interval (default 30s) | Reconciliation loop |
| Existing test helpers | lib.rs:7615 | `set_last_cron_fire()`, `set_last_run_commit()` | Test-only |

### Key Method Signature
```rust
// lib.rs:7039
async fn check_cron_schedules(&mut self)
// Uses:
//   now = chrono::Utc::now()
//   last_tick_time: chrono::DateTime<chrono::Utc>
//   last_cron_fire: HashMap<String, chrono::DateTime<chrono::Utc>>
//   scheduler.scheduled_agents() -> Vec<(&AgentDefinition, &Schedule)>
//   Schedule::after(&DateTime) -> Iterator
```

## 4. Constraints and Their Implications

### Constraint: Direct chrono::Utc::now() Call
- **Why it matters**: Cannot mock time without abstraction
- **Implication**: Test must manipulate `last_tick_time` field directly, not inject time

### Constraint: cron crate Iterator Returns chrono::DateTime
- **Why it matters**: The `schedule.after(&last_tick_time)` returns concrete chrono types
- **Implication**: Test's synthetic time must be compatible with chrono::DateTime comparison

### Constraint: No Existing TimeProvider Abstraction
- **Why it matters**: Adding trait would require invasive changes throughout codebase
- **Implication**: Must work with direct field manipulation within test context only

### Constraint: check_cron_schedules is async
- **Why it matters**: Spawning involves async operations
- **Implication**: Unit test may need to mock spawn or use test helpers

### Constraint: Must Not Break Production
- **Why it matters**: This is a test-only enhancement
- **Implication**: All changes must be #[cfg(test)] or test-only helpers

## 5. Risks, Unknowns, and Assumptions

### ASSUMPTIONS
1. Manipulating `last_tick_time` directly in tests will correctly simulate tick boundaries
2. The `last_cron_fire` field correctly stores the fire time as chrono::DateTime
3. Comparing `next_fire <= last_fire` correctly prevents re-trigger within same schedule occurrence
4. Agent completion within tick interval is the actual re-trigger cause (logs confirm this)

### RISKS
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test doesn't match production behavior | Low | High | Use actual cron schedule values from config |
| last_tick_time semantics unclear | Medium | Medium | Add comments explaining tick cycle timing |
| Spawn mock complexity | Medium | Low | Use existing test infrastructure patterns |

### UNKNOWNS
1. Whether compound review schedule needs same synthetic time testing
2. If there are other time-sensitive methods needing similar treatment

## 6. Complexity vs. Simplicity Opportunities

### Complexity Sources
1. Direct `chrono::Utc::now()` calls embedded in production code
2. Async spawn_agent() makes testing harder
3. Multiple time fields interacting (last_tick_time, last_cron_fire)

### Simplification Strategy
**Chosen Approach: Direct Field Manipulation**
- Add `#[cfg(test)]` helper `set_last_tick_time(&mut self, time: DateTime<Utc>)`
- Test controls time by setting `last_tick_time` to just before fire time
- Test calls `check_cron_schedules()` directly
- Use `set_last_cron_fire()` already exists

**Why This is Simpler:**
- No architectural changes to production code
- Localized to test helpers
- Follows existing pattern (set_last_cron_fire already exists)

### Alternative Considered and Rejected
**TimeProvider Trait**: Would require injecting time abstraction everywhere, too invasive for test verification

## 7. Questions for Human Reviewer

1. **Should compound review schedule also have synthetic time tests?** It has similar scheduling logic but is separate from agent scheduling.

2. **Is direct field manipulation acceptable?** This means tests directly modify internal state rather than using interface-based injection.

3. **Should we also test edge cases like:**
   - Agent becomes active between checks (should still skip if already fired)
   - Multiple schedule occurrences within single tick (rare but possible)

4. **Where should the test file be located?**
   - `tests/scheduler_tests.rs` (existing scheduler tests)
   - `tests/cron_schedule_tests.rs` (new dedicated file)
   - `src/lib.rs` in `#[cfg(test)]` module

5. **Should test verify actual spawn happens or just that to_spawn contains the agent?**
   - Current pattern uses event channel inspection
   - Full spawn test would require mocking AgentSpawner

6. **What cron expressions should be used in tests?**
   - Use actual schedules from production (`30 0-10 * * *`)
   - Or create test-specific simpler schedules (`0 * * * *` - hourly)

7. **Should we test the re-trigger prevention specifically?**
   - Call check_cron_schedules twice in same tick
   - Verify agent only in to_spawn once

8. **Should last_cron_fire be cleared between tests?**
   - Each test should be independent
   - Consider adding `clear_last_cron_fire()` helper

9. **Is 30-second tick interval sufficient for test timing?**
   - Tests will use arbitrary times not aligned to 30s boundaries
   - But last_tick_time is just a DateTime, not tied to actual tick

10. **Should we add integration test that verifies no re-trigger in running system?**
    - Would require monitoring logs over time
    - More validation than unit test
