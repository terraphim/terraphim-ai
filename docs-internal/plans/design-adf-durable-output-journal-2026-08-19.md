# Design: authoritative `terraphim_spawner` output journal

**Issue:** terraphim/terraphim-ai#3269
**Coordination:** terraphim/agent-tasks#99
**Approved parent plan:** `/home/alex/clawd/.hermes/plans/2026-08-18_214843-adf-fleet-convergence-fix.md`, SHA-256 `3241db9d2321dcc22f2766619c876e99927ecb3ac34bbe9aaa02f003ab2e3b97`

## Evidence and problem

The current spawner output is a bounded broadcast source. The orchestrator drains it only on a serialized reconciliation cadence. On 2026-08-19, terraphim-kg-agents#7 spawned reviewer, validator, and verifier normally, but terminal gate parsing failed while telemetry skipped 1,337, 1,129, and 1,332 events respectively. A best-effort ring therefore cannot be the source of truth for factory results.

## Boundary

This PR changes the owning `terraphim_spawner` crate only. It establishes durable pre-broadcast output and recovery APIs. It does not modify orchestrator scheduling, Gitea statuses, deployed configuration, credentials, or publish a release.

## Record and framing contract

```rust
struct DurableOutputRecord {
    run_id: Uuid,
    sequence: u64,
    process_id: ProcessId,
    kind: OutputKind,
    redacted_payload: String,
    completion_id: Option<Uuid>,
    observed_at: DateTime<Utc>,
}
```

The implementation may adapt exact public types to existing crate conventions, but must preserve these semantics:

1. Assign contiguous sequence numbers per stable run before any broadcast send.
2. Serialize every record into a length-delimited versioned frame with independently verifiable header metadata and payload integrity checksums. Recovery must validate the declared length before trusting it to allocate or read payload storage.
3. Append the frame to one journal per run, opened mode `0600`, before fan-out.
4. Flush each record to the OS. Call `sync_data` for completion and before exposing a terminal child result.
5. Treat journal create/write/flush/sync failures as explicit factory faults. Never silently fall back to broadcast-only behavior.
6. Keep the existing bounded broadcast API as observation-only compatibility fan-out where possible.
7. Redact before persistence. Durable storage must never capture a less-redacted payload than the existing output event.
8. Hold a kernel-enforced, nonblocking exclusive writer lock for the lifetime of an incomplete run. Successful completion releases it only after the terminal `sync_data`; failure keeps the poisoned writer owned until deterministic close/drop. Fallback hand-off must drain capture, close, and join the prior writer before reopening the same run; concurrent appenders fail closed with a typed busy error.
9. Make finalization cancellation-safe. Cancelling a wait or drain future must retain every capture-task handle so a retry cannot seal the journal ahead of detached output. Successful terminal methods join the blocking writer task before exposing finalized status.
10. Create and open Unix roots and journal leaves through an anchored `openat`/`mkdirat` directory-descriptor walk with `O_NOFOLLOW`; pathname metadata is diagnostic only after traversal begins. Sync each created directory and its parent, and sync the containing directory after journal leaf creation. Durable-journal entry points fail explicitly on unsupported non-Unix platforms rather than retaining a weaker path-based implementation.
11. A failed primary fallback attempt drains without sealing. If fallback spawn itself fails, reopen and seal the shared logical run with the primary's actual terminal identity/status before returning the spawn failure; sealing failure supersedes it and fails closed.

## Recovery and exactly-once contract

- Reader validates format version, frame length, checksum, run ID, and contiguous sequence.
- Only a torn final frame may be durably truncated and synced before recovery returns; interior corruption or a sequence gap is a blocking fault.
- Recovery enforces the writer state machine: data precedes exactly one completion record, `Complete` seals that identity, and `Ack` follows `Complete`; no data or terminal marker may reopen a sealed state.
- Recovery is bounded by explicit byte and record limits. Limit exhaustion is a typed blocking fault, never an unbounded allocation or silent partial replay.
- Resume from a caller-owned atomic checkpoint `(run_id, last_sequence, completion_id)` without skipping records.
- Completion records use stable `completion_id`. Replaying the same ID and identical payload is suppressible; the same ID with differing payload is corruption.
- Incomplete or unacknowledged runs are never removed by startup/daily retention.
- Expose explicit acknowledgement state/API so a later consumer can mark terminal telemetry and Gitea output durable before seven-day compression/retention and eventual GC.
- Recovery and retention operations must remain bounded by configured root/run paths and reject traversal/symlink escapes atomically. Path-based check-then-open validation is insufficient where an attacker can swap an ancestor.

## Compatibility

- Existing consumers that only call `OutputCapture::subscribe` or `AgentHandle::subscribe_output` continue to compile. The raw `OutputCapture::broadcaster` sender accessor is deliberately removed because it allowed unjournaled injection into the observation channel; callers must use the receiver-only subscription APIs.
- New durable mode must be explicit and testable. If the current `OutputStream` constructor lacks a journal root/run identity, prefer an additive constructor/config over ambient globals.
- The final PR must identify any semver impact and the exact consumer upgrade seam.

## TDD sequence

1. RED: stalled subscriber plus >1,000 output records; prove current broadcast can lag and the new durable assertion fails before implementation.
2. GREEN: journal-first sequence remains contiguous for every record despite broadcast lag.
3. Crash/torn-tail tests at frame header, payload, checksum, flush, sync, and terminal boundaries, including an interior length-field mutation that must not be classified as a torn tail.
4. Replay/checkpoint tests: no skips, stable completion ID, identical duplicate suppression, conflicting duplicate fault.
5. Security tests: mode 0600, redacted content only, atomic unsafe path/symlink rejection, concurrent-writer refusal, and containing-directory durability.
6. Retention tests: incomplete/unacknowledged preserved; acknowledged terminal run eligible only after retention period.
7. Compatibility tests for the existing broadcast subscriber API.

## Verification

- `cargo fmt --all -- --check`
- focused `terraphim_spawner` tests, including >1,000-event burst and crash recovery;
- clippy for the touched crate/all targets with warnings denied;
- existing crate tests;
- independent different-model structural review before commit/PR.

## Acceptance

Broadcast lag or orchestrator restart loses no durable record or completion. Journal corruption fails closed. No PASS-capable terminal result can be produced after journal failure. The exact API and semver/release requirement for terraphim-agents#134 are documented.