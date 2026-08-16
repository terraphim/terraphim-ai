//! Shared interrupt signaling for all tools.
//!
//! Mirrors Hermes `tools/interrupt.py`: a single process-wide flag that any
//! tool can poll during long-running operations. The agent loop sets/clears
//! the flag; tools check [`is_interrupted`] and bail out early with a
//! graceful `[interrupted]` result instead of spinning.

use std::sync::atomic::{AtomicBool, Ordering};

/// Process-wide interrupt flag. `SeqCst` is overkill for a poll flag but
/// keeps the semantics simple and correct across threads.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Signal or clear the interrupt state. Called by the agent between turns.
pub fn set_interrupt(active: bool) {
    INTERRUPTED.store(active, Ordering::SeqCst);
}

/// Check whether an interrupt has been requested. Cheap and safe to call
/// from any thread or async task.
pub fn is_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_not_interrupted() {
        set_interrupt(false);
        assert!(!is_interrupted());
    }

    #[test]
    fn set_and_clear_roundtrip() {
        set_interrupt(true);
        assert!(is_interrupted());
        set_interrupt(false);
        assert!(!is_interrupted());
    }
}
