# Atomic WASM Demo

This tiny project demonstrates how to call the `atomic-server-client` crate from the browser via WebAssembly.
It is entirely static (HTML + CSS + JS) and uses [trunk](https://trunkrs.dev) to bundle / serve the `.wasm`.

## Prerequisites

1. Install the `wasm32` target and trunk (once):

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli
```

2. Ensure an Atomic Server is running (defaults to `http://localhost:9883`).

## Build & run

```bash
cd wasm-demo
trunk serve --release    # opens http://127.0.0.1:8080
```

## What happens?

* `init client` invokes `init_client`, wiring the JS UI with a `Store` that talks to your server.
* Subsequent buttons issue `create`, `read`, `update`, `delete` requests through the WASM layer using
  the same API surface you already use in native Rust.

The result / error for each step is appended to the on-page `<pre>` output and logged to the browser console.

## Changing the code

Edit `src/lib.rs` or `index.html`, re-load the page; trunk rebuilds automatically.

Enjoy hacking! :rocket:

Run with Trunk and open demo:

```
trunk serve
```

Open http://localhost:8080/ in your browser for the CRUD playground or navigate to `/tests.html` for the automated WASM test runner. You can also click the link at the top of the index page.
