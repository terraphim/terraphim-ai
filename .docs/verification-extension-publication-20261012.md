# Extension publication verification — 12 October 2026

## Tested outputs

- VS Code: 25 actual WASM/host/filesystem tests and 34 packaging/publication tests pass; TypeScript and ESLint pass. Asset importer verifies 133 files from published editor source 05d0aa50151c131db4a3b30b56aba67c00add5fc. Real ZIP regressions reject extraction aliases, changed XML publisher/version/platform and mismatched assets.
- Native VS Code installed package saved Risk/Concern alternatives and graph JSON. Graph SHA-256: f8648b16470e764bdff5c95064078419d71e8d41d2ca30d4b1508017c166a45d. The export test invokes the packaged Rust helper and a real TextDocument save. It does not demonstrate the webview export dialog.
- Released grep 1.21.19 read that saved graph: concern retrieved three Risk decisions. Retrieval-only returned no answer. A separate direct CLI --answer run returned a source-grounded answer with three citations. Actual JSON/receipts are retained in the local publication evidence directory.
- Shared LSP/core: 421 tests passed across the full two-crate suite. Then five real filesystem alias regressions and a six-scenario stale-source export regression passed with the complete 38-test save/export suite. The stale-source test reproduced an actual obsolete output overwrite before the guard and verifies refusal after edits, closure and same-version reopen. Strict all-targets/all-features LSP Clippy and formatting passed. The shared exporter uses the released alternatives crate, including synonyms:: parsing, typed comma terms, conflict refusal, detached exclusion and save/reopen/history tests.
- Native-launch failure regression uses a nonexistent executable and intentionally exits nonzero. Its retained FAIL receipt identifies the exact snapshotted VSIX hash, proving startup failures do not disappear from CI evidence.

## Actual CI qualification

[VS Code run 38216701669](https://github.com/terraphim/terraphim-ai/actions/runs/38216701669) passed package creation and native Ubuntu 24.04, macOS 14 and Windows 2025 at source `673c921d7fe7b02a2997395af8610b8c61f432a8`. All hosts executed the same VSIX SHA-256 `50f0b1fc060cfa2e77406961dfaec1f0c5b476fbc369888d3bc856fbb469419e`. Downloaded output files were independently hashed: native edit/save, undo, redo, saved annotated brief and exported graph matched every receipt, identically across platforms. Undo differs from the edited file; redo equals it. The earlier Windows failure was real CRLF conversion of the checked-out asset manifest; an LF Git attribute and actual Git checkout regression fixed it without weakening byte checks.

[Continue run 38215897542](https://github.com/terraphim/terraphim-ai/actions/runs/38215897542) passed six real transport/configuration/package/source tests. The downloaded bundle records a clean source tree. Actual MCP receipt: baseline Concern query returned zero chunks; the exported graph returned the Risk line from brief.md, with answer null. The earlier three-decision direct-CLI evidence is a separate corpus/run.

[Zed Gitea run 39016](https://git.terraphim.cloud/terraphim/zed-terraphim/actions/runs/39016) at `76b256d0c90770e8134a8aa124dd11d509c90ea0` passed formatting, native/WASI Clippy, 74 Rust tests, WASI build, architecture guards and 42 Python tests. Actual job logs were read, rather than relying on the summary API (which leaves individual step statuses queued). Runner path-normalization warnings are infrastructure diagnostics; compiler Clippy gates passed. This does not qualify native Zed UI behavior or public installation.

## Legacy command repair

Final package review found two contributed commands depending on absent bindings. The existing `rust-lib` now builds from published Terraphim automata 1.20.4 with Rust 1.96.1/wasm-pack 0.13.1; actual compiled helper and extracted-VSIX tests produce exact Risk/Concern Markdown links while preserving Unicode/CRLF. Invalid graph data throws rather than yielding an empty document. Two Rust and four Node/WASM tests passed, along with 34 Python package/guard tests and prior host/export tests. The legacy NAPI command ID shares the existing server provider and refuses an unavailable server. Its new native tests exercise the installed helper and real unavailable endpoint; the latest native matrix must pass before this repaired package is release-qualified. Live Atomic-resource end-to-end behavior remains separate from helper proof.

## Independent review and repairs

Review identified and implementation fixed: ZIP extraction aliases, unchecked marketplace XML identity, lost failure receipts, mutable-package receipt hashes, background-editor history routing, and LSP symlink/hardlink source overwrite. Filesystem export regressions reproduced real byte corruption before the guard and pass after it. Zed release/package binding and Continue tag/source binding underwent separate review.

## Unqualified gates

- Native webview interactions, export dialogs/cancellation, clean-profile upgrade/rollback and the complete 41-requirement matrix remain unqualified. The prior local Mac focus failure is retained as a failed attempt; all three native CI hosts subsequently passed the enabled document/history assertions.
- Native stock Zed and Continue UI acceptance remain pending. All 41 original requirements are not certified for these hosts. The Zed application fork is untouched.
- Continue retrieval uses a real SDK client/server and released grep; native Continue UI and optional provider synthesis are separate. Automatic approval review rejected the optional Continue/OpenRouter synthesis check; no retry or bypass occurred.
- No marketplace publication, account federation, public Zed source mirror or catalog merge is claimed. See the publication design for prerequisite order and CI gates.

## Static analysis

UBS was run with the default repository policy and explicitly on the extension files (the default ignore file excludes VS Code). Its Rust production scan reported zero critical findings; warning inventory includes existing test assertions and conservative async-lock/clone/slicing heuristics. The initial JS global-assignment findings were declared closure variables; declarations were separated for clarity. Public SHA-256 comparisons now use explicit constant-time comparison. Continue's only critical static findings are equality comparisons of public SHA-256 file digests, not secret material; independent review and six real tests pass. Static findings are reviewed evidence, not a substitute for Rust/TypeScript tests.

## Evidence locations

Local evidence: `synonym-search-story/extensions-20261012/vscode-export-review`, `vscode-grep` under the task's visualization directory. Continue transport/schema/package receipts are committed under `integrations/continue/proof`. CI artifacts preserve host/platform/package digests, actual saved output and failure receipts. The publication design is `docs/plans/design-extension-marketplaces.md`.

## Public monorepo baseline check

The global commit hook checks the root Rust workspace, which excludes the extension's standalone `rust-lib`. That root check fails in unchanged `terraphim_rlm/src/mcp_tools.rs` against the resolved MCP API (missing Content/RawContent, non-exhaustive Tool literals and removed .raw fields). The same failures were reproduced in an isolated checkout of public main `09670565c4434519c0a688d093467f48a9eb4b17`; those root files are unchanged by this extension PR. The diagnostic Cargo.lock rewrite was discarded. This is a recorded failing baseline, not an extension test pass.

For the extension-only commit, a temporary copy of the existing commit hook targets its Rust checks at the standalone extension Cargo.toml with the pinned toolchain and --locked; all secret/size/format/config/static checks and commit-message validation remain. No persistent hook setting is changed. The private LSP/core commits passed the original complete workspace hook.
