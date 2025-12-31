# Troubleshooting Guide

This guide helps you resolve common issues with Terraphim AI across all platforms and installation methods.

## 🔧 Installation Issues

### "cargo install terraphim-agent" fails

#### Rust Toolchain Issues
**Error**: `error: toolchain 'stable' is not installed`
**Solution**: Install Rust toolchain:
```bash
curl --proto '=https://sh.rustup.rs' -sSf | sh
source ~/.cargo/env
```

#### Compilation Errors
**Error**: `error: failed to compile crate XYZ`
**Solution**: Update Rust and try again:
```bash
rustup update
rustup install stable
```

#### Permission Denied
**Error**: `permission denied: /path/to/.cargo`
**Solution**: Check Rust installation directory permissions:
```bash
# Fix ownership
sudo chown -R $USER:$(id -gn $USER) ~/.cargo
sudo chmod -R 755 ~/.cargo

# Or use alternative installation
CARGO_HOME=/tmp/.cargo cargo install terraphim-agent
```

---

## 🔍 Search & Query Issues

### No Results Found

#### Empty Search Results
**Error**: Search returns empty results
**Solutions**:

1. **Check Data Source Configuration**:
```bash
terraphim-agent config show
# Verify data paths and enabled sources
```

2. **Verify File Permissions**:
```bash
# Ensure Terraphim can read your data
chmod -R 644 ~/Documents/your-data
```

3. **Rebuild Index**:
```bash
terraphim-agent rebuild-index
# Forces reindexing of all configured sources
```

### Slow Search Performance

#### Search Taking > 5 Seconds
**Solutions**:

1. **Check System Resources**:
```bash
# Check available memory
free -h

# Check disk space
df -h

# Check CPU usage
top -p | grep terraphim
```

2. **Optimize Configuration**:
```toml
# In ~/.config/terraphim/config.toml
[performance]
cache_size = "1GB"  # Increase if you have RAM
max_concurrent_queries = 4
```

3. **Reduce Search Scope**:
```bash
# Search specific directory only
terraphim-agent search --source local --path "~/specific-folder" "query"
```

---

## 🤖 Desktop Application Issues

### "Command not found" Error

#### Linux/macOS
**Error**: `terraphim-ai-desktop: command not found`
**Solutions**:

1. **Use Installation Path**:
```bash
# Check installation location
which terraphim-ai-desktop

# Run directly if found
/path/to/terraphim-ai-desktop
```

2. **Reinstall with Package Manager**:
```bash
# Try system package manager
sudo apt install terraphim-ai-desktop  # Ubuntu/Debian
brew install --cask terraphim-ai-desktop  # macOS
```

### Application Won't Start

#### Windows Issues
**Error**: "Application failed to start properly"
**Solutions**:

1. **Check Windows Defender**:
- Add Terraphim to exclusions
- Allow through firewall

2. **Install Visual C++ Redistributable**:
- Download from Microsoft website
- Restart computer

3. **Run as Administrator**:
- Right-click → "Run as administrator"

---

## 🌐 Network & API Issues

### GitHub Integration Problems

#### Rate Limiting
**Error**: "API rate limit exceeded"
**Solutions**:

1. **Configure Authentication**:
```toml
[sources.github]
token = "your-github-token"  # Create at github.com/settings/tokens
```

2. **Reduce Concurrent Requests**:
```toml
[sources.github]
max_concurrent_requests = 3
rate_limit_delay = 1000  # milliseconds
```

### Ollama Connection Issues

#### Connection Refused
**Error**: "Connection refused" when connecting to Ollama
**Solutions**:

1. **Check Ollama Status**:
```bash
ollama list
# Should show available models
```

2. **Verify Ollama Running**:
```bash
ps aux | grep ollama
# Check if Ollama process is running
```

3. **Start Ollama Service**:
```bash
# Linux
systemctl --user start ollama

# macOS
brew services start ollama

# Manual start
ollama serve
```

---

## 🧠 Memory & Performance Issues

### High Memory Usage

#### Memory Leak Detection
**Symptoms**: Memory usage increases continuously over time
**Solutions**:

1. **Monitor Memory Usage**:
```bash
# Watch Terraphim memory usage
watch -n 5 'ps aux | grep terraphim-agent | grep -v grep | awk "{print \$6}"'
```

2. **Reduce Cache Size**:
```toml
[persistence]
cache_size = "256MB"  # Reduce from default
cache_ttl = 3600  # Reduce cache time
```

3. **Restart Periodically**:
```bash
# Use cron to restart daily
0 2 * * * pkill -f terraphim-agent && sleep 2 && terraphim-agent &
```

---

## 🔐 Authentication & Security Issues

### API Key Problems

#### OpenRouter Integration
**Error**: "Invalid API key or authentication failed"
**Solutions**:

1. **Verify API Key Format**:
- Keys start with `sk-`
- No extra spaces or line breaks

2. **Check Environment Variables**:
```bash
echo $OPENROUTER_API_KEY
# Should show your key without trailing newline
```

3. **Test API Key**:
```bash
curl -X POST https://openrouter.ai/api/v1/chat/completions \
  -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "meta-llama/llama-3.1-8b", "messages": [{"role": "user", "content": "test"}]}'
```

