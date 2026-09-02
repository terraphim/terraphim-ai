# A 1998 Loebner Prize Winner, Reborn in Rust/Wasm with a Knowledge Graph

**Date:** 2026-09-02
**Author:** Terraphim AI Team
**Status:** DRAFT for CTO review (issue terraphim/terraphim-ai#3264) -- do not publish without sign-off

Before ChatGPT, before Siri, there was MegaHAL: Jason Hutchens' Markov-chain chatterbot that won the 1998 Loebner Prize competition by charming judges with statistical wit. We have ported it, line for line, from Ruby to Rust, compiled it to WebAssembly, and put it back on the web -- fully client-side, fully deterministic, zero LLM.

**Try it now:** [terraphim-megahal-demo.pages.dev](https://terraphim-megahal-demo.pages.dev)

## Why resurrect a chatterbot from 1998?

Because what MegaHAL represents matters more than what it says. It is:

- **Zero-LLM**: five second-order Markov models on the hot path. No API keys, no tokens, no cloud.
- **Deterministic**: same seed plus same training equals byte-identical conversations. We prove it, with fixtures.
- **Auditable**: every reply word is traceable to the trained corpus. When a model cannot hallucinate beyond its inputs, you can audit what it knows by construction.
- **Tiny**: the whole browser bundle is under 400 KB gzipped -- smaller than a single hero image.

Those are the same principles we build Terraphim search on: local-first, private, and honest about what the machine does and does not know.

## The five-model brain

MegaHAL is not one Markov chain but five, each learning a different slice of language:

1. **seed** -- which words appear adjacent to each other (the conversation-starter)
2. **fore** -- a second-order forward model: what follows what
3. **back** -- the same, backwards: how sentences begin
4. **case** -- how to un-shout ALL-CAPS normalisation back into human casing
5. **punc** -- where the commas and full stops go

`reply()` extracts keywords from your input, uses the seed model to plant one adjacent to a keyword, then grows the sentence forwards and backwards with random walks, scores candidates by accumulated surprise (normalized information content), and finally rewrites the winner through the case and punctuation models. It is a small marvel of 90s NLP engineering, and the Ruby rewrite by Hutchens made it readable enough to port faithfully.

## The conformance harness: a Ruby ghost in the Rust machine

A port is only as good as its fidelity claim. Ours is mechanical, not aspirational: the upstream Ruby sources are vendored (Unlicense), a pure-Ruby mirror of the Rust PRNG (PCG32, matching `rand_pcg` bit for bit) drives the real upstream logic, and the resulting replies are frozen as golden fixtures. The Rust engine must replay those conversations byte for byte, across greetings, keyword seeding, learning toggles, and multi-turn personality conversations. Any divergence is a bug in the port until proven otherwise.

The subtle bugs we caught this way tell the story: Ruby's `Array#zip` pads with nil where Python's truncates (the trailing comma of every reply depended on it); `Hash[ANTONYMS + ANTONYMS.reverse]` makes the *last* duplicate antonym win; and the C sooth kernel's `select(id, count)` deterministically returns the highest observed symbol -- a quirk the seed model's behaviour secretly relies on. Faithfulness is in the details.

## The knowledge graph twist

Where we depart from 1998 is deliberate. Behind a feature flag, `terraphim_automata` runs Aho-Corasick matching of a role thesaurus over your input, and matched knowledge-graph concepts are injected into the reply seeding. The chatterbot stops reciting generic corpus fragments and starts gravitating towards your domain vocabulary: ask a Terraphim-seeded brain about "search" and it answers with *your* search concepts. A KG-aware Markov bot: auditable, offline, and speaking your language.

## Proving that training works

The demo ships with a Brain Lab: two brains share one seed and receive the same input. Train one through the panel and watch its replies diverge from its untrained twin -- a live, side-by-side proof that learning changes behaviour. The same proof runs in CI: seven seeded tests pin that new training changes text, that every reply word traces to the trained corpus, that personality switches change the brain, and that brain snapshots freeze evolution exactly.

## Try it

The demo runs entirely in your browser (session-local learning, brain saved to localStorage, nothing leaves the tab): [terraphim-megahal-demo.pages.dev](https://terraphim-megahal-demo.pages.dev)

For the terminal-minded:

```console
cargo install terraphim_megahal --registry terraphim
megahal
```

The crates (`terraphim_sooth`, `terraphim_megahal`) are on our internal registry; the sooth predictor is useful well beyond chatting -- novelty detection, watermark-style surprise scoring -- and is yours to build on.

## Credits

MegaHAL was created by Jason Hutchens (1998 C original; 2014 Ruby rewrite, Unlicense). The sooth predictor kernel is his as well. This port keeps the public-domain spirit: attribution preserved, licence Apache-2.0 for the new code. The browser demo follows the spirit of megahal.bisks.net.

*Drafted by the Terraphim AI team. Numbers in this post: bundle 392-395 KB gzipped (budget 500), worst reply latency ~6 ms in-browser (budget 50 ms), conformance suite 7 scenarios + 8 self-check seeds, all green.*
