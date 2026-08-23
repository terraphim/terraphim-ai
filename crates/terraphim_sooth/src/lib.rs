//! `terraphim_sooth` -- a deterministic Rust port of the [Sooth] predictor.
//!
//! Sooth is the small Markov-chain symbol predictor at the heart of
//! [MegaHAL]: given a `(u32, u32)` context (two preceding symbol ids), it
//! tracks how often each following symbol has been observed and can then
//! answer "what usually comes next", "how surprising was this symbol" and
//! "how uncertain is this context".
//!
//! Design constraints (see terraphim/terraphim-ai#3260):
//!
//! - **Deterministic iteration.** All state is kept in [`BTreeMap`], never
//!   [`std::collections::HashMap`], so iteration order (and therefore
//!   [`Predictor::select`]) is reproducible given the same observations and
//!   the same RNG stream.
//! - **No OS entropy.** The crate never calls `thread_rng()`/`OsRng`
//!   directly -- callers inject an [`rand_core::Rng`] implementation
//!   (see [`DefaultRng`] for a ready-made seedable PCG generator). This
//!   keeps the crate usable on `wasm32-unknown-unknown`, where OS entropy
//!   is not available without extra glue.
//! - **Own serialisation format.** [`Predictor`] derives [`serde::Serialize`]
//!   / [`serde::Deserialize`] directly; importing Ruby `Marshal` dumps is
//!   explicitly out of scope for this port.
//!
//! [Sooth]: https://github.com/jasonhutchens/sooth
//! [MegaHAL]: https://megahal.sourceforge.net/

use std::collections::BTreeMap;

use rand_core::Rng;
use serde::{Deserialize, Serialize};

/// A default, seedable, OS-entropy-free RNG suitable for [`Predictor::select`].
///
/// Re-exported so downstream crates don't need a direct `rand_pcg`
/// dependency just to seed a generator:
///
/// ```
/// use rand_core::SeedableRng;
/// use terraphim_sooth::DefaultRng;
///
/// let mut rng = DefaultRng::seed_from_u64(42);
/// let _ = rng;
/// ```
pub type DefaultRng = rand_pcg::Pcg32;

/// A two-symbol context: the pair of symbol ids that precede the symbol
/// being predicted.
pub type Context = (u32, u32);

/// A Markov-chain symbol predictor keyed by [`Context`].
///
/// All state is stored in [`BTreeMap`]s so that serialisation and iteration
/// (including [`Predictor::select`]'s traversal of the observed
/// distribution) are always in a deterministic, sorted-by-key order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Predictor {
    // context -> (symbol -> observation count)
    table: BTreeMap<Context, BTreeMap<u32, u32>>,
}

/// On-the-wire representation of [`Predictor`].
///
/// `BTreeMap` keys that aren't strings (our `Context` tuples and `u32`
/// symbol ids) aren't representable in self-describing formats like JSON, so
/// the predictor (de)serialises as a sorted `Vec` of `(context, counts)`
/// pairs instead. `BTreeMap` iteration is already sorted, so this list is
/// always written in deterministic `context`-then-`symbol` order.
type WireFormat = Vec<(Context, Vec<(u32, u32)>)>;

impl Serialize for Predictor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entries: WireFormat = self
            .table
            .iter()
            .map(|(&context, counts)| {
                let counts: Vec<(u32, u32)> = counts.iter().map(|(&s, &c)| (s, c)).collect();
                (context, counts)
            })
            .collect();
        entries.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Predictor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = WireFormat::deserialize(deserializer)?;
        let table = entries
            .into_iter()
            .map(|(context, counts)| (context, counts.into_iter().collect()))
            .collect();
        Ok(Predictor { table })
    }
}

impl Predictor {
    /// Create an empty predictor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one more observation of `symbol` following `context`.
    ///
    /// Returns the new (post-increment) count for `(context, symbol)`.
    pub fn observe(&mut self, context: Context, symbol: u32) -> u32 {
        let counts = self.table.entry(context).or_default();
        let count = counts.entry(symbol).or_insert(0);
        *count += 1;
        *count
    }

    /// Total number of observations recorded for `context` (sum of all
    /// symbol counts). Zero if the context has never been observed.
    pub fn count(&self, context: Context) -> u32 {
        self.table
            .get(&context)
            .map(|counts| counts.values().sum())
            .unwrap_or(0)
    }

    /// Number of distinct symbols observed for `context`.
    pub fn symbol_count(&self, context: Context) -> usize {
        self.table.get(&context).map(BTreeMap::len).unwrap_or(0)
    }

