# Terraphim IT - VS Code Extension

An intelligent VS Code extension that integrates with Terraphim AI to provide semantic search and AI-powered autocomplete functionality for your code and documents.

## Features

- **Terraphim IT**: Apply current Terraphim AI role to replace links in text
- **AI-Powered Autocomplete**: Multiple autocomplete providers with intelligent suggestions
- **Server Integration**: Connect to local or remote Terraphim servers
- **Health Monitoring**: Built-in server health check functionality
- **Configurable Endpoints**: Flexible server and knowledge base configuration

## Available Commands

- `Terraphim IT` - Replace links in the current document using Terraphim AI
- `Terraphim AI Autocomplete` - Enable atomic store-based autocomplete
- `Terraphim Napi Autocomplete` - Enable Node.js binding-based autocomplete with server fallback
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

1. **Install the extension** from the VS Code marketplace or load from VSIX
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

### Terraphim Napi Autocomplete
Uses Node.js bindings with server fallback:
- Fast local search using native bindings
- Automatic fallback to server if local search fails
- Best of both local performance and server capabilities

### Terraphim AI Autocomplete
Uses atomic store integration:
- Direct connection to knowledge graph
- Requires agent configuration for private knowledge bases

## Usage

### Basic Link Replacement
1. Open a document in VS Code
2. Run the `Terraphim IT` command
3. The extension will replace links based on your configured role and knowledge base

### Autocomplete Usage
1. Enable your preferred autocomplete provider
2. Start typing in any file
3. Autocomplete suggestions will appear as you type
4. Suggestions are triggered after 2+ characters and on space

### Server Health Check
Use the health check command to verify:
- ✅ Terraphim Server connectivity
- ✅ Atomic Server accessibility
- ❌ Any connection issues with detailed error messages

## Development

### Building from Source
```bash
# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Build Rust components
npm run build-rust

# Watch for changes during development
npm run watch
```

### Running Tests
```bash
# Compile and test
npm run compile
```

## Server Requirements

### Local Server
- Terraphim server running on configured port (default: 8000)
- Server should expose `/health`, `/documents/search`, and `/autocomplete` endpoints

### Remote Server
- Accessible Terraphim server URL
- Proper CORS configuration for VS Code extension access
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

MIT License - see LICENSE file for details.
