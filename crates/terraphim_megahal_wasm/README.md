# terraphim_megahal_wasm

wasm-bindgen API and Trunk browser demo for
[`terraphim_megahal`](../terraphim_megahal) -- a client-side MegaHAL chat in
the spirit of [megahal.bisks.net](https://megahal.bisks.net/): session-local
learning, brain save/load, personality selection, no server, no LLM.

The UI is deliberately framework-free (web-sys only), matching the org's
other Trunk projects (e.g. terraphim-editor); no Yew/Leptos.

## Develop

```console
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve            # http://127.0.0.1:8090
```

Release build (size-budgeted):

```console
trunk build --release --cargo-profile release-lto
gzip -9 -c dist/*.wasm | wc -c   # must stay < 500 KB
```

## Tests

```console
wasm-pack test --headless --chrome
```

Six tests cover the JS-facing API: personality listing, learn/reply,
seeded determinism, save/load round trip (a restored brain continues the
conversation identically), personality switching, and the reply-latency
budget (measured ~6 ms worst on a default brain; gate at 50 ms).

## Deployment

Live: **https://terraphim-megahal-demo.pages.dev** (Cloudflare Pages,
zestic.ai account, deployed with `wrangler pages deploy dist --branch main`).

Re-deploy after a rebuild:

```console
trunk build --release --cargo-profile release-lto
wrangler pages deploy dist --project-name terraphim-megahal-demo --branch main
```

Cloudflare Pages configuration lives in `wrangler.toml`.

## Binary size budget

The gzipped wasm must stay below **500 KB** (enforced in
`.github/workflows/ci-megahal-wasm.yml`). Current build: ~393 KB. The
`automata` and `persistence` integrations stay out of the browser build
(feature-gated since Phase 3); the `personalities` corpora are embedded and
dominate the budget.
