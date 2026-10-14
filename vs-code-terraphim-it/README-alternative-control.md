# Alternative Control in the existing Terraphim It extension

This preview is an opt-in custom Markdown editor in `applied-knowledge-systems.terraphim-it`. Open a Markdown file with **Terraphim: Open Alternative Control**. Click the word counter for Write_On. Existing server and Atomic commands remain available in trusted workspaces.

The view bundles the released terraphim-editor v0.1.0-rc.2 UI, WASM and source-pinned host bridge. The bundle records its public source repository and Cargo.lock checksum. Each view has its own session; VS Code owns document revisions, persistence and undo/redo. Exact source/revision checks protect asynchronous proposals. External edits retain conflicting drafts for recovery. Native export refuses the source itself and dirty output buffers. Browser persistence is disabled in this host.

**Export Document Knowledge Graph** waits for the current edit to be acknowledged, reads that exact source, and uses the shared Rust `synonyms::` parser/types to create the portable graph. It refuses malformed annotations or ambiguous concepts. The document and active role are unchanged. Save, undo, redo and reopen determine which aliases exist in the export.

A fresh checkout runs `npm ci --ignore-scripts`, `npm run editor:fetch`, `npm run compile`, `npm run lint`, `npm run test:assets`, `npm run test:host`, `npm run test:export`, then `npm run package:vsix`. Release inputs are pinned by URL/commit/SHA-256 in `editor-release.json`; every imported file is checked. Runtime dependencies are included. Generated binaries stay out of Git.

CI builds one VSIX and tests that package in native Linux, macOS and Windows VS Code profiles. Tag release and marketplace workflows promote the same verified package. Publisher identity setup is documented in `docs/plans/design-extension-marketplaces.md` in the repository.

The GitHub preview release does not depend on marketplace account setup. After
its exact package passes qualification and anonymous download verification,
`vscode-release.yml` reads the repository readiness variables
`VSCODE_PUBLISH_VISUAL_STUDIO_MARKETPLACE` and `VSCODE_PUBLISH_OPEN_VSX`.
Unset or `false` skips that marketplace and records `NOT_PUBLISHED` in the run
summary and `vscode-marketplace-plan-*` artifact. Only exact `true` schedules
an attempt; other values refuse all attempts. Set a gate only after its existing
publisher, protected environment and federated identity are ready. Enabled
targets remain `PENDING_PUBLICATION` until the separate registry readback passes.
These gates do not create access, weaken tag policies or provide a PAT fallback.

Acceptance reports distinguish actual saved files, engine tests and visual UI checks. The original 41 visual requirements are not all accepted. This is a scoped preview; stock Zed visual limitations and the separate Zed fork remain outside this VSIX's claims.
