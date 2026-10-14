# Terraphim for Continue

Connect the existing Continue extension to released `terraphim-grep` using the official Python MCP SDK. No Continue rebuild, new search engine, or global model configuration is needed. This integration supplies **Agent-mode tools**; it does not add editor panels or autocomplete, and native Continue interaction has not yet been qualified.

## Distribution and Continue compatibility

Use this project-local integration with the existing Continue extension. [Continue's final VS Code release is 2.0.0](https://github.com/continuedev/continue/releases/tag/v2.0.0-vscode). The [upstream repository](https://github.com/continuedev/continue) says it is no longer actively maintained, and [Continue has joined Cursor](https://continue.dev/). On 13 October 2026, the former Hub hostname did not resolve and its publishing-action repository returned 404. A Continue Hub listing is therefore not an available distribution route verified by this project.

The dedicated GitHub release workflow below packages this adapter and its installer; it does not publish a second copy of Continue or require a Hub account. Continue's current [MCP documentation](https://docs.continue.dev/customize/deep-dives/mcp#using-json-mcp-format-from-claude-cursor-cline-etc) supports the generated project-local JSON configuration. The MCP JSON parser and conversion files from `v2.0.0-vscode` match the source hashes in the existing schema proof. This verifies configuration compatibility, not a native Continue UI session. The released CLI and actual MCP transport are tested separately below.

`terraphim_retrieve` executes `--search-only` and returns the actual CLI JSON, source excerpts, argv, and binary/graph digests. An editor-exported graph can make **Concern** retrieve a document containing **Risk**. The server pins graph bytes until restarted, so review a newly exported graph and restart Continue's MCP connection to adopt it. The corpus remains live. The corpus directory is the retrieval scope, not an OS security sandbox; choose an owned public corpus and avoid secrets or symlinked private content.

`terraphim_answer` is absent by default. Install with `--allow-synthesis` to expose this explicit `--answer` tool. It uses the provider already configured for the CLI and can send retrieved context to that provider. The integration does not store credentials in Continue configuration. Provider credentials must already be available to the Continue-launched process; shell credentials are not necessarily inherited by a GUI. Continue's own Agent model can also synthesize retrieval results, so retrieval-only here describes the Terraphim tool, not all model traffic in Continue.

## Install

Keep this extracted bundle in a stable location. Use Python 3.11 or newer on macOS or Linux (including WSL) and the released grep executable (1.21.19 has the authored-synonym retrieval fixes). Verify its downloaded archive using the release SHA256SUMS, then hash the extracted executable.

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python install.py --project /absolute/project \
  --binary /absolute/terraphim-grep --binary-sha256 YOUR_VERIFIED_EXECUTABLE_SHA256 \
  --graph /absolute/risk-concern.knowledge-graph.json --corpus /absolute/public-corpus
```

This creates only `.continue/mcpServers/terraphim.json` and `.continue/rules/terraphim.md` in that project; existing files are refused and existing model settings retained. The generated config uses this venv's Python and the bundle's absolute path. Open the project in the existing Continue extension, select Agent mode and inspect its MCP tool list. Ask it to call `terraphim_retrieve` for `Concern`, then inspect the returned Risk excerpt. Explicitly request the answer tool only if installed and desired. Do not treat a model's narrative as proof that a tool ran.

## Use both CLIs directly

The integration calls `terraphim-grep`; `terraphim-agent` is a separate CLI for memory and learning. Install both separately from the [clients v1.21.20 release](https://github.com/terraphim/terraphim-clients/releases/tag/v1.21.20): select your platform archives, verify `SHA256SUMS`, extract them and add the executables to `PATH`. Neither CLI is bundled here. The commands below were checked with **1.21.20** on macOS ARM64; the existing MCP transport CI separately pins grep 1.21.19.

```sh
terraphim-grep --version
terraphim-agent --version
```

The shared concept uses Logseq-compatible `synonyms::` syntax in a Markdown knowledge-base note:

```markdown
# Risk
synonyms:: Concern
```

The adapter and CLI consume its compiled JSON graph. Run this example in a fresh working directory. It uses the graph exported from the editor, included as `risk-concern.exported.json` in this bundle. Copy that file into the working directory first. The clean corpus contains Risk but no literal Concern:

```sh
mkdir briefs
printf '%s\n' 'Risk: supplier delays require a backup plan.' > briefs/decision.md
terraphim-grep concern --search-only --haystack all \
  --paths ./briefs --thesaurus ./risk-concern.exported.json --json -n 3
```

The returned `chunks` contain the Risk sentence and `answer` is `null`. To adopt a newly authored synonym, review the editor export, replace the file at the configured `--graph` path, and restart the MCP server. The running server refuses changed graph bytes; a new process records the reviewed graph's digest at startup. If you use a different graph path, update `--graph` in `.continue/mcpServers/terraphim.json` first. The installer refuses existing configuration rather than overwriting it. The [synonym-search tutorial](https://terraphim.ai/how-tos/synonym-search/) includes the editor steps, video and downloadable examples.

To compare retrieval with provider synthesis on a corpus with several relevant passages, run the same search with `--answer` instead of `--search-only`. That requires a configured provider and can send the retrieved context to it; inspect the returned passages, answer and citations separately. The Continue adapter exposes this mode only when installed with `--allow-synthesis`.

Preserve an authored decision using agent memory, then read the actual saved item (Python 3 extracts the returned ID):

```sh
terraphim-agent --format json memory capture \
  --content 'Risk requires a Decision: approve a second supplier as the mitigation.' \
  --provenance-tag 'public-project-brief' > captured-memory.json
memory_id="$(python3 -c 'import json; print(json.load(open("captured-memory.json"))["memory_id"])')"
terraphim-agent memory show "$memory_id" --json
```

The readback's `memory_item.content` contains that sentence, with the `provenance:public-project-brief` tag. Capture writes to the current user's OS-local memory store (independent of the working directory). It does not consume the exported graph, configure a retrieval role or add a memory tool to this MCP adapter. Follow the [memory walkthrough](https://terraphim.ai/posts/cold-start-tax-agentic-memory/) for role-aware retrieval and the [hooks walkthrough](https://terraphim.ai/posts/native-hook-support/) for reviewed command corrections.

## Verification and packaging

```sh
TERRAPHIM_GREP_BINARY=/absolute/released/terraphim-grep \
TERRAPHIM_EXPORTED_GRAPH=/absolute/editor-exported.json \
TERRAPHIM_CONTINUE_RECEIPT=/absolute/new-receipt.json \
.venv/bin/python -m unittest discover -s .
```

The actual SDK client initializes the actual stdio server and calls the released CLI. It verifies literal Concern baseline has no chunks, the exported graph retrieves Risk, retrieval has no answer, synthesis is unavailable by default, invalid parameters and changed graph are refused. Installer tests execute the real installer and verify preservation and symlink refusal. No provider synthesis or native Continue UI acceptance is inferred from these tests.

`risk-concern.exported.json` is the exact public graph exported by the existing VS Code editor in the 2026-10-12 native proof; it is a reusable example input, not evidence of a new operation. The integration CI downloads the immutable 1.21.19 Linux archive, verifies its pinned digest, runs these tests, and produces a separately downloadable zip and SHA256SUMS. A dedicated `terraphim_continue-v0.1.1` tag runs the checks before publishing its own two-asset integration release; it never attaches to VS Code releases. The bundle records its source commit, clean-tree state and payload hashes. Immediately before each release mutation, the workflow verifies the live remote peeled tag still equals the qualified source and belongs to main history. Existing release assets must match exactly; partial uploads fail closed for inspection. The release job uses immutable action versions, a `continue-releases` environment, per-tag concurrency, and mandatory anonymous exact-byte download verification after publication.

We inspected the existing clients `terraphim_mcp_server`: it is a broader role-configured `TerraphimService` server, not installed in this environment, and its search path does not delegate the exact released grep exported-graph/`--answer` behavior. This thin transport reuses the released CLI instead of changing that service or reimplementing KG semantics. The maintained MCP v1 SDK 1.30.0 is pinned to match the existing stdio API; v2 is available but requires a separate compatibility qualification.

References: [Continue MCP configuration](https://docs.continue.dev/customize/deep-dives/mcp), [Continue rules](https://docs.continue.dev/customize/deep-dives/rules), [official MCP Python SDK](https://github.com/modelcontextprotocol/python-sdk).

## Release notes

### 0.1.1

- Include runnable examples for the separately installed `terraphim-grep` and `terraphim-agent` CLIs, including Logseq-compatible `synonyms::` concept Markdown.
- Clarify graph updates: a running connection refuses changed graph bytes; reviewing the replacement and restarting captures the new graph digest.
- Verify that behavior with the actual MCP transport and released `terraphim-grep` output.
