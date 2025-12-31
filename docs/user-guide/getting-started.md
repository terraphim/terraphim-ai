# Getting Started with Terraphim AI

Welcome to Terraphim AI! This guide will help you get up and running quickly, whether you're a developer, researcher, or end user.

## 🚀 Quick Start (5 Minutes)

### Choose Your Installation Method

#### Option 1: Rust CLI/TUI (Recommended for Power Users)
```bash
cargo install terraphim-agent
terraphim-agent --help
```

#### Option 2: Node.js Package (Great for Web Integration)
```bash
npm install @terraphim/autocomplete
# or with Bun
bun add @terraphim/autocomplete
```

#### Option 3: Python Library (Perfect for Data Processing)
```bash
pip install terraphim-automata
```

#### Option 4: Desktop Application (GUI Users)
Download from [GitHub Releases](https://github.com/terraphim/terraphim-ai/releases)

### Your First Query

After installation, try your first semantic search:

```bash
# Rust CLI
terraphim-agent search "your query here"

# Node.js
const terraphim = require('@terraphim/autocomplete');
const results = await terraphim.search('your query here');

# Python
from terraphim_automata import Autocomplete
engine = Autocomplete()
results = engine.search('your query here')
```

---

## 🎯 What Can Terraphim AI Do?

### Core Capabilities
- **🔍 Semantic Search**: Find information across multiple data sources
- **🧠 Knowledge Graph**: Understand relationships between concepts
- **💬 AI Chat**: Interactive conversations with your data
- **📊 Smart Linking**: Automatic markdown/html/wiki linking
- **🔄 Auto-Update**: Always up-to-date with latest features

### Data Sources You Can Search
- **Local Files**: Your markdown, text, and code files
- **GitHub Repositories**: Search across 500,000+ public repositories
- **Team Knowledge**: Jira, Confluence, SharePoint integration
- **Stack Overflow**: Programming questions and answers

---

## 🏗️ Architecture Overview

Terraphim AI uses a modular, privacy-first architecture:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Rust Core     │    │   Node.js Bindings│    │  Python Bindings │
│   (terraphim_   │    │  (@terraphim/     │    │ (terraphim-      │
│    service)      │    │   autocomplete)    │    │   automata)      │
└─────────┬───────┘    └─────────┬─────────┘    └─────────┬─────────┘
          │                        │                        │
          └─────────────────────────┴────────────────────┘
                    ┌─────────────────────────┐
                    │  Knowledge Graph       │
                    │  (terraphim_rolegraph) │
                    │  (terraphim_automata)   │
                    └─────────────────────────┘
```

---

## 🔧 Configuration Basics

### Environment Setup

#### 1. Basic Configuration
Create a configuration file at `~/.config/terraphim/config.toml`:

```toml
[data]
default_data_path = "~/Documents/terraphim"
index_documents = true

[search]
default_scorer = "tfidf"  # Options: bm25, tfidf, jaccard
max_results = 20

[llm]
provider = "ollama"  # Options: ollama, openrouter, claude
model = "llama3.2:3b"
temperature = 0.7
```

#### 2. Data Source Configuration
Add your data sources:

```toml
[sources.local_files]
path = "~/Documents"
file_types = ["md", "txt", "rst"]

[sources.github]
enabled = true
languages = ["rust", "python", "javascript"]
```

---

## 📚 Next Steps

### For Different User Types

#### 🔬 Researchers & Data Scientists
- [Data Processing Guide](developer-guide/data-processing.md)
- [Advanced Search Techniques](user-guide/advanced-usage.md)
- [API Reference](developer-guide/api-reference.md)

#### 🌐 Web Developers  
- [Node.js Integration](developer-guide/nodejs-integration.md)
- [REST API Guide](developer-guide/api-reference.md)
- [Browser Extension Setup](developer-guide/browser-extensions.md)

#### 👥‍💼 Business Users
- [Team Deployment Guide](user-guide/team-deployment.md)
- [Configuration Management](user-guide/configuration.md)
- [Troubleshooting](user-guide/troubleshooting.md)

#### 🔧 System Administrators
- [Server Installation](user-guide/installation.md)
- [Desktop Deployment](user-guide/desktop-app.md)
- [Security Configuration](user-guide/security-setup.md)

---

## ❓ Need Help?

### Common Questions

**Q: Which package should I use?**
A: 
- **Developers**: Rust CLI (`terraphim-agent`)
- **Web Integration**: Node.js (`@terraphim/autocomplete`)  
- **Data Analysis**: Python (`terraphim-automata`)
- **GUI Users**: Desktop application

**Q: How do I add my own data?**
A: See [Configuration Guide](user-guide/configuration.md) for detailed instructions

**Q: Can I use Terraphim offline?**
A: Yes! Terraphim is privacy-first and works completely offline with embedded defaults

### Getting Support

- **📚 Documentation**: [Full Documentation](https://docs.terraphim.ai)
- **💬 Discord**: [Join our Community](https://discord.gg/VPJXB6BGuY)
- **🏛️ Discourse**: [Discussion Forum](https://terraphim.discourse.group)
- **🐛 Issues**: [Report on GitHub](https://github.com/terraphim/terraphim-ai/issues)

---

## 🎉 You're Ready!

You now have Terraphim AI installed and configured. Start exploring your knowledge graph with semantic search powered by AI assistance.

**Try these commands first:**
```bash
terraphim-agent search "semantic search examples"
terraphim-agent chat "help me understand my data"
terraphim-agent role create research-assistant
```

Welcome to the future of privacy-first AI assistance! 🚀

---

*Last Updated: December 20, 2025*
*Part of: Terraphim AI Documentation v1.3.0*