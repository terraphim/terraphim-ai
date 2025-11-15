# Terraphim AI Assistant

[![Discord](https://img.shields.io/discord/852545081613615144?label=Discord&logo=Discord)](https://discord.gg/VPJXB6BGuY)
[![Discourse](https://img.shields.io/discourse/users?server=https%3A%2F%2Fterraphim.discourse.group)](https://terraphim.discourse.group)

Terraphim is a privacy-first AI assistant that works for you under your complete control and is fully deterministic.

You can use it as a local search engine, configured to search for different types of content on StackOverflow, GitHub, and the local filesystem using a predefined folder, which includes your Markdown files.

Terraphim operates on local infrastructure and works exclusively for the owner's benefit.

https://github.com/terraphim/terraphim-ai/assets/175809/59c74652-bab4-45b2-99aa-1c0c9b90196b


## Why Terraphim?

There are growing concerns about the privacy of data and the sharing of individuals' data across an ever-growing list of services, some of which have questionable data ethics policies. <sup>[1],[2],[3],[4]</sup>

**Individuals struggle to find relevant information in different knowledge repositories:**

- Personal ones like Roam Research, Obsidian, Coda, and Notion.
- Team-focused ones like Jira, Confluence, and SharePoint.
- Public sources such as StackOverflow and GitHub.

Terraphim aims to bridge this gap by providing a privacy-first AI assistant that operates locally on the user's hardware, enabling seamless access to various knowledge repositories without compromising privacy. With Terraphim, users can efficiently search personal, team-focused, and public knowledge sources, ensuring that their data remains under their control at all times.

[1]: https://www.coveo.com/en/resources/reports/relevance-report-workplace
[2]: https://cottrillresearch.com/various-survey-statistics-workers-spend-too-much-time-searching-for-information/
[3]: https://www.forbes.com/sites/forbestechcouncil/2019/12/17/reality-check-still-spending-more-time-gathering-instead-of-analyzing/
[4]: https://www.theatlantic.com/technology/archive/2021/06/the-internet-is-a-collective-hallucination/619320/

## Getting Started

### 🚀 Quick Install (Recommended)

#### Option 1: Docker (Easiest)
```bash
# Automated Docker installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/docker-run.sh | bash
```

#### Option 2: Binary Installation
```bash
# Automated source installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash
```

### 📚 Detailed Installation
For detailed installation instructions, see our [Installation Guide](https://github.com/terraphim/terraphim-ai/blob/main/release/v0.2.3/README.md).

### 🛠️ Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/terraphim/terraphim-ai.git
   cd terraphim-ai
   ```

2. **Set up development environment**:
   ```bash
   # Install Git hooks for code quality (optional but recommended)
   ./scripts/install-hooks.sh
   ```

3. **Start the backend server**:
   ```bash
   cargo run
   ```
   This starts an API endpoint for indexing and querying documents.

4. **Run the frontend** (choose one):

   **Web Frontend:**
   ```bash
   cd desktop
   yarn install
   yarn run dev
   ```

   **Desktop App (Tauri):**
   ```bash
   cd desktop
   yarn install
   yarn run tauri dev
   ```

   **Terminal Interface (TUI):**
   ```bash
   # Build with all features (recommended)
   cargo build -p terraphim_agent --features repl-full --release
   ./target/release/terraphim-agent

   # Or run minimal version
   cargo run --bin terraphim-agent
   ```

(See the [desktop README](desktop/README.md), [TUI documentation](docs/tui-usage.md), and [development setup guide](docs/src/development-setup.md) for more details.)

## Terminal User Interface (TUI)

Terraphim includes a comprehensive TUI that provides both interactive REPL functionality and CLI commands for advanced operations:

### Key Features

- **🤖 AI Chat Integration**: OpenRouter and Ollama support for intelligent conversations
- **⚡ Sub-2 Second VM Boot**: Advanced VM optimization system with sub-500ms allocation
- **🖥️ Enhanced VM Management**: Firecracker microVM pools with intelligent allocation
- **📝 Markdown Command System**: Extensible commands defined in YAML frontmatter
- **🔒 Security-First Execution**: Three execution modes (Local, Firecracker, Hybrid) with knowledge graph validation
- **🌐 Web Operations**: Secure web requests through VM sandboxing
- **📁 File Operations**: Semantic file analysis and intelligent content management
- **🔍 Knowledge Graph**: Interactive rolegraph visualization and navigation
- **⚙️ Configuration**: Real-time role and configuration management

### Quick Start

```bash
# Build with all features
cargo build -p terraphim_agent --features repl-full --release

# Launch interactive REPL
./target/release/terraphim-agent

# Available REPL commands:
/help           # Show all commands
/search "query" # Semantic search
/chat "message" # AI conversation
/commands list  # List available markdown commands
/deploy staging # Execute deployment (Firecracker mode)
/search "TODO"  # Execute search command (Local mode)
/vm list        # VM management with sub-2s boot
/web get URL    # Web operations
/file search    # Semantic file operations
```

For detailed documentation, see [TUI Usage Guide](docs/tui-usage.md).

## Terminology

When configuring or working on Terraphim, you will encounter the following
terms and concepts:

- **Haystack**: A data source that Terraphim can search through. For example, this
  could be a folder on your computer, a Notion workspace, or your email account.
- **Knowledge Graph**: A structured graph of information created from a
  haystack, where nodes represent entities and edges represent relationships
  between them.
- **Profile**: An endpoint for persisting user data (e.g. Amazon S3, sled, or
  rocksdb).
- **Role**: A role is a set of settings that define the default behavior of the
  AI assistant. For example, a developer role will search for code-related
  content, while a "father" role might search for parenting-related content. Each
  Terraphim role has its own separate knowledge graph that contains relevant
  concepts, with all synonyms.
- **Rolegraph**: A structure for ingesting documents into Terraphim. It is a knowledge
  graph that uses a scoring function (an Aho-Corasick automata build from the
  knowledge graph) for ranking results.

## Why "Terraphim"?

The term is originally taken from the [Relict series][relict] of science fiction
novels by [Vasiliy Golovachev](https://en.wikipedia.org/wiki/Vasili_Golovachov).
Terraphim is an artificial intelligence living inside a spacesuit (part of an
exocortex), or inside your house or vehicle, and it is designed to help you with
your tasks. You can carry it around with you.
Similar entities now common in science fiction, for example Destiny 2 has entity called [Ghost][ghost].


Or in Star Wars Jedi Survivor there is an AI assistant [BD-1][bd-1].

The compactness and mobility of such AI assistant drives the [[Design Decisions]] of Terraphim.

[bd-1]: https://starwars.fandom.com/wiki/BD-1
[ghost]: https://www.destinypedia.com/Ghost
[relict]: https://www.goodreads.com/en/book/show/196710046

Terraphim is a trademark registered in the UK, US and internationally (WIPO). All other trademarks mentioned above are the property of their respective owners.

## Configuration

### Storage Backends

Terraphim supports multiple storage backends for different deployment scenarios:

#### Local Development (Default)
The system uses local storage backends by default, requiring no additional configuration:
- **Memory**: In-memory storage for testing
- **DashMap**: High-performance concurrent storage
- **SQLite**: Local database storage
- **ReDB**: Embedded key-value database

#### Cloud Storage (Optional)
For production deployments, you can optionally enable cloud storage:

##### AWS S3 Configuration
To use AWS S3 storage, set the following environment variables:
```bash
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export TERRAPHIM_PROFILE_S3_REGION="us-east-1"
export TERRAPHIM_PROFILE_S3_ENDPOINT="https://s3.amazonaws.com/"
```

**Note**: AWS credentials are completely optional. The system will automatically fall back to local storage if AWS credentials are not available, ensuring local development works without any cloud dependencies.

### Environment Variables
- `TERRAPHIM_SETTINGS_PATH`: Override the settings file path
- `TERRAPHIM_DATA_PATH`: Set the data directory location
- `LOG_LEVEL`: Set logging verbosity (debug, info, warn, error)

## Installation Methods

### For End Users

#### Homebrew (macOS/Linux)
```bash
brew install terraphim/terraphim-ai/terraphim-ai
```
This installs the server, TUI, and desktop app (macOS only).

#### Debian/Ubuntu
```bash
# Download from GitHub releases
sudo dpkg -i terraphim-server_*.deb
sudo dpkg -i terraphim-agent_*.deb
sudo dpkg -i terraphim-ai-desktop_*.deb
```

#### Docker
```bash
docker run ghcr.io/terraphim/terraphim-server:latest
```

#### Direct Download
Download pre-built binaries from [GitHub Releases](https://github.com/terraphim/terraphim-ai/releases).

### Development Setup

For development, see our comprehensive [Development Setup Guide](docs/src/development-setup.md) which covers:
- Code quality tools and pre-commit hooks
- Multiple installation options (no Python required!)
- IDE integration and troubleshooting

## Contributing

We welcome contributions! Here's how to get started:

1. **Read our guides**:
   - [Contributing guide](CONTRIBUTING.md)
   - [Development setup](docs/src/development-setup.md)

2. **Set up your environment**:
   ```bash
   git clone https://github.com/terraphim/terraphim-ai.git
   cd terraphim-ai
   ./scripts/install-hooks.sh  # Sets up code quality tools
   ```

3. **Code quality standards**:
   - All commits must follow [Conventional Commits](https://www.conventionalcommits.org/)
   - Rust code is automatically formatted with `cargo fmt`
   - JavaScript/TypeScript uses [Biome](https://biomejs.dev/) for linting and formatting
   - No secrets or large files allowed in commits

4. **Make your changes**:
   - Create a feature branch: `git checkout -b feat/your-feature`
   - Make your changes with proper tests
   - Commit with conventional format: `git commit -m "feat: add amazing feature"`
   - Push and create a Pull Request

### Dependency Management

**Important**: Some dependencies are pinned to specific versions to ensure compatibility:

- `wiremock = "0.6.4"` - Version 0.6.5 uses unstable Rust features requiring nightly compiler
- `schemars = "0.8.22"` - Version 1.0+ introduces breaking API changes
- `thiserror = "1.0.x"` - Version 2.0+ requires code migration for breaking changes

These constraints are enforced in `.github/dependabot.yml` to prevent automatic updates that would break CI. Future upgrades require manual code migration work.

### Contributors are awesome
<a href="https://github.com/terraphim/terraphim-ai/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=terraphim/terraphim-ai" />
</a>



## License

This project is licensed under the [Apache license](LICENSE).
