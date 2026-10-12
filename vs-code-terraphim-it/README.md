# Terraphim It — intelligent Markdown editing

Keep alternatives beside your writing, swap wording, and share their knowledge graph with search. This extends the existing `applied-knowledge-systems.terraphim-it` extension.

1. Open a Markdown file and run **Terraphim: Open Alternative Control**.
2. Select a word such as **Risk**, add **Concern** as an alternative, and choose which wording is active.
3. Save the document. VS Code owns persistence and undo/redo; the annotation block preserves the alternatives.
4. Run **Terraphim: Export Document Knowledge Graph**, or use the editor's **Share with search** dialog. Save the JSON and export clean Markdown into a corpus folder.
5. Search it with a released CLI:

```sh
terraphim-grep concern --paths ./briefs --thesaurus ./brief.knowledge-graph.json --search-only --json
terraphim-grep concern --paths ./briefs --thesaurus ./brief.knowledge-graph.json --answer --json
```

The first command retrieves matching sources. `--answer` additionally uses your configured synthesis provider. Exporting a graph is explicit: it does not automatically modify a shared role or upload your document.

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

1. **Install the preview VSIX** from a qualified build; marketplace publication is a separate release gate
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
