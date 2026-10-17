# Alternative Control in the existing Terraphim It extension


Recoverable host notices can be dismissed while editing continues. Conflict recovery and fatal errors remain blocking so an unacknowledged draft cannot overwrite the native document. Released version 0.1.6 does not fix grouped native Undo or whole-alternative Escape cancellation ([#995](https://github.com/terraphim/terraphim-ai/issues/995)).

This preview is an opt-in custom Markdown editor in `applied-knowledge-systems.terraphim-it`. Open a Markdown file with **Terraphim: Open Alternative Control**. Click the word counter for Write_On. Existing server and Atomic commands remain available in trusted workspaces.

The view bundles the released terraphim-editor v0.1.0-rc.2 UI, WASM and source-pinned host bridge. The bundle records its public source repository and Cargo.lock checksum. Each view has its own session. The next source candidate uses native custom-document persistence and transports checkpoints of the existing editor history across views; qualification is still in progress. Exact source/revision checks protect asynchronous proposals. External edits retain conflicting drafts for recovery. Native export refuses the source itself and dirty output buffers. Browser persistence is disabled in this host.

**Export Document Knowledge Graph** waits for the current edit to be acknowledged, reads that exact source, and uses the shared Rust `synonyms::` parser/types to create the portable graph. It refuses malformed annotations or ambiguous concepts. The document and active role are unchanged. Save, undo, redo and reopen determine which aliases exist in the export.

## Companion command-line tools

The [packaged README](https://github.com/terraphim/terraphim-ai/blob/main/vs-code-terraphim-it/README.md#use-the-graph-with-terraphim-grep-and-terraphim-agent) includes runnable examples for both separately installed CLIs: `terraphim-grep` consumes the exported synonym graph, and `terraphim-agent` captures authored text and reads the saved memory item by its returned ID. The commands are checked against [clients v1.21.20](https://github.com/terraphim/terraphim-clients/releases/tag/v1.21.20). Neither executable is bundled with the extension, and agent memory capture does not import a graph or configure a role.

The same packaged README supplies the Microsoft Marketplace and Open VSX listing content, including compatible Cursor installs. README changes reach those listings with the next qualified VSIX release; updating repository documentation alone does not update an existing immutable package. The Continue integration ships its own [README and direct CLI examples](https://github.com/terraphim/terraphim-ai/tree/main/integrations/continue#use-both-clis-directly) through its separate release workflow.

## Building and publication

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

Microsoft publication also requires `VSCE_MARKETPLACE_PROFILE_ID` in the existing
`visual-studio-marketplace` environment. After federated login, CI verifies the
service principal client and tenant, then queries its own Azure DevOps profile.
The profile ID is distinct from the Entra application and enterprise object IDs.
An empty or different expected profile ID stops the job before upload and retains
only non-secret metadata in `vscode-marketplace-identity-<commit>-<attempt>`.
That receipt proves identity admission, not publisher membership or publication.

For initial setup, enable the Microsoft readiness gate only after the existing
publisher and its federation are ready. Leave the profile ID unset for the first
identity read. Use the returned profile ID to add that principal as **Contributor**
to `applied-knowledge-systems`; verify the assignment, then configure the expected
profile ID. Rerun only that release's `marketplace-plan` job and its dependent
marketplace job to use the already qualified package. Verify the final registry
receipt against every extension payload file. Do not rerun package qualification
or dispatch the same version afresh: a rebuilt VSIX can have different ZIP
timestamps, and the immutable GitHub publisher will refuse replacement bytes.
The original source tag, package artifact and three native receipts must remain
available; select retained receipts by artifact ID and digest, not name alone.

Version 0.1.2 introduces this CI-only identity step because the immutable 0.1.1
tag cannot be changed. It retains the same released editor inputs and runtime
features. Real Azure identity retrieval remains an integration gate until the
configured principal executes it; local payload fixtures do not qualify login.
This step assigns no Azure resource roles or organization membership and creates
no long-lived secrets. See Microsoft's documented
[principal profile lookup](https://learn.microsoft.com/en-us/azure/devops/extend/publish/command-line?view=azure-devops#publish-with-a-microsoft-entra-token).

Acceptance reports distinguish actual saved files, engine tests and visual UI checks. The original 41 visual requirements are not all accepted. This is a scoped preview; stock Zed visual limitations and the separate Zed fork remain outside this VSIX's claims.

## Alternative Control history (next candidate; qualification pending)

Use the editor keyboard Undo/Redo shortcuts or the accessible **Terraphim: Undo (Alternative Control)** and **Terraphim: Redo (Alternative Control)** title/command-palette controls. These commands are scoped to this custom view. Native Edit-menu Undo/Redo does not operate Alternative Control; ordinary Markdown editors retain their native history.

Typing a new alternative remains live. Enter or leaving keeps it as one action; Escape cancels the complete addition without placing it in Redo. Saving uses native Save, Save All, autosave and backup hooks. The native dirty marker can remain after Undo returns to saved bytes until Save/Revert/autosave completes. Recovered source includes annotations; prior-session Undo history is not restored.

Other custom/text views and external filesystem edits cannot silently replace a retained draft. Resolve a reported conflict explicitly or open a recovery draft. Checkpoint transport stages bounded chunks separately from source proposals and retains the existing Rust source/revision/hash guards. A capacity refusal retains the local draft; it does not silently trim history. Issue #995 stays open until exact released packages pass the approved contract.
