# Changelog

## 0.1.0 (preview)

- Extend the existing Terraphim It extension with the shared Alternative Control Markdown editor.
- Create, edit and swap alternatives with document-scoped KG synonym registration, native save/history and clean Markdown export.
- Export an acknowledged document's knowledge graph through the existing Terraphim Rust `synonyms::` parser and types. The resulting JSON can be passed to `terraphim-grep --thesaurus`.
- Bundle verified editor v0.1.0-rc.1 bytes. No Rust toolchain or private registry credentials are required for extension users or VSIX packaging.
- Automate package integrity, real-WASM tests and native packaged lifecycle checks before marketplace publication.
- Require VS Code 1.126 or newer for this preview's native qualification. The original 41 visual requirements are not all certified.

Existing server/Atomic commands remain. KG suggestions use the selected role's vocabulary; the shared editor has no built-in LLM alternatives provider.
