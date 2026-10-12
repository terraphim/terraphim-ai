# Extension publication design — 12 October 2026

Continue the approved publication implementation, retaining the existing VS Code and stock Zed integrations. The user has explicitly requested Continue integration too. This is a release plan with execution gates; it is not a claim that listings are published or all 41 requirements pass.

## User flow and shared implementation

1. Open a project brief in Terraphim Alternative Control inside VS Code, or Markdown with the Terraphim LSP in stock Zed.
2. Create a `concern` alternative for `risk`, swap wording, save and reopen. Shared Terraphim Rust code records the alternatives through `synonyms::` types.
3. Export `<brief>.knowledge-graph.json`. The source and shared role are unchanged. Conflicting concepts and malformed annotations are refused.
4. Search the project corpus with released `terraphim-grep concern --thesaurus <graph> --paths <corpus> --search-only --json`; compare with `--answer --json` using the same graph and corpus.
5. In the existing Continue extension's Agent mode, retrieve context through the same released grep adapter. Synthesis is a separate explicit tool, so an LLM answer cannot be confused with retrieved evidence.

The browser and VS Code share the published editor WASM. The LSP reuses `terraphim_alternatives` and `terraphim_types`; the extension contains no alternative synonym implementation. Continue delegates to released grep; it does not create a second graph or reimplement normalization/search.

## Release units and sequence

| Order | Release unit | CI qualification | Publication |
|---|---|---|---|
| 1 | Shared LSP with document graph export | Locked Rust core/LSP tests, warnings-denied Clippy, native file exports, cross-platform builds | New immutable LSP candidate after PR merge; raw binaries plus checksum manifest |
| 2 | VS Code 0.1.0 preview | Reproducible editor import, TypeScript/lint, actual WASM tests, real VSIX verification, native package lifecycle on Linux/macOS/Windows | GitHub preview asset then same VSIX to VS Marketplace and Open VSX |
| 3 | Continue integration | Real MCP SDK transport against released grep, baseline/registered graph comparison; provider synthesis separately qualified | Dedicated tag-driven bundle/configuration release with checksums; Continue Hub block after namespace/API qualification |
| 4 | Existing Zed adapter update | Rust/WASI tests and size guards; every pinned binary anonymously downloadable; native stock-Zed saved outputs | Reviewed adapter archive; separately qualified catalog LSP package |
| 5 | Website/distribution links | Anonymous exact-version installs/readback, launch/version check, same synonym-to-search saved output | Update terraphim.ai capability/download documentation only with qualified release links |

Do not move an existing tag or replace existing published asset bytes. A fix uses a new version. Existing installed VS Code users retain `applied-knowledge-systems.terraphim-it`; Open VSX uses that same identity. The preview host floor is VS Code 1.126, the version used for native qualification, until an older-version matrix establishes compatibility.

## Files and CI ownership

- `vs-code-terraphim-it/editor-release.json` pins public release source, archive, manifest and bridge hashes. `scripts/released_editor.py` imports those exact files without private Cargo access.
- `src/alternativeControl.ts` owns native source/version checks, saves, exports and routing. `media/knowledge-graph.cjs` invokes the released Rust exporter on acknowledged source. `media/export-target.cjs` prevents overwriting source aliases or dirty documents.
- `scripts/verify_vsix.py` verifies real ZIP paths, both JSON and marketplace XML identities, preview/engine metadata, required runtime files and every shared asset hash. Source dirty state is explicit.
- `.github/workflows/vscode-extension.yml` builds once, then launches the extracted same VSIX in disposable profiles on all three platforms. Receipts survive failure and carry the package hash. No marketplace credentials enter PR CI.
- `.github/workflows/vscode-release.yml` accepts exact version tags on reviewed main history. Native qualification is mandatory. Publication uses frozen CI artifacts, draft/resume with byte comparisons, and post-publication anonymous payload checks.
- Zed `.github/workflows/ci.yml`, `release.yml`, `catalog.yml` continue existing Rust/WASI and packaging work. The release contract is compiled from the actual provisioning constants, avoiding a second guessed pin. The catalog workflow creates/reuses a PR and never merges it automatically.
- Continue integration lives in `integrations/continue`, uses the existing Continue configuration format and a pinned official MCP SDK. Its tests must execute a released grep binary and save protocol/output receipts.

## Zed catalog migration

