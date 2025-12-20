# Quick Start Guide

Get productive with Terraphim AI in 5 minutes! This guide shows you the fastest path to your first semantic search.

## 🚀 Your 5-Minute Quick Start

### Step 1: Install (2 Minutes)

```bash
# Choose your platform and run ONE command:

# Linux/macOS
curl --proto '=https://sh.rustup.rs' -sSf | sh && source ~/.cargo/env && cargo install terraphim-agent

# Windows (PowerShell as Administrator)
Invoke-WebRequest -Uri https://sh.rustup.rs -OutFile rustup-init.sh; bash rustup-init.sh; cargo install terraphim-agent
```

### Step 2: Initialize (30 Seconds)

```bash
# Create your personal AI workspace
terraphim-agent init

# Answer the prompts:
# ✓ Default data path: ~/Documents/terraphim
# ✓ Enable local file indexing: Yes
# ✓ Enable GitHub search: Yes (optional)
# ✓ Default scorer: tfidf
```

### Step 3: First Search (30 Seconds)

```bash
# Try your first semantic search
terraphim-agent search "how to install rust"

# Or explore your data
terraphim-agent search --help
```

### Step 4: Optional - Add Your Data (2 Minutes)

```bash
# Add your documents folder
terraphim-agent add-source local --path ~/Documents

# Add a GitHub repository (if you want code search)
terraphim-agent add-source github --owner terraphim --repo terraphim-ai
```

### Step 5: AI Chat (1 Minute)

```bash
# Start chatting with your documents
terraphim-agent chat "What are the main features of Terraphim?"
```

---

## 🎯 You're Ready!

**Congratulations!** You now have a fully functional semantic AI assistant working with your local data.

### What You Can Do Now
- **🔍 Semantic Search**: Find information across your documents instantly
- **🧠 Knowledge Graph**: Discover connections between concepts
- **💬 AI Chat**: Have conversations with your documents
- **🔗 Smart Linking**: Automatic markdown and wiki linking
- **⚡ Offline First**: Everything works without internet (optional GitHub search)

---

## 📊 Example Use Cases

### For Developers
```bash
# Search your codebase for specific patterns
terraphim-agent search "database connection error handling"

# Find API endpoints in your project
terraphim-agent search "REST API endpoints authentication"

# Get help with debugging
terraphim-agent chat "I'm getting a segmentation fault. What should I check?"
```

### For Researchers  
```bash
# Search research papers and notes
terraphim-agent search "machine learning algorithms comparison"

# Find related concepts
terraphim-agent search "reinforcement learning applications"

# Generate summaries
terraphim-agent chat "Summarize the key findings about neural networks"
```

### For Business Users
```bash
# Search company documentation
terraphim-agent search "quarterly financial report Q3"

# Find policies and procedures
terraphim-agent search "expense approval process"

# Get quick answers
terraphim-agent chat "What are our remote work policies?"
```

---

## 🛠️ Configuration Tips

### Speed Up Searches
```bash
# Optimize for your hardware
terraphim-agent config set performance.cache_size "2GB"  # If you have 16GB+ RAM
terraphim-agent config set search.max_results "50"       # For comprehensive results
terraphim-agent config set search.timeout_seconds "10"    # For faster responses
```

### Better Search Results
```bash
# Try different scorers
terraphim-agent search --scorer tfidf "your query"    # Best for general search
terraphim-agent search --scorer bm25 "your query"     # Best for technical terms
terraphim-agent search --scorer jaccard "your query"   # Best for exact matches

# Combine data sources
terraphim-agent search --source local,github "your query"  # Search both sources
```

---

## 🔧 Customization

### Personalize Your Experience
```bash
# Create custom roles
terraphim-agent role create "my-research" --purpose "Research assistance" --model "llama3.2:3b"

# Set up custom workflows
terraphim-agent workflow create "literature-review" --steps "search,analyze,summarize"
```

### Keyboard Shortcuts
```bash
# Enable TUI mode for keyboard users
terraphim-agent --tui

# Common shortcuts in TUI:
# Ctrl+R: Search
# Ctrl+C: Chat mode
# Ctrl+L: List sources
# Ctrl+H: Help
```

---

## 📱 Cross-Platform Usage

### Linux/macOS Terminal
```bash
# Full terminal experience
terraphim-agent

# TUI mode (enhanced interface)
terraphim-agent --tui

# Background mode (for scripts)
terraphim-agent search "query" > results.json
```

### Windows PowerShell
```powershell
# PowerShell commands
terraphim-agent search "query"

# TUI mode
terraphim-agent --tui
```

### Desktop Application
```bash
# Launch GUI (if installed)
terraphim-ai-desktop

# Command-line usage with desktop app
terraphim-ai-desktop --cli search "query"
```

---

## 🚨 Common Quick Issues

### "Command not found"
```bash
# Reload your shell environment
source ~/.bashrc  # or ~/.zshrc

# Or use full path
~/.cargo/bin/terraphim-agent search "query"
```

### "Permission denied"
```bash
# Fix permissions
chmod +x ~/.cargo/bin/terraphim-agent

# Or reinstall with proper permissions
cargo install terraphim-agent --force
```

### "No results found"
```bash
# Check data sources
terraphim-agent sources list

# Rebuild index
terraphim-agent rebuild-index

# Try broader search terms
terraphim-agent search "search term" --scorer tfidf --max-results 50
```

---

## 🎉 Success Metrics

### You're Successful When
- [ ] Terraphim responds to queries in under 2 seconds
- [ ] You can see your documents in search results
- [ ] Chat mode provides relevant answers from your data
- [ ] Configuration file exists at `~/.config/terraphim/config.toml`
- [ ] Help command shows all available features
- [ ] You can switch between search modes (local, GitHub, combined)

### Performance Tips
- **First query** might be slower (index building)
- **Subsequent queries** are much faster (cache enabled)
- **Large datasets** benefit from larger cache sizes
- **SSD storage** significantly improves indexing speed

---

## 🎯 Next Steps

### Advanced Features to Explore
1. **Multi-source search**: Combine local + GitHub + team data
2. **Custom scorers**: Configure for your specific domain
3. **AI workflows**: Create automated search and analysis pipelines
4. **Integration**: Connect with your development tools

### Learning Resources
- [Full Documentation](https://docs.terraphim.ai)
- [Video Tutorials](https://www.youtube.com/@terraphim)
- [Community Examples](https://github.com/terraphim/terraphim-ai/examples)
- [Discord Community](https://discord.gg/VPJXB6BGuY)

### Need Help?
- **Quick Help**: `terraphim-agent --help`
- **Troubleshooting**: [Full Guide](user-guide/troubleshooting.md)
- **Community Support**: [Discord](https://discord.gg/VPJXB6BGuY) | [GitHub](https://github.com/terraphim/terraphim-ai/discussions)

---

**🚀 Welcome to Terraphim AI!** You now have a powerful semantic search assistant ready to help you find, understand, and work with your knowledge.

---

*Last Updated: December 20, 2025*
*Version: Terraphim AI v1.3.0*
*Part of: Terraphim AI Documentation Suite*