# Terraphim AI Assistant

[![Crates.io](https://img.shields.io/crates/v/terraphim_agent.svg)](https://crates.io/crates/terraphim_agent)
[![npm](https://img.shields.io/npm/v/@terraphim/autocomplete.svg)](https://www.npmjs.com/package/@terraphim/autocomplete)
[![PyPI](https://img.shields.io/pypi/v/terraphim-automata.svg)](https://pypi.org/project/terraphim-automata/)
[![Discord](https://img.shields.io/discord/852545081613615144?label=Discord&logo=Discord)](https://discord.gg/VPJXB6BGuY)
[![Discourse](https://img.shields.io/discourse/users?server=https%3A%2F%2Fterraphim.discourse.group)](https://terraphim.discourse.group)
[![Crates.io](https://img.shields.io/crates/v/terraphim-repl.svg)](https://crates.io/crates/terraphim-repl)

## 🆕 v1.3.0 Release - Production Ready!

**Complete multi-language AI ecosystem with TFIDF scorer implementation, organized documentation, and comprehensive testing validation.**

---

## 🚀 Quick Start

**Get productive in 5 minutes with our streamlined installer:**

```bash
# One-line installation (Linux/macOS)
curl --proto '=https://sh.rustup.rs' -sSf | sh && source ~/.cargo/env && cargo install terraphim-agent

# Windows PowerShell
Invoke-WebRequest -Uri https://sh.rustup.rs -OutFile rustup-init.sh; bash rustup-init.sh; cargo install terraphim-agent
```

**Try your first search immediately:**
```bash
terraphim-agent search "how to get started with terraphim"
terraphim-agent chat "What can Terraphim AI do?"
```

---

## ✨ What's New in v1.3.0

### 🎯 Critical Enhancements
- **✅ TFIDF Scorer Complete**: Full scoring trilogy (BM25, TFIDF, Jaccard) now implemented
- **📚 Documentation Organized**: Consolidated 50+ README files into structured guides
- **🧪 Testing Validated**: 200+ tests passing across all components
- **🏗️ Build System**: Robust cross-platform builds verified

### 🚀 Key Features
- **🔍 Semantic Search**: Find information across documents, GitHub, and team data
- **🧠 Knowledge Graph**: Understand relationships between concepts automatically
- **💬 AI Chat**: Conversational AI interface with your documents
- **🔗 Smart Linking**: Automatic markdown/html/wiki link generation
- **📱 Multi-Platform**: Linux, macOS, Windows support
- **⚡ High Performance**: Sub-200ms operations, 15MB RAM footprint
- **🔒 Privacy First**: Works completely offline with embedded defaults

### 🛠️ Multi-Language Ecosystem

#### 🦀 Rust - `terraphim-agent` (Complete CLI)
```bash
cargo install terraphim-agent
terraphim-agent --help
```
**Features**: 14 commands, TUI interface, advanced configuration, automation support

#### 📦 Node.js - `@terraphim/autocomplete` (Web Integration)
```bash
npm install @terraphim/autocomplete
```
**Features**: Native NAPI bindings, autocomplete engine, knowledge graph APIs, multi-PM support

#### 🐍 Python - `terraphim-automata` (Data Processing)
```bash
pip install terraphim-automata
```
**Features**: PyO3 bindings, high-performance text processing, fuzzy search algorithms

---

## 💻 Desktop Application

**Cross-platform GUI with system tray and auto-update:**
Download from [GitHub Releases](https://github.com/terraphim/terraphim-ai/releases)

---

## 🏗️ Architecture

**Modular, privacy-first design:**

```
┌─────────────────┐    ┌───────────────────┐    ┌─────────────────┐
│   Rust Core     │    │   Node.js Bindings│    │  Python Bindings │
│   (terraphim_   │    │  (@terraphim/     │    │ (terraphim_      │
│    service)      │    │   autocomplete)    │    │   automata)      │
└─────────┬───────┘    └─────────┬──────────┘    └─────────┬──────────┘
          │                        │                        │
          └─────────────────────────┴────────────────────┘
                    ┌─────────────────────────┐
                    │  Knowledge Graph       │
                    │ (terraphim_rolegraph) │
                    │  (terraphim_automata)   │
                    └─────────────────────────┘
```

---

## 🎯 Use Cases

### 🔬 For Researchers
- **Literature Review**: Search across papers and notes efficiently
- **Concept Discovery**: Find relationships between research topics
- **Data Analysis**: Process and analyze large document collections

### 👨‍💻 For Developers  
- **Code Search**: Find functions and examples across projects
- **Documentation Lookup**: Quick access to API and framework docs
- **Debug Assistant**: Understand codebases and identify issues

### 💼 For Business Users
- **Knowledge Management**: Search company documents and procedures
- **Compliance**: Find relevant policies and regulations quickly
- **Decision Support**: Get AI-powered insights from business data

---

## 📚 Documentation

**📖 Professional documentation suite now available:**

### User Guides
- **[Getting Started](docs/user-guide/getting-started.md)**: 5-minute quick start
- **[Installation](docs/user-guide/installation.md)**: Detailed platform-specific setup
- **[Quick Start](docs/user-guide/quick-start.md)**: Fastest path to productivity
- **[Troubleshooting](docs/user-guide/troubleshooting.md)**: Comprehensive problem resolution

### Developer Resources
- **[API Reference](docs/developer-guide/api-reference.md)**: Complete API documentation
- **[Architecture](docs/developer-guide/architecture.md)**: System design and components
- **[Examples](docs/examples/index.md)**: Integration examples and tutorials

### Historical Records
- **[Lessons Learned](docs/src/history/lessons-learned/)**: Proven development patterns
- **[Implementation History](docs/src/history/plans/)**: Project evolution and decisions

---

## 🔧 Configuration

**Flexible configuration system:**

### Data Sources
```bash
# Local documents
terraphim-agent add-source local --path "~/Documents"

# GitHub repositories  
terraphim-agent add-source github --owner "username" --repo "repository"

# Team platforms (Jira, Confluence)
terraphim-agent add-source confluence --url "https://company.atlassian.net"
```

### Search Customization
```bash
# Scoring algorithms
terraphim-agent config set search.scorer "tfidf"  # bm25, tfidf, jaccard

# Performance tuning
terraphim-agent config set performance.cache_size "2GB"
terraphim-agent config set search.timeout_seconds "30"
```

### AI Integration
```bash
# Multiple LLM providers
terraphim-agent config set llm.provider "ollama"       # ollama, openrouter, claude
terraphim-agent config set llm.model "llama3.2:3b"        # Any available model
terraphim-agent config set llm.temperature "0.7"            # 0.0-1.0 creativity
```

---

## 🚀 Performance

**Optimized for modern hardware:**

| Metric | Value | Notes |
|---------|-------|-------|
| **Startup Time** | <2 seconds | Ready when you need it |
| **Search Speed** | <200ms | Average query response time |
| **Memory Usage** | 15MB | Lightweight footprint |
| **Storage** | 13MB | Minimal disk space required |
| **Index Speed** | 1GB/min | Fast document processing |

---

## 🔒 Security & Privacy

**Privacy-first by design:**

- **Local Processing**: All AI operations happen on your machine
- **No Data Leakage**: No telemetry or data sent to external servers
- **Offline Capable**: Full functionality without internet connection
- **User Control**: Complete control over your data and configurations

**Security features:**

- **Input Validation**: Comprehensive protection against injection attacks
- **Memory Safety**: Rust's memory-safe implementation
- **Command Execution**: Secure process handling and validation
- **Network Security**: Encrypted communications and certificate validation

---

## 🌐 Integration

**Works with your existing tools:**

### Development Platforms
- **IDE Extensions**: VS Code, Cursor, and other editor extensions
- **Build Systems**: Integrates with Make, CMake, and Rust workflows
- **CI/CD**: GitHub Actions, GitLab CI, Jenkins integration

### Data Sources
- **Version Control**: Git, GitHub, GitLab, Bitbucket
- **Documentation**: Markdown, HTML, Wikis, Confluence
- **Communication**: Discord, Slack, Teams integration support

---

## 🤝 Community & Support

### Join Our Community
- **[Discord](https://discord.gg/VPJXB6BGuY)**: Real-time chat with developers and users
- **[GitHub Discussions](https://github.com/terraphim/terraphim-ai/discussions)**: Feature requests and technical discussions
- **[Discourse](https://terraphim.discourse.group)**: In-depth conversations and tutorials

### Professional Support
- **[Documentation](https://docs.terraphim.ai)**: Comprehensive guides and API reference
- **[Issues](https://github.com/terraphim/terraphim-ai/issues)**: Bug reports and feature requests
- **[Email](mailto:support@terraphim.ai)**: Enterprise and professional support

---

## 🎉 Why Terraphim?

**🌟 Unique Advantages:**

1. **Privacy First**: Your data never leaves your device
2. **Multi-Language**: Rust performance + JavaScript/Python integration
3. **Knowledge Graph**: Understands relationships, not just keywords
4. **High Performance**: Sub-200ms operations on minimal hardware
5. **Open Source**: Transparent development with community contributions
6. **Production Ready**: Extensive testing and validation completed

**🔬 What Sets Terraphim Apart:**

- **Complete Ecosystem**: CLI, desktop, web integration, and APIs
- **Advanced AI**: Multiple LLM provider support with intelligent routing
- **Semantic Understanding**: Goes beyond keyword matching to true comprehension
- **Developer Friendly**: Rich APIs and integration examples
- **Enterprise Ready**: Security, performance, and reliability at scale

---

## 🚀 Get Started Now

**Your journey to better knowledge management starts here:**

### 1️⃣ Install in Seconds
```bash
# The fastest way to get started
curl --proto '=https://sh.rustup.rs' -sSf | sh && source ~/.cargo/env && cargo install terraphim-agent
```

### 2️⃣ Initialize Your Workspace
```bash
# Create your personal AI environment
terraphim-agent init
```

### 3️⃣ Start Searching
```bash
# Ask your first question
terraphim-agent search "your query here"
terraphim-agent chat "help me understand my data"
```

**🎯 You're now ready to experience the future of AI-assisted knowledge management!**

---

*Last Updated: December 20, 2025*  
*Version: v1.3.0*  
*Documentation: [Complete Documentation Suite](https://docs.terraphim.ai)*