# Extension publication research — 12 October 2026

This continues the approved editor publication design and existing extension identities. The user explicitly confirmed Continue AI integration. The Zed application fork remains outside this work.

## Existing implementation and evidence

- Existing VS Code extension: `vs-code-terraphim-it`, publisher/name `applied-knowledge-systems.terraphim-it`. A native Markdown CustomTextEditorProvider owns persistence/history; the released editor's Rust/WASM supplies alternatives and document synonyms. Shared browser assets come from editor v0.1.0-rc.1, source `05d0aa50151c131db4a3b30b56aba67c00add5fc`.
- Existing Zed adapter: `terraphim/zed-terraphim`, version 0.2.0, ID `terraphim`. It provides Markdown LSP plus one MCP context server, with verified download/cache provisioning. LSP pin rc.3 exists on the internal Gitea release channel; anonymous GitHub download of that exact pin returns 404. GitHub lookup for `terraphim/zed-terraphim` returns 404. Neither is public-installation evidence.
- Existing Continue fork already supports project `.continue/mcpServers` configuration in Agent mode. Reuse this capability. Do not rebuild Continue or install a second Terraphim editor. The existing Terraphim MCP service uses role/service search; it does not provide the exact released grep graph-file/synthesis contract. A narrow MCP adapter may delegate to released grep without implementing another search engine.
- Document-created synonyms are represented by the released Terraphim alternatives model and `synonyms::` parser/types. Explicit export produces a typed `Thesaurus`. Export is document-scoped; it does not silently change a shared role.
- VS Code local package tests: 25 real WASM/host/filesystem checks passed. Native installed package saved risk/concern Markdown and graph JSON; the graph hash is `f8648b16470e764bdff5c95064078419d71e8d41d2ca30d4b1508017c166a45d`. Released grep 1.21.19 searched that actual file and returned three risk decisions for concern.
- The initial local native attempt failed at the locked-desktop focus precondition. Subsequent packaged CI run 38216701669 passed the document/save/history assertions on Linux, macOS and Windows; actual output files were downloaded and hashed. The saved graph comes from the packaged host helper and real TextDocument save. Export-dialog and webview UI acceptance remain distinct.

## Marketplace contracts

| Destination | What is published | Required control |
|---|---|---|
| Visual Studio Marketplace | Existing publisher/extension identity; preview VSIX | Publisher membership, Entra federation, exact package qualification |
| Open VSX | Same identity and VSIX for compatible VS Code hosts | Namespace ownership, publisher agreement, trusted publisher mapping |
| Zed extensions catalog | Public extension source at a tested immutable commit | Manifest policy, manual native qualification, catalog PR and maintainer merge |
| GitHub/Gitea releases | VSIX, Zed archive, Continue integration and receipts | Reviewed immutable tags, checksums and anonymous readback |
| Continue / Continue Hub | Versioned integration bundle and reusable MCP configuration block | Agent-mode interoperability; namespace ownership and a verified publishing route for Hub |

VS Code recommends workload identity federation for automated publishing. A pre-release package must be marked as such, retain its existing publisher/name and obey the host engine floor. [Official VS Code publishing guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).

Open VSX requires publisher/namespace setup; uploading a VSIX or configuring CI does not itself establish ownership. [Official Open VSX publishing guide](https://github.com/eclipse-openvsx/openvsx/wiki/Publishing-Extensions).

Zed's current prerequisites require an MCP extension to provide one MCP server and nothing else; a language-server-only ID must identify that role. The existing combined manifest cannot be submitted unchanged. A source/catalog PR is not an installed marketplace listing. [Zed prerequisites](https://zed.dev/docs/extensions/publishing/prerequisites), [publishing guide](https://zed.dev/docs/extensions/publishing/publishing-guide).

## Constraints and decisions

1. Preserve installed extension identities and saved document format. A new catalog LSP identity requires an explicit migration path and separate package; do not silently remove existing MCP behavior.
2. Publicize only the reviewed extension/integration source. Do not mirror unrelated private monorepo history to make CI convenient.
3. Stock Zed uses completion, hover, code actions, diagnostics, inlay hints and saved annotations. Side panels, drag interaction and browser rendering are not full visual parity. Keep the 41-check host matrix honest.
4. Alternative suggestions in the released editor are KG-backed by default. A provider API exists, but that does not establish a configured LLM service.
5. CI checks package bytes, source identity and actual output. Missing platform evidence, missing binary releases and account configuration must fail publication, not become skipped green gates.

## Continue Hub distribution

Continue supports shared MCP blocks imported with `uses: owner/item` and configurable inputs. A Terraphim block should accept users' local binary, graph and corpus paths and reference the qualified versioned integration; never publish local development paths or credentials. [Official configuration guide](https://docs.continue.dev/guides/configuring-models-rules-tools).

The previously documented `continuedev/continue-publish-action` repository/API and raw action source returned HTTP 404 during this research. Cached examples are not a verified automation contract. The existing Continue fork supplies registry reads and schema validation, not a supported publishing CLI. Hub publication therefore requires a currently supported API/action and Terraphim namespace access before CI can own the upload. Acceptance must resolve the public versioned block through Continue's real registry loader and repeat Concern → Risk retrieval. [Official action repository](https://github.com/continuedev/continue-publish-action).
