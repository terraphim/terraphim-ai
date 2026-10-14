# Marketplace CLI examples — actual output verification

The VS Code package README and Continue bundle README now contain runnable examples for both `terraphim-grep` and `terraphim-agent`, with separate installation instructions, clients v1.21.20 links, `synonyms::` concept Markdown, and the public search, memory and hooks tutorials. This is a documentation amendment; extension/integration versions and runtime code are unchanged.

## Executed examples

Both new README command blocks were executed on macOS ARM64 using exact released clients **1.21.20**, with isolated child profiles so authored fixture captures did not enter the user's memory. The user-facing commands intentionally use the current user's OS-local memory store; their working directory does not establish memory isolation.

- The existing editor-exported `integrations/continue/risk-concern.exported.json` was supplied as an input. It is not a newly authored graph from these runs.
- The corpus contains only `Risk: supplier delays require a backup plan.` An explicit empty graph returns zero passages; the exported Risk/Concern graph returns one actual Risk passage for `concern` with `answer: null`.
- Both exact capture/Python-ID-extraction/show blocks persisted and read back `Risk requires a Decision: approve a second supplier as the mitigation.` with the `provenance:public-project-brief` tag and matching generated ID. Capturing authored text does not import the editor graph or configure an agent retrieval role.
- The VS Code `--answer` example was also executed on the single-passage corpus and correctly returned no generated answer. The README explains this insufficient-evidence result and that synthesis requires a configured provider. These runs do not claim provider synthesis; the linked tutorial shows the fuller comparison.

[receipt.json](receipt.json) preserves exact shell blocks, binary versions/digests and output filenames. Its original execution-document hashes are retained; `finalDocumentSha256` additionally binds the final explanatory prose, and every executed block was verified unchanged. The adjacent stdout/JSON files are actual outputs. Empty stderr files are intentional; grep's harmless fresh-directory/no-Git warning is retained where emitted.

## Existing integration tests

All six Continue tests passed, including real SDK/CLI transport, installer preservation, package determinism and immutable tag/source checks. The existing real transport test now also proves graph-restart behavior: changing graph bytes is refused by the running process; restarting the same server arguments admits the reviewed graph's new digest and retrieves Risk for the added Hazard alias. The changed graph is a labelled test input, not an editor operation. See [continue-transport.json](continue-transport.json) and [continue-tests.txt](continue-tests.txt).

The binary digest is explicitly pinned in generated Continue arguments. The graph digest is captured at process startup. A new graph path requires an explicit config edit; the installer refuses existing configuration. No existing profile or model configuration was overwritten.

## Publication scope

Microsoft/Open VSX consume the packaged VS Code README. Continue packages its own README via the explicit existing allowlist, and future CI release notes now reference both separately installed CLIs. A repository merge alone does not update existing marketplace package bytes. A subsequent versioned CI release, registry readback and package inspection are required; this change does not republish or replace any immutable release.
