# terraphim_sooth

A deterministic Rust port of the [Sooth] predictor -- a small, general-purpose
stochastic model that answers "what usually follows this context", "how
surprising was this symbol" and "how uncertain is this context". Sooth is the
predictive core behind [MegaHAL] and [TypingSimulator]; this crate is Phase 1
of a Terraphim MegaHAL-style port (terraphim/terraphim-ai#3260).

## Attribution

Sooth was designed and written by [Jason Hutchens](https://github.com/jasonhutchens)
and released into the public domain under the [Unlicense]. This crate is an
independent Rust re-implementation of Sooth's observation/select/surprise/
uncertainty formulas (see `ext/sooth_native/sooth_predictor.c` in the
[upstream repository][Sooth]) -- no upstream code is vendored or linked.
Importing the Ruby gem's `Marshal`-based `.save`/`.load` files is explicitly
out of scope for this port; `terraphim_sooth::Predictor` uses its own `serde`
representation (see [`Serialisation`](#serialisation) below).

## Design

- **`Context = (u32, u32)`.** Upstream Sooth's native context/event are plain
  scalars; this port extends the context to a two-symbol tuple to match
  MegaHAL's bigram lookups (a "context" is the two words preceding the word
  being predicted).
- **Deterministic state.** All observation counts are stored in
  [`BTreeMap`](std::collections::BTreeMap), never `HashMap` -- iteration
  order (and therefore `select`'s traversal of the observed distribution) is
  always the same for the same sequence of observations.
- **No OS entropy.** `Predictor::select` takes an injected
  [`rand_core::Rng`] implementation rather than reaching for
  `thread_rng()`/`OsRng` itself. [`DefaultRng`] re-exports a seedable PCG
  generator (`rand_pcg::Pcg32`) for convenience. This keeps the crate usable
  on `wasm32-unknown-unknown`, where OS entropy needs extra glue (`getrandom`
  with the `js` feature) that this crate deliberately avoids depending on.

## Usage

```rust
use rand_core::SeedableRng;
use terraphim_sooth::{DefaultRng, Predictor};

let mut predictor = Predictor::new();
let context = (1, 2); // symbol ids of the two preceding words

predictor.observe(context, 10); // "the"
predictor.observe(context, 10); // "the" again
predictor.observe(context, 20); // "a"

assert_eq!(predictor.count(context), 3);
assert_eq!(predictor.surprise(context, 20), Some(3f64.log2())); // -log2(1/3) == log2(3)
assert!(predictor.uncertainty(context).unwrap() > 0.0);

let mut rng = DefaultRng::seed_from_u64(42);
let next = predictor.select(context, &mut rng); // Some(10) or Some(20)
assert!(next.is_some());
```

## API

| Method | Meaning |
| --- | --- |
| `observe(context, symbol) -> u32` | Record one more observation; returns the new count for `(context, symbol)`. |
| `select(context, &mut rng) -> Option<u32>` | Weighted-random draw from the observed distribution, proportional to counts. |
| `surprise(context, symbol) -> Option<f64>` | `-log2(P(symbol \| context))`, in bits. |
| `uncertainty(context) -> Option<f64>` | Shannon entropy of `context`'s distribution, in bits. |
| `count(context) -> u32` | Total observations for `context`. |
| `clear()` | Reset to the empty predictor. |

## Serialisation

`Predictor` implements `serde::Serialize`/`Deserialize` directly, using its
own wire format: a sorted list of `(context, [(symbol, count), ...])` pairs
(`BTreeMap` keys aren't representable as JSON object keys, since they aren't
strings). This round-trips through any `serde` format (JSON, bincode, ...);
it is not compatible with the upstream gem's binary `Marshal`/`MH11` save
files.

## Golden fixtures

`fixtures/sooth_fixtures.json` holds observation sequences with expected
`surprise`/`uncertainty`/`count` values, asserted against in
`tests/golden_fixtures.rs`. The formulas are pure functions of observation
counts (count-based frequency, `-log2` surprise, Shannon-entropy
uncertainty) taken directly from `sooth_predictor.c`, so these values are
what the real gem would produce for the same observations.

The fixtures currently ship as computed directly from those formulas rather
than captured by running the actual gem, because Ruby and a C toolchain to
build the `sooth_native` extension weren't available in the environment that
produced this port. `scripts/generate_fixtures.rb` regenerates the fixture
file against the real `sooth` gem (`gem install sooth`) when Ruby is
available -- diff its output against the committed file to confirm
agreement.

[Sooth]: https://github.com/jasonhutchens/sooth
[MegaHAL]: https://megahal.sourceforge.net/
[TypingSimulator]: https://github.com/jasonhutchens/typingsimulator
[Unlicense]: https://github.com/jasonhutchens/sooth/blob/master/UNLICENSE
