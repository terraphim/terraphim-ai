# Terraphim Engineer Configuration

This document describes how to set up and use the Terraphim server with the **Terraphim Engineer** role configuration that uses a **local knowledge graph** built from `./docs/src/kg` and documents from `./docs/src`.

## 🎯 Overview

The Terraphim Engineer configuration provides:

- **Local Knowledge Graph**: Built from `./docs/src/kg` markdown files during server startup
- **Local Document Integration**: Indexes all documentation from `./docs/src`
- **Advanced Search**: TerraphimGraph ranking with locally-built knowledge graph
- **Engineering Focus**: Specialized for Terraphim architecture, services, and development content

## 📋 Prerequisites

- Rust and Cargo installed
- Access to `./docs/src` directory (should be present in the Terraphim project)
- At least 1GB free disk space for indexing

## 🚀 Quick Start

### 1. Setup Terraphim Engineer Environment

Run the automated setup script:

```bash
./scripts/setup_terraphim_engineer.sh
```

This script will:
- Validate the `./docs/src` directory structure
- Count available documentation and KG files
- Verify configuration files are in place
- Display configuration information

### 2. Start the Server

```bash
cargo run --bin terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json
```

The server will start on `http://127.0.0.1:8000` by default.

**Note**: First startup may take 10-30 seconds to build the knowledge graph from local files.

### 3. Test the Configuration

Run the integration test to verify everything works:

```bash
cargo test --test terraphim_engineer_integration_test -- --nocapture
```

## 📁 Configuration Files

### Core Configuration
- **`terraphim_server/default/terraphim_engineer_config.json`** - Main server configuration
- **`terraphim_server/default/settings_terraphim_engineer_server.toml`** - Server settings

### Source Data
- **`./docs/src/kg/`** - Knowledge graph source files (3 markdown files)
- **`./docs/src/`** - Documentation source (~15 markdown files)

## 🔧 Configuration Details

### Roles Available

1. **Terraphim Engineer** (Default)
   - **Relevance Function**: `terraphim-graph` 
   - **Theme**: `lumen` (light theme)
   - **Local KG**: ✅ Built from `./docs/src/kg`
   - **Local Docs**: ✅ `./docs/src`

2. **Engineer**
   - **Relevance Function**: `terraphim-graph`
   - **Theme**: `lumen` (light theme)  
   - **Local KG**: ✅ Built from `./docs/src/kg`
   - **Local Docs**: ✅ `./docs/src`

3. **Default**
   - **Relevance Function**: `title-scorer`
   - **Theme**: `spacelab`
   - **Local KG**: ❌ Disabled
   - **Local Docs**: ✅ `./docs/src`

### Local Knowledge Graph Details

- **Source**: `./docs/src/kg/*.md` files
- **Build Time**: During server startup (10-30 seconds)
- **Content**: 
  - `terraphim-graph.md` - Graph architecture concepts
  - `service.md` - Service definitions
  - `haystack.md` - Haystack integration
- **Performance**: Fast search once built, no external dependencies

### Document Collection Details

- **Source**: `./docs/src/*.md` files  
- **Count**: ~15 documentation files
- **Content**: 
  - Architecture documentation
  - API guides
  - Use cases and examples
  - Development guides
- **Access**: Read-only for development safety

## 🔍 API Usage Examples

### Health Check
```bash
curl http://127.0.0.1:8000/health
```

### Search with Terraphim Engineer Role
```bash
curl "http://127.0.0.1:8000/documents/search?q=terraphim&role=Terraphim%20Engineer&limit=5"
```

### Get Configuration
```bash
curl http://127.0.0.1:8000/config
```

### Search for Specific Terms
```bash
# Service architecture
curl "http://127.0.0.1:8000/documents/search?q=service&role=Terraphim%20Engineer"

# Haystack integration
curl "http://127.0.0.1:8000/documents/search?q=haystack&role=Terraphim%20Engineer"

# Graph concepts
curl "http://127.0.0.1:8000/documents/search?q=graph&role=Terraphim%20Engineer"
```

## 🧪 Testing

