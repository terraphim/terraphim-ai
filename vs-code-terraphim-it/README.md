# Terraphim It — intelligent Markdown editing

Keep alternatives beside your writing, swap wording, and share their knowledge graph with search. This extends the existing `applied-knowledge-systems.terraphim-it` extension.

1. Open a Markdown file and run **Terraphim: Open Alternative Control**.
2. Select a word such as **Risk**, add **Concern** as an alternative, and choose which wording is active.
3. Save the document. VS Code owns persistence and undo/redo; the annotation block preserves the alternatives.
4. Run **Terraphim: Export Document Knowledge Graph**, or use the editor's **Share with search** dialog. Save the JSON and export clean Markdown into a corpus folder.
5. Use the exported graph with `terraphim-grep`, then preserve a useful decision with `terraphim-agent` as shown below.

Exporting a graph is explicit: it does not automatically modify a shared role or upload your document.

## Use the graph with terraphim-grep and terraphim-agent

Install the two CLIs separately from the [Terraphim clients v1.21.20 release](https://github.com/terraphim/terraphim-clients/releases/tag/v1.21.20). Choose the archives for your operating system, verify them against that release's `SHA256SUMS`, extract them and put the executables on your `PATH`. The extension bundles its editor engine; it does not install either CLI. These examples were checked with both CLIs at **1.21.20** on macOS ARM64. Run the following in a terminal; the memory readback also uses Python 3.

```sh
terraphim-grep --version
terraphim-agent --version
```

### Search by an alternative

For a small example, write `Risk: supplier delays require a backup plan.` in the editor, add **Concern** as an alternative, and keep **Risk** active. Export the graph as `brief.knowledge-graph.json` and clean Markdown as `briefs/decision.md`. Keep only the clean brief in `briefs`, so a hit for Concern comes from the synonym graph rather than annotation text.

The concept can also be written as a knowledge-base note using Logseq-compatible `synonyms::` syntax:

```markdown
# Risk
synonyms:: Concern
```

The editor export is the compiled JSON representation used by the search command.

```sh
terraphim-grep concern --search-only --haystack all \
  --paths ./briefs --thesaurus ./brief.knowledge-graph.json --json -n 3
```

The returned `chunks` include the Risk sentence, while `answer` is `null`. See the [editor → synonym → search walkthrough](https://terraphim.ai/how-tos/synonym-search/) for the video and downloadable examples.

For a corpus with several relevant passages, request synthesis explicitly:

```sh
terraphim-grep concern --answer --haystack all \
  --paths ./briefs --thesaurus ./brief.knowledge-graph.json --json -n 3
```

`--answer` uses your configured synthesis provider and can send retrieved context to it. Inspect the returned passages and citations alongside the answer. A small or unmatched corpus can return no generated answer.

### Save a decision in agent memory

`terraphim-agent` can retain authored text for later readback. This command writes to the current user's OS-local memory store (independent of the working directory); it does not import the editor's graph or configure a retrieval role.

```sh
terraphim-agent --format json memory capture \
  --content 'Risk requires a Decision: approve a second supplier as the mitigation.' \
  --provenance-tag 'public-project-brief' > captured-memory.json
memory_id="$(python3 -c 'import json; print(json.load(open("captured-memory.json"))["memory_id"])')"
terraphim-agent memory show "$memory_id" --json
```

The readback's `memory_item.content` contains the authored sentence and its tags include `provenance:public-project-brief`. Use the returned ID, rather than copying an ID from an example. The [agent memory walkthrough](https://terraphim.ai/posts/cold-start-tax-agentic-memory/) explains retrieval and role setup; the [hooks walkthrough](https://terraphim.ai/posts/native-hook-support/) covers reviewed command corrections and actual tool execution.

This preview targets desktop VS Code 1.126+. Editing and KG export run locally using bundled Terraphim WASM. Suggestions from the built-in graph are synonyms, not generated LLM rewrites. [Integration details and current acceptance scope](README-alternative-control.md).

## Existing server and Atomic commands


Existing commands connect to your Terraphim server or Atomic knowledge base. Their suggestions come from configured knowledge-graph resources.

## Features

- **Terraphim IT**: Link known terms using your Atomic resources and bundled Terraphim automata
- **Knowledge-graph autocomplete**: Suggestions from your configured server or Atomic store
- **Server Integration**: Connect to local or remote Terraphim servers
- **Health Monitoring**: Built-in server health check functionality
- **Configurable Endpoints**: Flexible server and knowledge base configuration

## Available Commands

- `Terraphim IT` - Replace links in the current document using Terraphim AI
- `Terraphim AI Autocomplete` - Enable atomic store-based autocomplete
- `Terraphim Search Autocomplete (Server)` - Preserve the legacy NAPI command ID using the existing server provider
- `Terraphim Server Autocomplete` - Enable pure server-based autocomplete
- `Terraphim Server Health Check` - Check server connectivity and health

## Configuration

Configure the extension through VS Code settings:

### Server Configuration
- `terraphimIt.serverUrl`: Main Terraphim server URL (default: `http://localhost:8000`)
- `terraphimIt.atomicServerUrl`: Atomic server URL for knowledge base (default: `https://common.terraphim.io/drive/h6grD0ID`)
- `terraphimIt.enableLocalServer`: Use local server instead of remote (default: `true`)

### Role and Agent Configuration
- `terraphimIt.selectedRole`: Default role for searches and autocomplete (default: `Terraphim Engineer`)
- `terraphimIt.agent`: Agent string for authentication (optional)

## Getting Started

1. **Install Terraphim It** from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=applied-knowledge-systems.terraphim-it) or [Open VSX](https://open-vsx.org/extension/applied-knowledge-systems/terraphim-it). Compatible Cursor installations use the Open VSX package.
2. **Configure your server** settings in VS Code preferences
3. **Start your Terraphim server** (if using local server):
   ```bash
   cd terraphim-ai
   cargo run --release
   ```
4. **Check server health** using the `Terraphim Server Health Check` command
5. **Enable autocomplete** using one of the autocomplete commands

## Autocomplete Providers

### Terraphim Server Autocomplete (Recommended)
Uses the Terraphim server's autocomplete and search endpoints:
- Tries autocomplete endpoint first for fast suggestions
- Falls back to search endpoint for broader results
- Requires server to be running and healthy

### Terraphim Search Autocomplete (Server)
Retains the existing `extension.terraphimNapiAutocomplete` command ID for saved keybindings. It uses the same server provider and health check as Server Autocomplete. An unavailable server returns failure and does not claim activation. No platform-specific Node binding is required.

### Terraphim AI Autocomplete
Uses atomic store integration:
- Direct connection to knowledge graph
- Requires agent configuration for private knowledge bases

## Usage

### Basic Link Replacement
1. Open a document in VS Code
2. Run the `Terraphim IT` command
3. The bundled Rust/WASM engine replaces known terms with links from your Atomic knowledge base. Invalid graph data or a document changed during loading is refused without applying stale or empty replacement text.

### Autocomplete Usage
1. Enable your preferred autocomplete provider
2. Start typing in any file
3. Autocomplete suggestions will appear as you type
4. Suggestions are triggered after 2+ characters and on space

### Server Health Check
Use the health check command to verify:
- ✅ Terraphim Server connectivity
- ❌ Any connection issues with detailed error messages

## Development

### Building from Source

Use Node 22, Python 3.11+ and rustup. Extension users do not need Rust. Builders use the checked-in Rust 1.96.1 toolchain, wasm-pack 0.13.1 and locked public crates.io dependencies.

```sh
npm ci --ignore-scripts
npm run editor:fetch
(cd rust-lib && rustup show active-toolchain && cargo install wasm-pack --version 0.13.1 --locked)
npm run build-rust
npm run compile
npm run lint
npm run test:assets
npm run test:host
npm run test:export
npm run test:legacy
npm run package:vsix
npm run test:native
```

The editor importer verifies published hashes. The existing link wrapper reuses pinned `terraphim_automata` 1.20.4; its build receipt binds the actual WASM bytes to the Rust sources and Cargo.lock. Packaging refuses missing or stale assets. CI builds one VSIX and tests that same artifact on Linux, macOS and Windows before release.

## Server Requirements

### Local Server
- Terraphim server running on configured port (default: 8000)
- Server should expose `/health`, `/documents/search`, and `/autocomplete` endpoints

### Remote Server
- Accessible Terraphim server URL
- Valid atomic server URL for knowledge base access

## Troubleshooting

### Common Issues

**"Server is not available" error:**
- Check that your Terraphim server is running
- Verify the server URL in settings
- Use the health check command to diagnose connectivity

**Autocomplete not working:**
- Ensure you've activated an autocomplete provider
- Check that you're typing at least 2 characters
- Verify server connectivity if using server-based providers

**Authentication errors:**
- Configure the `agent` setting if accessing private knowledge bases
- Ensure your agent string has proper permissions

**Performance issues:**
- Try different autocomplete providers to find the best performance
- Consider using local server instead of remote for better responsiveness

### Debug Information

Check the VS Code Developer Console (Help > Toggle Developer Tools) for detailed logging:
- Server connection attempts
- Search and autocomplete requests
- Error messages and stack traces

## Support

For issues and questions:
- Check the [Terraphim AI repository](https://github.com/terraphim/terraphim-ai)
- Review server logs for connectivity issues
- Use the health check command for diagnostic information

## License

Apache License 2.0 — see LICENSE.txt.
