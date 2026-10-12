# Extension publication verification — 12 October 2026

## Tested outputs

- VS Code: 25 actual WASM/host/filesystem tests and 30 packaging/publication tests pass; TypeScript and ESLint pass. Asset importer verifies 133 files from published editor source 05d0aa50151c131db4a3b30b56aba67c00add5fc. Real ZIP regressions reject extraction aliases, changed XML publisher/version/platform and mismatched assets.
- Native VS Code installed package saved Risk/Concern alternatives and graph JSON. Graph SHA-256: f8648b16470e764bdff5c95064078419d71e8d41d2ca30d4b1508017c166a45d. The export test invokes the packaged Rust helper and a real TextDocument save. It does not demonstrate the webview export dialog.
- Released grep 1.21.19 read that saved graph: concern retrieved three Risk decisions. Retrieval-only returned no answer. A separate direct CLI --answer run returned a source-grounded answer with three citations. Actual JSON/receipts are retained in the local publication evidence directory.
- Shared LSP/core: 421 tests passed across the full two-crate suite. Then the five real filesystem alias regressions passed with the complete 37-test save/export suite. Strict all-targets/all-features LSP Clippy and formatting passed. The shared exporter uses the released alternatives crate, including synonyms:: parsing, typed comma terms, conflict refusal, detached exclusion and save/reopen/history tests.
- Native-launch failure regression uses a nonexistent executable and intentionally exits nonzero. Its retained FAIL receipt identifies the exact snapshotted VSIX hash, proving startup failures do not disappear from CI evidence.

## Independent review and repairs

Review identified and implementation fixed: ZIP extraction aliases, unchecked marketplace XML identity, lost failure receipts, mutable-package receipt hashes, background-editor history routing, and LSP symlink/hardlink source overwrite. Filesystem export regressions reproduced real byte corruption before the guard and pass after it. Zed release/package binding and Continue tag/source binding underwent separate review.

## Unqualified gates

- Native VS Code history remains FAIL at the focused-window precondition on the locked Mac. A plain native file also could not execute undo without focus; no Terraphim-only history diagnosis is inferred. Existing undo/redo assertions remain enabled. Windows/Linux native workflows must run; no result is invented.
- Native stock Zed and Continue UI acceptance remain pending. All 41 original requirements are not certified for these hosts. The Zed application fork is untouched.
- Continue retrieval uses a real SDK client/server and released grep; native Continue UI and optional provider synthesis are separate. Automatic approval review rejected the optional Continue/OpenRouter synthesis check; no retry or bypass occurred.
- No marketplace publication, account federation, public Zed source mirror or catalog merge is claimed. See the publication design for prerequisite order and CI gates.

## Static analysis

UBS was run with the default repository policy and explicitly on the extension files (the default ignore file excludes VS Code). Its Rust production scan reported zero critical findings; warning inventory includes existing test assertions and conservative async-lock/clone/slicing heuristics. The initial JS global-assignment findings were declared closure variables; declarations were separated for clarity. Public SHA-256 comparisons now use explicit constant-time comparison. Continue's only critical static findings are equality comparisons of public SHA-256 file digests, not secret material; independent review and six real tests pass. Static findings are reviewed evidence, not a substitute for Rust/TypeScript tests.

## Evidence locations

Local evidence: `synonym-search-story/extensions-20261012/vscode-export-review`, `vscode-grep` under the task's visualization directory. Continue transport/schema/package receipts are committed under `integrations/continue/proof`. CI artifacts preserve host/platform/package digests, actual saved output and failure receipts. The publication design is `docs/plans/design-extension-marketplaces.md`.