    /// Weighted-random draw of a symbol from `context`'s observed
    /// distribution, proportional to each symbol's observation count.
    ///
    /// Returns `None` if `context` has never been observed. Deterministic
    /// for a fixed RNG stream: iterates the (sorted) symbol table in symbol
    /// id order, so the same RNG output sequence always yields the same
    /// pick.
    pub fn select<R: Rng + ?Sized>(&self, context: Context, rng: &mut R) -> Option<u32> {
        let counts = self.table.get(&context)?;
        let total: u32 = counts.values().sum();
        if total == 0 {
            return None;
        }
        let mut pick = unbiased_below(rng, total);
        for (&symbol, &count) in counts.iter() {
            if pick < count {
                return Some(symbol);
            }
            pick -= count;
        }
        // Unreachable: `pick < total` by construction of `unbiased_below`,
        // and `counts` sums to `total`, so the loop always returns above.
        None
    }

    /// Information content, in bits, of observing `symbol` in `context`:
    /// `-log2(P(symbol | context))`.
    ///
    /// Returns `None` if `context` has never been observed, or `symbol` has
    /// never been observed within `context`.
    pub fn surprise(&self, context: Context, symbol: u32) -> Option<f64> {
        let counts = self.table.get(&context)?;
        let total: u32 = counts.values().sum();
        if total == 0 {
            return None;
        }
        let count = *counts.get(&symbol)?;
        if count == 0 {
            return None;
        }
        let probability = f64::from(count) / f64::from(total);
        Some(-probability.log2())
    }

    /// Shannon entropy, in bits, of `context`'s observed distribution:
    /// `-sum(p * log2(p))` over all observed symbols.
    ///
    /// Returns `None` if `context` has never been observed.
    pub fn uncertainty(&self, context: Context) -> Option<f64> {
        let counts = self.table.get(&context)?;
        let total: u32 = counts.values().sum();
        if total == 0 {
            return None;
        }
        let entropy = counts.values().fold(0.0_f64, |acc, &count| {
            if count == 0 {
                return acc;
            }
            let probability = f64::from(count) / f64::from(total);
            acc - probability * probability.log2()
        });
        Some(entropy)
    }

    /// Discard all observations, returning the predictor to its initial
    /// (empty) state.
    pub fn clear(&mut self) {
        self.table.clear();
    }

    /// `true` if no observations have ever been recorded.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Number of distinct contexts observed.
    pub fn len(&self) -> usize {
        self.table.len()
    }
}

