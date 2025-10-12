# 🚀 Terraphim Novel Autocomplete Testing Scripts

This directory contains comprehensive bash scripts for testing the Novel editor autocomplete integration with Terraphim's knowledge graph system.

## 📁 Scripts Overview

### 🎯 Quick Start (Recommended)
```bash
./quick-start-autocomplete.sh
```
**Interactive menu** with common testing scenarios. Perfect for getting started quickly!

### 🔧 Main Testing Script
```bash
./start-autocomplete-test.sh [options]
```
**Comprehensive startup script** with full control over services and configuration.

### 🛑 Stop Script
```bash
./stop-autocomplete-test.sh [options]
```
**Clean shutdown** of all testing services with graceful or force stop options.

### 🧪 Integration Tests
```bash
./desktop/test-novel-autocomplete-integration.js
```
**Validation script** to test MCP server integration and autocomplete functionality.

## 🚀 Quick Start Guide

### Method 1: Interactive Menu (Easiest)
```bash
./quick-start-autocomplete.sh
```
Choose from preset configurations:
1. **Full Testing** - Everything you need
2. **MCP Only** - Backend development
3. **Desktop Development** - UI work
4. **Tests Only** - Validation
5. **Status Check** - See what's running
6. **Stop Services** - Clean shutdown

### Method 2: Direct Commands
```bash
# Full testing environment
./quick-start-autocomplete.sh full

# Development environment
./quick-start-autocomplete.sh dev

# Check what's running
./quick-start-autocomplete.sh status

# Stop everything
./quick-start-autocomplete.sh stop
```

## 🔧 Advanced Usage

### Start Full Testing Environment
```bash
./start-autocomplete-test.sh
```
**Starts:**
- MCP Server on port 8001
- Desktop Tauri app
- Runs integration tests
- Shows real-time status

### Custom Port Configuration
```bash
./start-autocomplete-test.sh --port 8080 --verbose
```

### Start Only MCP Server
```bash
./start-autocomplete-test.sh --mcp-only --port 8001
```

### Development Mode (No Tests)
```bash
./start-autocomplete-test.sh --no-tests
```

### Stop All Services
```bash
./stop-autocomplete-test.sh
```

### Force Stop (If Normal Stop Fails)
```bash
./stop-autocomplete-test.sh --force
```

### Check Service Status
```bash
./stop-autocomplete-test.sh --status
```

## 📋 Command Reference

### start-autocomplete-test.sh Options
```
-h, --help              Show help message
-m, --mcp-only          Start only MCP server
-a, --axum-only         Start only Axum server
-d, --desktop-only      Start only desktop app
-t, --test-only         Run only integration tests
-p, --port PORT         Set MCP server port (default: 8001)
-w, --web-port PORT     Set web server port (default: 3000)
--no-desktop            Skip desktop app startup
--no-tests              Skip integration tests
--verbose               Enable verbose logging
```

### stop-autocomplete-test.sh Options
```
-h, --help      Show help message
-s, --status    Check status of running services
-f, --force     Force kill all processes
```

### quick-start-autocomplete.sh Arguments
```
full        Full testing environment
mcp         MCP server only
dev         Development environment
test        Run tests only
status      Check service status
stop        Stop all services
custom      Custom configuration
```

## 🌐 Services & Ports

| Service | Default Port | URL | Purpose |
|---------|-------------|-----|---------|
| MCP Server | 8001 | http://127.0.0.1:8001 | Autocomplete API |
| Axum Server | Dynamic | N/A | Alternative backend |
| Desktop App | N/A | Tauri Window | Novel editor UI |

## 📊 Monitoring & Logs

### Log Files
```
logs/mcp_server.log      - MCP server output
logs/axum_server.log     - Axum server output
logs/desktop_app.log     - Desktop app output
```

### Process IDs
```
pids/mcp_server.pid      - MCP server PID
pids/axum_server.pid     - Axum server PID
pids/desktop_app.pid     - Desktop app PID
```

### Real-time Monitoring
```bash
# Watch MCP server logs
tail -f logs/mcp_server.log

# Watch all logs
tail -f logs/*.log

# Check process status
ps aux | grep terraphim
```

## 🧪 Testing Workflow

### 1. Start Services
```bash
./quick-start-autocomplete.sh full
```

### 2. Wait for "Ready" Message
Look for:
```
🎉 Terraphim Novel Autocomplete Testing Environment Ready!
```