Keep the existing combined `terraphim` adapter working for current users. Introduce a separate catalog source/package named `terraphim-lsp` containing only the language-server manifest. Reuse the existing provisioning implementation and tests; no Zed application fork is involved. Carry the old MCP capability in the existing package until users can migrate to a separately distributed MCP configuration/registry entry. The two language-server packages must not run simultaneously on the same workspace; document disabling the legacy LSP when installing the catalog entry.

Before creating the catalog PR: publish an anonymously readable source mirror containing only the reviewed extension code; publish a new LSP candidate with graph export for every supported platform; update the pin from verified assets; then test that exact extension commit inside stock Zed. The current mixed manifest and missing public rc.3 artifacts deliberately fail the catalog gate. The migration package is an outstanding implementation dependency, not a completed listing.

## Account setup and trust

VS Marketplace: retain the publisher, configure its Entra principal and GitHub environment federation, then set `VSCE_AZURE_CLIENT_ID` and `VSCE_AZURE_TENANT_ID`. Open VSX: confirm namespace ownership and agreement and configure the repository/workflow/environment trusted publisher. These account bindings were not present in the inspected repository variables; do not fabricate a successful login or fall back to printing tokens.

Continue Hub: verify Terraphim namespace access and a currently supported publishing API/action before implementing its upload job. The previously documented publish-action source currently returns 404. The bundle release is already automated independently; Hub availability must not be inferred from it. The Hub block must accept user-supplied paths and resolve through Continue's real registry loader before release status is green.

Zed: use a public catalog fork and narrowly scoped `ZED_CATALOG_TOKEN` only in the catalog environment. The catalog input records the exact native-qualified source and an HTTPS evidence URL. This is a maintainer attestation; it does not automatically prove GUI behavior. Source publication and catalog acceptance are separate states.

For the private Gitea monorepo, preserve the existing PR. Publish a scoped VS Code/Continue source PR against the public repository instead of pushing unrelated private history. Binary-only public LSP releases must identify their reviewed source and immutable tag.

## Acceptance and evidence

- Saved source contains actual alternatives; exported JSON maps concern and risk to the same concept and preserves typed terms including commas and Unicode.
- Save/reopen and Undo/Redo are verified from resulting files, not input keystrokes. A focus switch must not route a queued history command to another document.
- Stale revisions, malformed annotations, conflicting concepts, cancelled destinations and dirty output buffers refuse safely. Export does not mutate source or other documents.
- Baseline corpus has risk and no concern. Without the exported graph, concern does not retrieve the risk decisions; with it, both grep and Continue return those actual sources. Synthesis on/off remains separately labelled.
- Native host histories, actual dialogs/webview interactions, the complete 41-requirement matrix and clean-profile install/upgrade/uninstall must have independent receipts before broad feature claims.
- Public release verification downloads exact-version artifacts anonymously and compares bytes or signed VSIX payloads. A catalog PR or CI artifact is not marketplace publication.

Current partial evidence is recorded in `.docs/verification-extension-publication-20261012.md`. Any red/native or account gate remains visible; a passing core test cannot clear it.

## Implementation and review status

- [Public VS Code and Continue PR #972](https://github.com/terraphim/terraphim-ai/pull/972) contains the scoped source and automated package/release workflows. The same VSIX passed Linux/macOS/Windows native lifecycle in [run 38216701669](https://github.com/terraphim/terraphim-ai/actions/runs/38216701669); the final legacy-command repair at source `4c45aa221` also passed all three hosts in [run 38218712077](https://github.com/terraphim/terraphim-ai/actions/runs/38218712077), with all 18 actual saved output hashes verified.
- [Private LSP/core PR #3484](https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/3484) adds authored synonym export using existing crates and protects source aliases/stale snapshots. Native Zed requalification remains a release gate.
- [Existing Zed adapter PR #34](https://git.terraphim.cloud/terraphim/zed-terraphim/pulls/34) adds immutable release/readback and catalog-PR CI. Existing Gitea CI passed actual 74 Rust and 42 Python tests; public GitHub/catalog prerequisites above remain pending.
- [Publication tracking issue #3488](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3488) records the cross-repository release sequence and actual evidence.

Marketplace publication is driven only by the reviewed version tag and qualified artifacts. Credentials/account ownership, upstream Zed acceptance and Continue Hub's unavailable publication contract cannot be replaced by a passing local test.
