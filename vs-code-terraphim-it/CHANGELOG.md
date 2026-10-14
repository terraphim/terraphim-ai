# Changelog

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