### 3. Test in Desktop App
- Open Tauri window (should appear automatically)
- Navigate to editor page
- Click "Demo" button
- Type `/terraphim` in editor
- Verify dropdown appears with suggestions

### 4. Validate with Integration Tests
Tests run automatically, or manually:
```bash
node desktop/test-novel-autocomplete-integration.js
```

### 5. Stop Services
```bash
./quick-start-autocomplete.sh stop
```

## 🔍 Troubleshooting

### Common Issues

#### "Port already in use"
```bash
# Check what's using the port
lsof -i :8001

# Kill existing process
kill -9 $(lsof -ti :8001)

# Or use force stop
./stop-autocomplete-test.sh --force
```

#### "MCP server not responding"
```bash
# Check MCP server logs
cat logs/mcp_server.log

# Restart just MCP server
./start-autocomplete-test.sh --mcp-only
```

#### "Desktop app won't start"
```bash
# Check desktop logs
cat logs/desktop_app.log

# Ensure dependencies installed
cd desktop && yarn install

# Try building Tauri
cd desktop && yarn tauri build --debug
```

#### "No autocomplete suggestions"
1. Verify MCP server is running: `./stop-autocomplete-test.sh --status`
2. Check backend connection in UI status panel
3. Try known terms: "terraphim", "graph", "role"
4. Verify role has knowledge graph data

### Debug Mode
```bash
# Enable verbose logging
./start-autocomplete-test.sh --verbose

# Check all process statuses
./stop-autocomplete-test.sh --status

# Manual MCP test
curl -X POST "http://localhost:8001/message?sessionId=test" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

## 🎯 Testing Scenarios

### 1. Developer Workflow
**Goal:** Test autocomplete during development
```bash
./quick-start-autocomplete.sh dev
# Starts MCP + Desktop, no tests
# Good for iterative development
```

### 2. CI/CD Pipeline
**Goal:** Automated testing
```bash
./start-autocomplete-test.sh --test-only
# Runs integration tests only
# Perfect for automated validation
```

### 3. Backend Development
**Goal:** Test MCP server changes
```bash
./start-autocomplete-test.sh --mcp-only --verbose
# Just MCP server with detailed logs
# Test with curl or integration script
```

### 4. Full System Test
**Goal:** End-to-end validation
```bash
./quick-start-autocomplete.sh full
# Complete environment with tests
# Comprehensive validation
```

## ⚙️ Configuration

### Environment Variables
```bash
export MCP_SERVER_PORT=8080        # Custom MCP port
export WEB_SERVER_PORT=3001        # Custom web port
export RUST_LOG=debug              # Verbose Rust logging
export TERRAPHIM_INITIALIZED=true  # Skip setup prompts
```

### Prerequisites Check
Scripts automatically verify:
- ✅ Rust/Cargo installation
- ✅ Node.js/Yarn installation
- ✅ Required commands (curl, lsof)
- ✅ Correct directory structure

## 🎉 Success Indicators

### MCP Server Ready
```
✅ MCP server is ready!
📝 Testing autocomplete_terms...
✅ autocomplete_terms working
```

### Desktop App Ready
```
✅ Desktop app started (PID: 12345)
• Desktop App: Starting (Tauri window should appear)
```

### Integration Tests Pass
```
✅ MCP server responding
✅ autocomplete_terms working
✅ autocomplete_with_snippets working
🎉 Autocomplete integration is ready!
```

## 📚 Additional Resources

- **Implementation Details:** `NOVEL_AUTOCOMPLETE_FIXES.md`
- **Autocomplete Demo:** `desktop/AUTOCOMPLETE_DEMO.md`
- **Configuration Options:** `desktop/src/lib/config/autocomplete.ts`
- **Custom Extension:** `desktop/src/lib/Editor/TerraphimSuggestion.ts`

## 🤝 Contributing

When modifying the testing scripts:

1. **Test all scenarios** before committing
2. **Update this README** if adding new options
3. **Maintain backward compatibility** when possible
4. **Add proper error handling** and user feedback
5. **Follow the existing color scheme** and formatting

## 📞 Support

If you encounter issues with the testing scripts:

1. **Check logs** in the `logs/` directory
2. **Run status check**: `./stop-autocomplete-test.sh --status`
3. **Try force stop/restart**: `./stop-autocomplete-test.sh --force`
4. **Review troubleshooting section** above
5. **Open an issue** with log files and error messages

---

**Happy Testing!** 🚀 The scripts are designed to make Novel editor autocomplete testing as smooth as possible.