/// Draw an unbiased value in `[0, bound)` from `rng`, using Lemire's
/// nearly-divisionless rejection method.
///
/// Plain modulo (`rng.next_u32() % bound`) is *biased* toward small values
/// whenever `bound` doesn't evenly divide `u32::MAX + 1`; this keeps
/// [`Predictor::select`] a faithful weighted draw. `bound` is assumed
/// non-zero (checked by the caller).
fn unbiased_below<R: Rng + ?Sized>(rng: &mut R, bound: u32) -> u32 {
    debug_assert!(bound > 0, "unbiased_below requires a positive bound");
    let mut x = rng.next_u32();
    let mut m = u64::from(x) * u64::from(bound);
    let mut low = m as u32;
    if low < bound {
        // Reject outcomes that would bias the distribution: threshold is
        // the remainder of u32::MAX+1 divided by bound.
        let threshold = bound.wrapping_neg() % bound;
        while low < threshold {
            x = rng.next_u32();
            m = u64::from(x) * u64::from(bound);
            low = m as u32;
        }
    }
    (m >> 32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::SeedableRng;

    #[test]
    fn observe_returns_incrementing_count() {
        let mut predictor = Predictor::new();
        assert_eq!(predictor.observe((1, 2), 42), 1);
        assert_eq!(predictor.observe((1, 2), 42), 2);
        assert_eq!(predictor.observe((1, 2), 7), 1);
        assert_eq!(predictor.count((1, 2)), 3);
    }

    #[test]
    fn count_is_zero_for_unknown_context() {
        let predictor = Predictor::new();
        assert_eq!(predictor.count((9, 9)), 0);
    }

    #[test]
    fn counts_sum_correctly_after_many_observes() {
        let mut predictor = Predictor::new();
        for _ in 0..5 {
            predictor.observe((1, 1), 10);
        }
        for _ in 0..3 {
            predictor.observe((1, 1), 20);
        }
        assert_eq!(predictor.count((1, 1)), 8);
        assert_eq!(predictor.symbol_count((1, 1)), 2);
    }

    #[test]
    fn surprise_matches_hand_computed_values() {
        let mut predictor = Predictor::new();
        // Context (0, 0): symbol 1 observed 3 times, symbol 2 observed 1 time.
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 2);

        // P(1) = 3/4 -> surprise = -log2(0.75) ~= 0.415037...
        let surprise_1 = predictor.surprise((0, 0), 1).unwrap();
        assert!((surprise_1 - (-0.75_f64.log2())).abs() < 1e-12);

        // P(2) = 1/4 -> surprise = -log2(0.25) = 2.0 exactly.
        let surprise_2 = predictor.surprise((0, 0), 2).unwrap();
        assert!((surprise_2 - 2.0).abs() < 1e-12);
    }

    #[test]
    fn surprise_is_none_for_unknown_context_or_symbol() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 1);
        assert_eq!(predictor.surprise((9, 9), 1), None);
        assert_eq!(predictor.surprise((0, 0), 999), None);
    }

    #[test]
    fn uncertainty_is_zero_for_single_symbol_context() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 1);
        // Only one symbol ever observed -> deterministic -> zero entropy.
        assert!((predictor.uncertainty((0, 0)).unwrap() - 0.0).abs() < 1e-12);
    }

    #[test]
    fn uncertainty_matches_hand_computed_value_for_uniform_pair() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 2);
        // Uniform over two symbols -> entropy = 1 bit exactly.
        assert!((predictor.uncertainty((0, 0)).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn uncertainty_is_none_for_unknown_context() {
        let predictor = Predictor::new();
        assert_eq!(predictor.uncertainty((0, 0)), None);
    }

    #[test]
    fn select_is_none_for_unknown_context() {
        let predictor = Predictor::new();
        let mut rng = DefaultRng::seed_from_u64(1);
        assert_eq!(predictor.select((0, 0), &mut rng), None);
    }

    #[test]
    fn select_only_returns_observed_symbols() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 10);
        predictor.observe((0, 0), 20);
        predictor.observe((0, 0), 30);

        let mut rng = DefaultRng::seed_from_u64(7);
        for _ in 0..200 {
            let symbol = predictor.select((0, 0), &mut rng).unwrap();
            assert!([10, 20, 30].contains(&symbol));
        }
    }

    #[test]
    fn select_is_deterministic_under_fixed_seed() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 10);
        predictor.observe((0, 0), 20);
        predictor.observe((0, 0), 30);

        let draw = |seed: u64| {
            let mut rng = DefaultRng::seed_from_u64(seed);
            (0..50)
                .map(|_| predictor.select((0, 0), &mut rng).unwrap())
                .collect::<Vec<_>>()
        };

        assert_eq!(draw(42), draw(42));
    }

    #[test]
    fn select_is_weighted_toward_higher_counts() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 1); // count 1
        for _ in 0..99 {
            predictor.observe((0, 0), 2); // count 99
        }

        let mut rng = DefaultRng::seed_from_u64(1234);
        let mut hits_2 = 0u32;
        let trials = 2000;
        for _ in 0..trials {
            if predictor.select((0, 0), &mut rng) == Some(2) {
                hits_2 += 1;
            }
        }
        // Expect close to 99% but allow generous slack to avoid flakiness.
        assert!(
            hits_2 > trials * 90 / 100,
            "hits_2={hits_2} trials={trials}"
        );
    }

    #[test]
    fn clear_resets_all_state() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 1);
        assert!(!predictor.is_empty());
        predictor.clear();
        assert!(predictor.is_empty());
        assert_eq!(predictor.count((0, 0)), 0);
        assert_eq!(predictor.len(), 0);
    }

    #[test]
    fn serde_round_trip_preserves_state() {
        let mut predictor = Predictor::new();
        predictor.observe((1, 2), 3);
        predictor.observe((1, 2), 4);
        predictor.observe((5, 6), 7);

        let json = serde_json::to_string(&predictor).unwrap();
        let restored: Predictor = serde_json::from_str(&json).unwrap();
        assert_eq!(predictor, restored);
    }

    #[test]
    fn unbiased_below_never_reaches_bound() {
        let mut rng = DefaultRng::seed_from_u64(99);
        for _ in 0..1000 {
            let bound = 7;
            let value = unbiased_below(&mut rng, bound);
            assert!(value < bound);
        }
    }
}

/// Runs the same core behaviour under `wasm-bindgen-test` so CI can exercise
/// it on `wasm32-unknown-unknown` (`wasm-pack test --headless`), proving the
/// crate never reaches for OS entropy on that target.
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use rand_core::SeedableRng;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn observe_select_surprise_uncertainty_work_on_wasm32() {
        let mut predictor = Predictor::new();
        predictor.observe((0, 0), 1);
        predictor.observe((0, 0), 2);

        let mut rng = DefaultRng::seed_from_u64(1);
        assert!(predictor.select((0, 0), &mut rng).is_some());
        assert!(predictor.surprise((0, 0), 1).is_some());
        assert!((predictor.uncertainty((0, 0)).unwrap() - 1.0).abs() < 1e-12);
    }

    #[wasm_bindgen_test]
    fn serde_round_trip_works_on_wasm32() {
        let mut predictor = Predictor::new();
        predictor.observe((1, 2), 3);
        let json = serde_json::to_string(&predictor).unwrap();
        let restored: Predictor = serde_json::from_str(&json).unwrap();
        assert_eq!(predictor, restored);
    }
}
