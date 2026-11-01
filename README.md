<p align="center">
  <a href="https://terraphim.ai">
    <img alt="Terraphim AI" src="https://raw.githubusercontent.com/terraphim/terraphim.ai/main/static/images/terraphim_logo_gray.png" width="160" />
  </a>
</p>

<h1 align="center">Terraphim AI</h1>

<p align="center">
  <strong>A privacy-first, deterministic AI assistant that runs on your own hardware and works for you.</strong>
</p>

<p align="center">
  <a href="https://github.com/terraphim/terraphim-clients/releases/latest"><img alt="CLI release" src="https://img.shields.io/github/v/release/terraphim/terraphim-clients?filter=v*&label=CLI%20release" /></a>
  <a href="https://github.com/terraphim/homebrew-terraphim"><img alt="Homebrew tap" src="https://img.shields.io/badge/homebrew-terraphim%2Fterraphim-orange?logo=homebrew" /></a>
  <a href="https://crates.io/crates/terraphim_grep"><img alt="terraphim_grep on crates.io" src="https://img.shields.io/crates/v/terraphim_grep.svg?label=terraphim_grep" /></a>
  <a href="https://pypi.org/project/terraphim-automata/"><img alt="terraphim-automata on PyPI" src="https://img.shields.io/pypi/v/terraphim-automata.svg?label=terraphim-automata" /></a>
  <a href="LICENSE-Apache-2.0"><img alt="Licence: Apache-2.0" src="https://img.shields.io/badge/licence-Apache--2.0-blue.svg" /></a>
  <a href="https://discord.gg/VPJXB6BGuY"><img alt="Discord" src="https://img.shields.io/discord/852545081613615144?label=Discord&logo=discord" /></a>
  <a href="https://terraphim.discourse.group"><img alt="Discourse" src="https://img.shields.io/discourse/users?server=https%3A%2F%2Fterraphim.discourse.group" /></a>
</p>

<p align="center">
  <a href="https://terraphim.ai">Website</a> |
  <a href="https://docs.terraphim.ai">Documentation</a> |
  <a href="https://terraphim.ai/how-tos/">How-tos</a> |
  <a href="https://terraphim.ai/editor/">Browser editor</a> |
  <a href="https://github.com/orgs/terraphim/repositories">All repositories</a>
</p>

Terraphim searches your code, notes and team knowledge through role-based knowledge graphs. Matching uses Aho-Corasick automata over terms you define, so every result traces back to a concept, and the same input gives the same output. Indexing and search run locally. LLM synthesis is optional and off unless you ask for it.

