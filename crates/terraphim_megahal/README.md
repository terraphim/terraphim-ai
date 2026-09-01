# terraphim_megahal

A faithful Rust port of [MegaHAL](https://github.com/kranzky/megahal), Jason
Hutchens' second-order Markov chatterbot (winner of the 1998 Loebner Prize
competition), built on [`terraphim_sooth`](../terraphim_sooth). The upstream
Ruby gem is Unlicensed (public domain); this port carries the same licence
via the workspace.

## Why

Zero-LLM, deterministic, offline, auditable text generation. The engine never
touches OS entropy or the clock: every "random" decision flows from a caller
supplied seedable RNG, so the same seed plus the same training text yields
byte-identical conversations on native and `wasm32-unknown-unknown`.

## Usage

```rust
use rand_core::SeedableRng;
use terraphim_megahal::MegaHal;
use terraphim_sooth::DefaultRng;

let mut hal = MegaHal::new(); // trained on the embedded :default personality
let mut rng = DefaultRng::seed_from_u64(42);

hal.learn("I love Rust and WebAssembly.");
let reply = hal.reply(Some("What do you think about Rust?"), &mut rng);
println!("{reply}");

let greeting = hal.reply(None, &mut rng); // ask for a greeting
```

CLI (native builds):

```console
$ cargo run -p terraphim_megahal --bin megahal
MegaHAL (terraphim port). Type /help for commands.
> /help
```

Commands: `/help`, `/reset`, `/train <file>`, `/save <file>`,
`/load <file>`, `/personality <name>`, `/list`, `/learning <on|off>`,
`/quit`. Set `MEGAHAL_SEED` for a reproducible conversation.

## Determinism contract

- `rand(n)` consumes exactly one `u32` from the injected RNG and yields
  `value % n` (`n == 0` yields 0).
- `Array#shuffle.first` (candidate and context selection) is a descending
  Fisher-Yates shuffle over the same RNG.
- The RNG is `rand_pcg::Pcg32` re-exported as
  [`terraphim_sooth::DefaultRng`], seeded with
  `SeedableRng::seed_from_u64`.

This contract mirrors the upstream Ruby gem under a canonicalisation and is
pinned by golden fixtures: `tests/conformance.rs` replays
`fixtures/megahal_fixtures.json`, captured from the real upstream logic by
`scripts/generate_megahal_fixtures.rb` (which mirrors the same PCG32 in pure
Ruby). Any divergence is a bug in the port until the fixture is regenerated
with provenance.

## Personalities

The `:default` corpus is always embedded (`MegaHal::new`). The eleven other
upstream personalities (Sherlock, Pepys, Star Trek, Star Wars, ...) are
behind the `personalities` feature:

```toml
terraphim_megahal = { version = "...", features = ["personalities"] }
```

## Divergences from upstream

- Brain files: upstream's zip+Ruby-Marshal format is not portable; this port
  defines its own JSON format (version tag `MHRS1`).
- Language detection: upstream uses CLD to fall back to character
  segmentation for CJK-like languages; this port uses a Unicode-range
  heuristic (English and alphabetic scripts are unaffected).
- The Ruby `Kernel#rand`/`Array#shuffle` RNG is replaced by the canonical
  contract above (identical on both sides of the conformance harness).

## Attribution

MegaHAL was written by Jason Hutchens (1998 C; 2014 Ruby rewrite). The sooth
predictor kernel and the Ruby gem sources are public domain (Unlicense).
