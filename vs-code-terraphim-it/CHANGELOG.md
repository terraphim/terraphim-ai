# Changelog

## 0.1.5 (preview)

- Keep the handled Overflow stash shortcut inside Alternative Control, so it does not also open Extensions and move keyboard focus. Unhandled shortcuts retain native VS Code behavior.

- Known limitation: live alternative typing is still not grouped into one native Undo step, and Escape does not reliably cancel the whole pending alternative ([#995](https://github.com/terraphim/terraphim-ai/issues/995)). This release fixes the separate stash shortcut conflict.

## 0.1.4 (preview)

- Require the exact indexed prerelease and downloaded package payload before CI reports marketplace availability.
- Verify Cursor and Windsurf catalogs separately from Open VSX publication; retain complete catalog responses.
- Require Open VSX trusted-publishing metadata and remove the obsolete PAT bootstrap route. Namespace ownership and trusted-publisher setup still gate its publication.
- Document immutable release readback and separate catalog availability from native host qualification.

## 0.1.3 (preview)

- Read the configured Atomic agent on first autocomplete activation and rebuild the store when its agent or endpoint changes. Clearing the agent selects anonymous access; invalid credentials refuse activation.
- Apply `enableLocalServer` to every server request: disabling it refuses localhost and loopback URLs with an actionable warning. An explicit remote `serverUrl` is required; no remote fallback is selected automatically.
- Discard stale autocomplete responses when the role, endpoint, agent or query changes while loading.
- Correct the Atomic autocomplete instructions: it triggers after a space, not merely after two characters.
- Include runnable `terraphim-grep` synonym-search and `terraphim-agent` memory examples, with separate CLI installation and links to the public walkthroughs.

## 0.1.2 (preview)

- Verify the federated service principal and its Azure DevOps profile before Marketplace publication; retain non-secret identity evidence and refuse an unconfirmed profile.

## 0.1.1 (preview)

- Bundle the verified editor v0.1.0-rc.2 release built with public Terraphim crate dependencies.
- Check the editor release manifest against its pinned source commit before importing any assets.
- Verify and retain the public source repository and Cargo.lock checksum when the editor release supplies them.
- Publish the qualified GitHub VSIX independently of marketplace setup; disabled marketplace targets are explicitly reported as not published.

## 0.1.0 (preview)

- Extend the existing Terraphim It extension with the shared Alternative Control Markdown editor.
- Create, edit and swap alternatives with document-scoped KG synonym registration, native save/history and clean Markdown export.
- Export an acknowledged document's knowledge graph through the existing Terraphim Rust `synonyms::` parser and types. The resulting JSON can be passed to `terraphim-grep --thesaurus`.
- Bundle verified editor v0.1.0-rc.1 bytes. Extension users need no Rust toolchain; package builders use a pinned Rust toolchain and public dependencies.
- Package the existing Terraphim link engine as portable WASM; refuse invalid data and stale document replacements. Preserve the legacy NAPI command ID through the existing health-checked server provider.
- Automate package integrity, real-WASM tests and native packaged lifecycle checks before marketplace publication.
- Require VS Code 1.126 or newer for this preview's native qualification. The original 41 visual requirements are not all certified.

Existing server/Atomic commands remain. KG suggestions use the selected role's vocabulary; the shared editor has no built-in LLM alternatives provider.