This repository holds the HTTP server, the desktop front end, language bindings and project documentation. The CLI tools and foundation crates live in [related repositories](#ecosystem).

https://github.com/terraphim/terraphim-ai/assets/175809/59c74652-bab4-45b2-99aa-1c0c9b90196b

## Contents

| | | |
|---|---|---|
| [Current downloads](#current-downloads) | [Features](#features) | [Build from source](#build-from-source) |
| [Installation](#installation) | [Ecosystem](#ecosystem) | [Why Terraphim?](#why-terraphim) |
| [Quick start](#quick-start) | [Terminology](#terminology) | [Contributing](#contributing) |
| [AI coding agents](#ai-coding-agents) | [Documentation](#documentation) | [Licence](#licence) |

## Current downloads
<!-- terraphim-downloads:start -->
Verified release links as of 19 October 2026. Core services and CLI clients have
separate releases.

| Component | Release | Use |
| --- | --- | --- |
| Core project | [v1.21.3](https://github.com/terraphim/terraphim-ai/releases/tag/v1.21.3) | Stable core/source release |
| CLI clients | [v1.21.22](https://github.com/terraphim/terraphim-clients/releases/tag/v1.21.22) | Stable `terraphim-grep`, agent and client tools |
| Terraphim Editor | [v0.1.0-rc.4](https://github.com/terraphim/terraphim-editor/releases/tag/v0.1.0-rc.4) | Browser editor prerelease |
| VS Code extension | [0.1.7](https://github.com/terraphim/terraphim-ai/releases/tag/terraphim_vscode-v0.1.7) | Prerelease VSIX for compatible editors |
| Cursor plugin | [v0.2.7](https://github.com/terraphim/terraphim-cursor-plugin/releases/tag/v0.2.7) | Native Cursor plugin; separate from the VSIX catalogue |
| Stock Zed extension | [v0.2.1](https://github.com/terraphim/zed-terraphim/releases/tag/v0.2.1) | GitHub download; official Zed catalogue submission remains pending |
| Continue integration | [0.1.1](https://github.com/terraphim/terraphim-ai/releases/tag/terraphim_continue-v0.1.1) | Continue integration package |
<!-- terraphim-downloads:end -->

For future stable releases, use [latest core downloads](https://github.com/terraphim/terraphim-ai/releases/latest) or [latest CLI client downloads](https://github.com/terraphim/terraphim-clients/releases/latest). Use the explicit version links above for prereleases. [Release channels](docs/src/release-channels.md) explains how the channels relate.

## Installation

### Installer script (macOS and Linux)

```bash
# terraphim-agent only
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash

# terraphim-agent, terraphim-grep and terraphim-cli
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash -s -- --with-grep --tool terraphim-cli
```

The script verifies checksums, installs to `~/.local/bin` and adds that directory to your shell's `PATH`. Run it with `--help` for all options.

### Homebrew (macOS and Linux)

```bash
brew tap terraphim/terraphim
brew install terraphim-grep     # Local code and documentation search
brew install terraphim-agent    # Agent memory, learning and session commands
brew install terraphim-server   # HTTP API server
```

### Cargo

```bash
cargo install terraphim_grep --locked
cargo install terraphim_agent --locked
cargo install terraphim-cli --locked
```

The crates.io packages can lag behind the binaries listed under [Current downloads](#current-downloads).

### Python

```bash
pip install terraphim-automata
```

```python
import json
from terraphim_automata import build_index

thesaurus = json.dumps({"name": "Engineering", "data": {
    "machine learning": {"id": 1, "nterm": "machine learning", "url": "https://example.com/ml"},
}})

for result in build_index(thesaurus).search("mach", max_results=10):
    print(result.term, result.url)
```

See the [Python bindings guide](crates/terraphim_automata_py/README.md) for fuzzy search, link replacement and paragraph extraction.

### Docker

The public server image is `ghcr.io/terraphim/terraphim-ai:latest`. It needs an explicit role configuration and the dependencies for your haystacks. See [Released server qualification](docs/vscode-released-server-qualification.md) for the exact tested image, configuration and runtime dependencies.

## Quick start

Search a codebase. No configuration, thesaurus or API key is needed:

```bash
terraphim-grep "fn main" --haystack code --paths . --search-only
```

Check a shell command before an agent runs it:

```bash
terraphim-agent guard --json "git reset --hard"
```

Use a knowledge graph to rewrite text. Each Markdown file defines a concept and its synonyms:

```bash
terraphim-agent setup --template llm-enforcer

mkdir -p docs/src/kg
cat > docs/src/kg/bun_install.md <<'EOF'
# bun install

synonyms:: npm install, yarn install, pnpm install
EOF

echo "npm install" | terraphim-agent replace
# bun install
```

Run `terraphim-agent setup --list-templates` to see other starting roles, including local notes, Rust, Python, frontend and Quickwit log analysis. Start the offline REPL with `terraphim-agent repl`.

### From wording alternatives to search

[Try the browser editor](https://terraphim.ai/editor/): add **Concern** as an alternative to **Risk**, save the annotated Markdown, then export its knowledge graph. Knowledge notes can also declare aliases with `synonyms:: Risk, Concern`. Supply the exported graph explicitly to the released `terraphim-grep` client to retrieve related passages. Synthesis is optional.

Follow the [saved-output walkthrough](https://terraphim.ai/how-tos/synonym-search/) for the exact search commands and example files.

## AI coding agents

`terraphim-agent` adds safety checks, searchable history and durable memory to coding agents such as Claude Code, Codex and Cursor.

```bash
terraphim-agent sessions sources             # Detect Claude Code, Cursor and Aider history
terraphim-agent sessions search "rust"       # Search imported sessions
terraphim-agent learn install-hook claude    # Capture failed commands (also: codex, opencode)
terraphim-agent learn query "npm"            # Look up past failures and corrections
terraphim-agent memory capture --content "Use the project validation command."
```

<details>
<summary>Block destructive commands in Claude Code</summary>

Save this as `~/.claude/hooks/pre_tool_use.sh` and make it executable:

```bash
#!/bin/bash
set -euo pipefail
INPUT=$(cat)
[ "$(jq -r '.tool_name // empty' <<< "$INPUT")" = "Bash" ] || exit 0
COMMAND=$(jq -r '.tool_input.command // empty' <<< "$INPUT")
[ -n "$COMMAND" ] || exit 0

GUARD=$(terraphim-agent guard --json <<< "$COMMAND" 2>/dev/null || echo '{"decision":"allow"}')
if [ "$(jq -r '.decision' <<< "$GUARD")" = "block" ]; then
  jq -n --arg reason "$(jq -r '.reason' <<< "$GUARD")" \
    '{hookSpecificOutput: {hookEventName: "PreToolUse", permissionDecision: "deny", permissionDecisionReason: $reason}}'
fi
```

Then register it in `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{ "type": "command", "command": "~/.claude/hooks/pre_tool_use.sh" }]
    }]
  }
}
```

</details>

Further guides:

- [AI Agent Onboarding Quickstart](docs/src/howto/ai-agent-onboarding-quickstart.md) — Orientation for agents working in this repository.
- [Claude Code integration](examples/TERRAPHIM_CLAUDE_INTEGRATION.md) — Hooks and skills for knowledge graph replacement.
- [MCP integration](docs/src/mcp-integration.md) — Connect Terraphim to MCP clients using `terraphim_mcp_server`.
- [Codebase quality evaluation](examples/codebase-evaluation/) — Compare a codebase before and after an agent's changes.

## Features

- **Role-based knowledge graphs** — Each role has its own concepts, synonyms and data sources.
- **Deterministic ranking** — Aho-Corasick automata and graph embeddings, with no neural vectors or GPU.
- **Many haystacks** — Local files, code, Quickwit logs and other providers, configured per role.
- **Local by default** — Storage is local unless you opt in to a cloud profile such as Amazon S3.
- **Optional LLM support** — Chat and synthesis through OpenRouter or Ollama when you enable them.
- **Agent safety** — Guard patterns, knowledge graph replacement, failure capture and memory lifecycle commands.
- **Self-updating CLI** — `terraphim-agent check-update` and `terraphim-agent update`. See [auto-update](docs/autoupdate.md).

## Ecosystem

Terraphim was split into focused repositories, so many crates that once lived here now have their own home. The [full list of public repositories](https://github.com/orgs/terraphim/repositories) also includes experiments and third-party forks.

### Clients and distribution

- [terraphim-clients](https://github.com/terraphim/terraphim-clients) — `terraphim-grep`, `terraphim-agent`, `terraphim-cli`, MCP server, LSP server and session search.
- [homebrew-terraphim](https://github.com/terraphim/homebrew-terraphim) — Homebrew tap for the agent, grep and server.

### Foundation crates

- [terraphim-core](https://github.com/terraphim/terraphim-core) — Types, Markdown parser, automata and role graph. Builds from crates.io without private registry credentials.
- [terraphim-config-persistence](https://github.com/terraphim/terraphim-config-persistence) — Role configuration and multi-backend persistence.
- [terraphim-service](https://github.com/terraphim/terraphim-service) — Service layer, middleware, routing, haystack providers and session analysis.
- [terraphim-agents](https://github.com/terraphim/terraphim-agents) — Multi-agent orchestration.
- [terraphim-kg-agents](https://github.com/terraphim/terraphim-kg-agents) — Knowledge graph agents, linting and codebase evaluation.

### Editors and integrations

- [terraphim-editor](https://github.com/terraphim/terraphim-editor) — Markdown editor in Rust and WebAssembly that keeps alternative wordings. Prerelease.
- [zed-terraphim](https://github.com/terraphim/zed-terraphim) — Zed adapter for knowledge graphs, synonyms and Markdown annotations.
- [terraphim-cursor-plugin](https://github.com/terraphim/terraphim-cursor-plugin) — Cursor skills for search, synonyms, learning and memory.
- [nimbalyst-terraphim](https://github.com/terraphim/nimbalyst-terraphim) — Nimbalyst editor components. In development.
- [VS Code extension](vs-code-terraphim-it/README.md) — In this repository. Prerelease.
- [Continue integration](integrations/continue/README.md) — In this repository.
- [Browser extensions](browser_extensions/INSTALL.md) — In this repository.

### Desktop

- [terraphim-ai-desktop](https://github.com/terraphim/terraphim-ai-desktop) — Tauri desktop application.
- [terraphim-gpui-desktop](https://github.com/terraphim/terraphim-gpui-desktop) — Native desktop application using GPUI. Experimental.
- [terraphim-gpui-editor](https://github.com/terraphim/terraphim-gpui-editor) — GPUI editor with slash commands and block editing. Experimental.

### Websites and documentation

- [terraphim.ai](https://github.com/terraphim/terraphim.ai) — Source for the [project website](https://terraphim.ai).
- [terraphim.rs](https://github.com/terraphim/terraphim.rs) — Source for [terraphim.rs](https://terraphim.rs).
- [md-book](https://github.com/terraphim/md-book) — Rust documentation generator.

### Developer tooling

- [gitea-robot](https://github.com/terraphim/gitea-robot) — Gitea CLI with PageRank-based issue prioritisation.
- [gitea](https://github.com/terraphim/gitea) — Gitea fork customised for agent-native development.
- [pi_agent_rust](https://github.com/terraphim/pi_agent_rust) — Fork of a Rust AI coding agent CLI.
- [terraphim-essentialist-design](https://github.com/terraphim/terraphim-essentialist-design) — Design system skill for Terraphim interfaces.

## Terminology

- **Haystack** — A data source that Terraphim can search, such as a local folder, a Notion workspace or an email account.
- **Knowledge graph** — A structured graph built from a haystack. Nodes represent entities and edges represent the relationships between them.
- **Profile** — An endpoint for persisting user data, such as Amazon S3, sled or RocksDB.
- **Role** — A set of settings that defines how the assistant behaves. A developer role searches code-related content; a parent's role might search parenting advice. Each role has its own knowledge graph of concepts and synonyms.
- **Rolegraph** — The structure that ingests documents into Terraphim. It is a knowledge graph that ranks results with a scoring function built from an Aho-Corasick automaton.

## Documentation

- [Documentation site](https://docs.terraphim.ai) — Guides and reference.
- [Quickstart](QUICKSTART.md) — Fast paths for getting running.
- [Installation and deployment](docs/installation.md) — Platform packages and deployment options.
- [Architecture](docs/src/Architecture.md) — How the system fits together.
- [TUI usage](docs/tui-usage.md) — Terminal interface reference.
- [Quickwit log search](examples/quickwit-log-search.md) — Incident investigation walkthrough.
- [Runtime validation hooks](.docs/runtime-validation-hooks.md) — Guard and validation design.
- [Changelog](https://docs.terraphim.ai/src/changelog.html) — Release history.

## Build from source

> [!IMPORTANT]
> This workspace resolves some dependencies from the private `terraphim` Cargo registry, so `cargo build` needs a token in `CARGO_REGISTRIES_TERRAPHIM_TOKEN`. Without one, use the released binaries above, or start from [terraphim-core](https://github.com/terraphim/terraphim-core), which builds from crates.io alone.

```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
./scripts/install-hooks.sh    # Optional code quality hooks
cargo run                     # Starts the HTTP API server
```

To run the web front end:

```bash
cd desktop
yarn install
yarn run dev
```

The [development setup guide](docs/src/development-setup.md) covers tooling and IDE integration.

## Why Terraphim?

People struggle to find information spread across personal tools such as Obsidian and Notion, team tools such as Jira and Confluence, and public sources such as StackOverflow and GitHub.<sup>[1],[2],[3]</sup> Sending all of it to third-party services raises real concerns about privacy.<sup>[4]</sup> Terraphim bridges those sources from your own hardware, so your data stays under your control.

The name comes from the [Relict series][relict] of science fiction novels by [Vasiliy Golovachev](https://en.wikipedia.org/wiki/Vasili_Golovachov). A Terraphim is an artificial intelligence living inside a spacesuit as part of an exocortex, or inside your house or vehicle, and it helps you with your tasks wherever you are. Similar companions appear elsewhere in science fiction, such as the [Ghost][ghost] in Destiny 2 and [BD-1][bd-1] in Star Wars Jedi: Survivor. That compactness and mobility drive Terraphim's design.

[1]: https://www.coveo.com/en/resources/reports/relevance-report-workplace
[2]: https://cottrillresearch.com/various-survey-statistics-workers-spend-too-much-time-searching-for-information/
[3]: https://www.forbes.com/sites/forbestechcouncil/2019/12/17/reality-check-still-spending-more-time-gathering-instead-of-analyzing/
[4]: https://www.theatlantic.com/technology/archive/2021/06/the-internet-is-a-collective-hallucination/619320/
[relict]: https://www.goodreads.com/en/book/show/196710046
[ghost]: https://www.destinypedia.com/Ghost
[bd-1]: https://starwars.fandom.com/wiki/BD-1

## Contributing

Contributions are welcome. Read the [contributing guide](CONTRIBUTING.md) first. Commits follow [Conventional Commits](https://www.conventionalcommits.org/), Rust is formatted with `cargo fmt`, JavaScript and TypeScript use [Biome](https://biomejs.dev/), and secrets and large files must stay out of commits.

Join the discussion on [Discourse](https://terraphim.discourse.group) or [Discord](https://discord.gg/VPJXB6BGuY).

<a href="https://github.com/terraphim/terraphim-ai/graphs/contributors">
  <img alt="Contributors" src="https://contrib.rocks/image?repo=terraphim/terraphim-ai" />
</a>

## Licence

This project is licensed under the [Apache License 2.0](LICENSE-Apache-2.0). Related repositories carry their own licences.

Terraphim is a trademark registered in the UK, US and internationally (WIPO). All other trademarks mentioned above are the property of their respective owners.