---

## 📋 Data Source Issues

### Local File Indexing Problems

#### Large File Handling
**Error**: "File too large to process"
**Solutions**:

1. **Exclude Large Files**:
```toml
[sources.local_files]
exclude_patterns = ["*.log", "*.db", "node_modules/*", "target/*"]
max_file_size = "50MB"
```

2. **Optimize File Types**:
```toml
[sources.local_files]
binary_extensions = [".zip", ".tar.gz", ".mp4", ".pdf"]
index_binaries = false
```

### Git Repository Issues

#### Repository Not Found
**Error**: "Failed to clone repository"
**Solutions**:

1. **Check Repository URL**:
```bash
# Test repository accessibility
git ls-remote https://github.com/user/repo.git
```

2. **Use SSH Instead of HTTPS**:
```bash
# Configure SSH for GitHub
git config --global url.git@github.com:.insteadOf https://github.com/
```

---

## 🔄 Update & Auto-Update Issues

### Update Process Fails

#### Update Download Failed
**Error**: "Failed to download update"
**Solutions**:

1. **Check Network Connectivity**:
```bash
curl -I https://releases.terraphim.ai
# Should return 200 OK
```

2. **Manual Update**:
```bash
# Download latest version manually
wget https://releases.terraphim.ai/latest/terraphim-agent-linux-x64
chmod +x terraphim-agent-linux-x64
sudo ./terraphim-agent-linux-x64 --install
```

3. **Disable Auto-Update**:
```toml
[updates]
auto_check = false
# Check manually with: terraphim-agent update --check
```

---

## 🐛 Error Codes Reference

### Exit Codes

| Code | Meaning | Solution |
|-------|----------|----------|
| 1 | General Error | Check error message for details |
| 2 | File Not Found | Verify file paths and permissions |
| 3 | Permission Denied | Run with appropriate permissions |
| 4 | Network Error | Check network connectivity and configuration |
| 5 | Configuration Error | Validate config file syntax and values |
| 6 | Memory Error | Check available RAM and reduce cache sizes |
| 7 | LLM Error | Verify API keys and model availability |

### Log Locations

#### Finding Logs
```bash
# Rust CLI logs
~/.local/share/terraphim/logs/
tail -f ~/.local/share/terraphim/logs/terraphim-agent.log

# Desktop application logs
# Linux
~/.local/share/terraphim-ai-desktop/logs/
# macOS
~/Library/Logs/terraphim-ai-desktop/
# Windows
%APPDATA%/terraphim-ai-desktop/logs/
```

#### Log Levels
```bash
# Enable debug logging
export TERRAPHIM_LOG=debug
terraphim-agent search "test"

# Enable trace logging
export TERRAPHIM_LOG=trace
terraphim-agent search "test"
```

---

## 🚨 Emergency Procedures

### Reset to Defaults

If Terraphim becomes unresponsive or corrupted:

```bash
# Backup current configuration
cp ~/.config/terraphim/config.toml ~/terraphim-config-backup.toml

# Reset to defaults
rm -rf ~/.config/terraphim/
terraphim-agent init

# Restore custom settings
cp ~/terraphim-config-backup.toml ~/.config/terraphim/config.toml
```

### Complete Reinstall

```bash
# Remove all traces
rm -rf ~/.config/terraphim/
rm -rf ~/.local/share/terraphim/
cargo uninstall terraphim-agent

# Clean reinstall
cargo install terraphim-agent
```

---

## 📞 Getting Additional Help

### Community Support

1. **Discord Community**: [Join our Discord](https://discord.gg/VPJXB6BGuY)
   - Real-time help from community
   - Weekly office hours with maintainers
   - User discussions and tips

2. **GitHub Discussions**: [Start a Discussion](https://github.com/terraphim/terraphim-ai/discussions)
   - Detailed technical discussions
   - Feature requests and suggestions
   - Community knowledge base

3. **GitHub Issues**: [Report an Issue](https://github.com/terraphim/terraphim-ai/issues)
   - Bug reports and feature requests
   - Technical support from maintainers
   - Track issue resolution progress

### Professional Support

1. **Documentation**: [Full Documentation](https://docs.terraphim.ai)
   - Comprehensive guides and API reference
   - Troubleshooting steps and examples
   - Architecture and integration guides

2. **Email Support**: support@terraphim.ai
   - Enterprise and professional support
   - Priority response for business users
   - Security vulnerability reporting

---

## 🔧 Diagnostic Commands

### System Information Collection

Before reporting issues, collect this information:

```bash
# System information
terraphim-agent --version
uname -a
lsb_release -a 2>/dev/null || echo "Not Ubuntu/Debian"

# Rust information
rustc --version
cargo --version

# Memory and disk
free -h
df -h

# Network test
curl -I https://api.github.com
curl -I https://openrouter.ai

# Save to file
terraphim-agent --diagnostic-info > terraphim-diagnostic.txt
```

Include this diagnostic information when reporting issues for faster resolution.

---

*Last Updated: December 20, 2025*
*Version: Terraphim AI v1.3.0*
*Part of: Terraphim AI Documentation Suite*