### Run Integration Tests
```bash
# Run Terraphim Engineer specific tests
cargo test --test terraphim_engineer_integration_test -- --nocapture

# Run all server tests
cargo test -p terraphim_server -- --nocapture
```

### Manual Testing
1. Start the server
2. Open `http://127.0.0.1:8000` in browser
3. Search for terms like "terraphim", "service", "haystack"
4. Verify results are ranked by knowledge graph relevance

## 📊 Expected Performance

### Startup Time
- **Local KG Build**: ~10-30 seconds (depends on content size)
- **Document Indexing**: ~1-3 seconds for 15 files
- **Total Startup**: ~30-45 seconds

### Search Performance
- **Knowledge Graph Search**: <50ms for most queries
- **Document Count**: ~15 markdown files
- **Index Size**: ~5MB in memory

### Sample Search Results

When searching for "service":
- Service architecture documentation
- API service definitions
- Integration guides
- Configuration examples

## 🛠️ Troubleshooting

### Common Issues

1. **KG Build Failed**
   ```bash
   # Check if KG files exist
   ls -la docs/src/kg/
   
   # Verify markdown files
   find docs/src/kg -name "*.md"
   ```

2. **No Search Results**
   - Ensure documents are in `./docs/src/`
   - Check server logs for indexing errors
   - Verify role has `terraphim-graph` relevance function

3. **Server Won't Start**
   ```bash
   # Check if port is in use
   lsof -i :8000
   
   # Use different port
   cargo run --bin terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json --addr 127.0.0.1:8080
   ```

4. **Knowledge Graph Not Building**
   - Ensure `./docs/src/kg/` directory exists
   - Check that markdown files contain valid content
   - Look for build errors in server logs

### Debug Mode
```bash
RUST_LOG=debug cargo run --bin terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json
```

## 🔄 Updating Documentation

To refresh the Terraphim documentation:

1. Edit files in `./docs/src/` or `./docs/src/kg/`
2. Restart the server to rebuild the knowledge graph
3. New content will be automatically indexed

## 🌐 Development Deployment

### Environment Variables
```bash
export TERRAPHIM_SERVER_HOSTNAME="127.0.0.1:8000"
export TERRAPHIM_SERVER_API_ENDPOINT="http://localhost:8000/api"
```

### Docker Development
```bash
# Build with Terraphim Engineer config
docker build -t terraphim-engineer .

# Run with mounted docs
docker run -p 8000:8000 \
  -v $(pwd)/docs:/app/docs:ro \
  terraphim-engineer
```

## 📊 Content Analysis

### Knowledge Graph Files
- **terraphim-graph.md**: 352 bytes, 11 lines - Graph architecture concepts
- **service.md**: 52 bytes, 3 lines - Service definitions  
- **haystack.md**: 49 bytes, 2 lines - Haystack integration

### Documentation Files
- **Introduction.md**: 7.5KB, 44 lines - Main introduction
- **ClaudeDesktop.md**: 2.5KB, 56 lines - Claude integration
- **Architecture.md**: 120 bytes, 8 lines - System architecture
- **Use-cases.md**: 1.7KB, 28 lines - Usage examples
- And more...

## 🔍 Search Optimization

### Best Search Terms
- **"terraphim"** - General Terraphim content
- **"service"** - Service architecture and APIs
- **"haystack"** - Document processing and integration
- **"graph"** - Knowledge graph and architecture
- **"architecture"** - System design and structure

### Knowledge Graph Enhanced Terms
The following terms benefit from KG ranking:
- terraphim-graph
- service architecture
- haystack integration
- graph embeddings

## 📚 Related Documentation

- [Terraphim Configuration Guide](../docs/src/Configuration.md)
- [Knowledge Graph Documentation](../docs/src/kg/)
- [API Reference](../docs/src/API.md)
- [System Architecture](../docs/src/Architecture.md)

## 🤝 Contributing

To improve the Terraphim Engineer configuration:

1. Edit documentation in `./docs/src/`
2. Update knowledge graph files in `./docs/src/kg/`
3. Test with the integration test suite
4. Submit a pull request

## 📄 License

This configuration is part of the Terraphim project and follows the same Apache 2.0 license. 