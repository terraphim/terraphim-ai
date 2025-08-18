# Terraphim System Operator Configuration

This document describes how to set up and use the Terraphim server with the **System Operator** role configuration that uses a **remote knowledge graph** and documents from the [terraphim/system-operator](https://github.com/terraphim/system-operator.git) GitHub repository.

## 🎯 Overview

The System Operator configuration provides:

- **Remote Knowledge Graph**: Uses pre-built automata from `https://staging-storage.terraphim.io/thesaurus_Default.json`
- **GitHub Document Integration**: Automatically indexes documents from the system-operator repository
- **Advanced Search**: TerraphimGraph ranking with knowledge graph-based relevance scoring
- **System Engineering Focus**: Specialized for MBSE, requirements, architecture, and verification content

## 📋 Prerequisites

- Rust and Cargo installed
- Git for cloning repositories
- Internet connection for remote knowledge graph access
- At least 2GB free disk space for documents

## 🚀 Quick Start

### 1. Setup System Operator Environment

Run the automated setup script:

```bash
./scripts/setup_system_operator.sh
```

This script will:
- Clone the system-operator repository to `/tmp/system_operator`
- Verify markdown files are available
- Display configuration information

### 2. Start the Server

```bash
cargo run --bin terraphim_server -- --config terraphim_server/default/system_operator_config.json
```

The server will start on `http://127.0.0.1:8000` by default.

### 3. Test the Configuration

Run the integration test to verify everything works:

```bash
cargo test --test system_operator_integration_test -- --nocapture
```

## 📁 Configuration Files

### Core Configuration
- **`terraphim_server/default/system_operator_config.json`** - Main server configuration
- **`terraphim_server/default/settings_system_operator_server.toml`** - Server settings with S3 profiles

### Generated Data
- **`/tmp/system_operator/pages/`** - ~1,300 markdown files from GitHub repository
- **Remote KG**: `https://staging-storage.terraphim.io/thesaurus_Default.json`

## 🔧 Configuration Details

### Roles Available

1. **System Operator** (Default)
   - **Relevance Function**: `terraphim-graph` 
   - **Theme**: `superhero` (dark theme)
   - **Remote KG**: ✅ Enabled
   - **Local Docs**: ✅ `/tmp/system_operator/pages`

2. **Engineer**
   - **Relevance Function**: `terraphim-graph`
   - **Theme**: `lumen` (light theme)  
   - **Remote KG**: ✅ Enabled
   - **Local Docs**: ✅ `/tmp/system_operator/pages`

3. **Default**
   - **Relevance Function**: `title-scorer`
   - **Theme**: `spacelab`
   - **Remote KG**: ❌ Disabled
   - **Local Docs**: ✅ `/tmp/system_operator/pages`

### Remote Knowledge Graph Details

- **URL**: `https://staging-storage.terraphim.io/thesaurus_Default.json`
- **Type**: Pre-built automata with 1,700+ terms
- **Coverage**: System engineering, MBSE, requirements, architecture
- **Performance**: Fast loading, no local build required

## 🔍 API Usage Examples

### Health Check
```bash
curl http://127.0.0.1:8000/health
```

### Search with System Operator Role
```bash
curl "http://127.0.0.1:8000/documents/search?q=MBSE&role=System%20Operator&limit=5"
```

### Get Configuration
```bash
curl http://127.0.0.1:8000/config
```

### Search for Specific Terms
```bash
# Requirements management
curl "http://127.0.0.1:8000/documents/search?q=requirements&role=System%20Operator"

# Architecture modeling
curl "http://127.0.0.1:8000/documents/search?q=architecture&role=System%20Operator"

# Verification and validation
curl "http://127.0.0.1:8000/documents/search?q=verification&role=System%20Operator"
```

## 🧪 Testing

### Run Integration Tests
```bash
# Run system operator specific tests
cargo test --test system_operator_integration_test -- --nocapture

# Run all server tests
cargo test -p terraphim_server -- --nocapture
```

### Manual Testing
1. Start the server
2. Open `http://127.0.0.1:8000` in browser
3. Search for terms like "MBSE", "requirements", "system architecture"
4. Verify results are ranked by knowledge graph relevance

## 📊 Expected Performance

### Startup Time
- **Remote KG Load**: ~2-3 seconds
- **Document Indexing**: ~5-10 seconds for 1,300+ files
- **Total Startup**: ~15 seconds

### Search Performance
- **Knowledge Graph Search**: <100ms for most queries
- **Document Count**: 1,300+ markdown files
- **Index Size**: ~50MB in memory

### Sample Search Results

When searching for "MBSE":
- Model-Based Systems Engineering documents
- Adoption strategies and best practices
- Tool integration approaches
- Case studies and lessons learned

## 🛠️ Troubleshooting

### Common Issues

1. **Repository Clone Failed**
   ```bash
   # Manual clone
   git clone https://github.com/terraphim/system-operator.git /tmp/system_operator
   ```

2. **Remote KG Not Loading**
   - Check internet connection
   - Verify URL: https://staging-storage.terraphim.io/thesaurus_Default.json
   - Check firewall settings

3. **No Search Results**
   - Ensure documents are in `/tmp/system_operator/pages/`
   - Check server logs for indexing errors
   - Verify role has `terraphim-graph` relevance function

4. **Server Won't Start**
   ```bash
   # Check if port is in use
   lsof -i :8000
   
   # Use different port
   cargo run --bin terraphim_server -- --config terraphim_server/default/system_operator_config.json --addr 127.0.0.1:8080
   ```

### Debug Mode
```bash
RUST_LOG=debug cargo run --bin terraphim_server -- --config terraphim_server/default/system_operator_config.json
```

## 🔄 Updating Documents

To refresh the system operator documents:

```bash
cd /tmp/system_operator
git pull origin main
# Restart the server to re-index
```

## 🌐 Production Deployment

### Environment Variables
```bash
export TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8000"
export TERRAPHIM_SERVER_API_ENDPOINT="https://your-domain.com/api"
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
```

### Docker Deployment
```bash
# Build with system operator config
docker build -t terraphim-system-operator .

# Run with mounted documents
docker run -p 8000:8000 \
  -v /tmp/system_operator:/tmp/system_operator:ro \
  terraphim-system-operator
```

## 📚 Related Documentation

- [Terraphim Configuration Guide](../docs/src/Configuration.md)
- [Knowledge Graph Documentation](../docs/src/kg/)
- [API Reference](../docs/src/API.md)
- [System Operator Repository](https://github.com/terraphim/system-operator)

## 🤝 Contributing

To improve the system operator configuration:

1. Fork the repository
2. Make changes to configuration files
3. Test with the integration test suite
4. Submit a pull request

## 📄 License

This configuration is part of the Terraphim project and follows the same Apache 2.0 license. 