# Terraphim System Operator Configuration

This document describes how to run Terraphim with the **System Operator** role, which indexes a Logseq-formatted MBSE vocabulary from [terraphim/system-operator](https://github.com/terraphim/system-operator) and ranks results using Terraphim's knowledge graph.

The System Operator role is the canonical Logseq-based KG demo. For a local-first companion role that searches personal email and notes, see the [Personal Assistant role how-to](../docs/src/howto/personal-assistant-role.md) -- same engine, different haystacks.

## Overview

The System Operator configuration provides:

- **Logseq knowledge graph** from a public repository: 1,300+ Logseq markdown pages, ~50 of which carry Terraphim-format `synonyms::` lines.
- **GitHub document integration**: `setup_system_operator.sh` clones the repository to a durable local path.
- **TerraphimGraph ranking**: Aho-Corasick automaton built from the synonym files, so queries like `RFP` normalise to `acquisition need` with rank reflecting graph depth.
- **Systems engineering focus**: MBSE vocabulary around requirements, architecture, verification, validation, life cycle concepts.

## Prerequisites

- Rust and Cargo installed
- Git
- Internet connection for the initial clone and optional remote-thesaurus fetch
- ~200 MB free disk space for the repository

## Quick start

### 1. Set up the repository

```bash
./scripts/setup_system_operator.sh
```

This clones `terraphim/system-operator` to `~/.config/terraphim/system_operator` by default (override with `SYSTEM_OPERATOR_DIR=<path>`). The previous default of `/tmp/system_operator` lost the clone on reboot; the new path survives restarts.

The script prints the page count, synonym-file count, and the commands for the next step.

### 2. Drive the role via terraphim_server

```bash
cargo run --bin terraphim_server -- --config terraphim_server/default/system_operator_config.json
```

The server binds `http://127.0.0.1:8000`. Update the config's `knowledge_graph_local.path` and haystack `location` if you chose a non-default `SYSTEM_OPERATOR_DIR`.

### 3. Drive the role via the terraphim-agent CLI

Add this entry to `~/.config/terraphim/embedded_config.json` under `roles`:

```json
"System Operator": {
  "shortname": "SysOps",
  "name": "System Operator",
  "relevance_function": "terraphim-graph",
  "terraphim_it": true,
  "theme": "superhero",
  "kg": {
    "automata_path": null,
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "/Users/<you>/.config/terraphim/system_operator/pages"
    },
    "public": true,
    "publish": true
  },
  "haystacks": [
    {
      "location": "/Users/<you>/.config/terraphim/system_operator/pages",
      "service": "Ripgrep",
      "read_only": true
    }
  ],
  "llm_enabled": false
}
```

Reload and query:

```bash
terraphim-agent config reload
terraphim-agent search --role "System Operator" --limit 5 "RFP"
terraphim-agent validate --role "System Operator" --connectivity "RFP business analysis life cycle model"
```

The connectivity check prints the canonical terms each query word matched to -- proof that the KG is actually driving the search.

### 4. Run the integration test

```bash
cargo test --test system_operator_integration_test -- --nocapture
```

## Configuration files

### Core

- `terraphim_server/default/system_operator_config.json` -- server config
- `terraphim_server/default/settings_system_operator_server.toml` -- settings with S3 profiles

### Generated

- `~/.config/terraphim/system_operator/pages/` -- 1,300+ markdown files from the repository
- Optional remote KG: `https://staging-storage.terraphim.io/thesaurus_Default.json`

## Roles in the server config

| Role | Relevance | Theme | KG | Haystack |
| --- | --- | --- | --- | --- |
| System Operator (default) | TerraphimGraph | superhero | Logseq KG | Ripgrep over pages/ |
| Engineer | TerraphimGraph | lumen | Logseq KG | Ripgrep over pages/ |
| Default | TitleScorer | spacelab | none | Ripgrep over pages/ |

## Remote thesaurus (optional)

The server config supports a pre-built automaton at `https://staging-storage.terraphim.io/thesaurus_Default.json` for faster cold starts. The `embedded_config` format expects `automata_path: null` (build locally). Pick one path per role -- do not mix.

## API usage

### Health check

```bash
curl http://127.0.0.1:8000/health
```

### Search

```bash
curl "http://127.0.0.1:8000/documents/search?q=MBSE&role=System%20Operator&limit=5"
```

Try these MBSE queries to exercise the KG:

- `requirements`
- `architecture`
- `verification`
- `RFP` (expands to `acquisition need`)
- `life cycle model` (expands to `life cycle concepts`)

### Configuration

```bash
curl http://127.0.0.1:8000/config
```

## Expected performance

- Remote KG load: ~2-3 s
- Local document indexing: ~5-10 s for 1,300 files
- Cold start total: ~15 s
- Warm search: <100 ms per query
- In-memory index size: ~50 MB

## Troubleshooting

- **Clone failed** -- run `git clone https://github.com/terraphim/system-operator.git ~/.config/terraphim/system_operator` manually.
- **Remote KG not loading** -- check `curl https://staging-storage.terraphim.io/thesaurus_Default.json`; set `automata_path: null` to build locally from `pages/`.
- **No search results** -- confirm `pages/` contains markdown, check server logs, ensure the role uses `terraphim-graph`.
- **Port in use** -- `lsof -i :8000`; start with `--addr 127.0.0.1:8080`.
- **Debug logging** -- `RUST_LOG=debug cargo run --bin terraphim_server -- --config ...`.

## Updating documents

```bash
git -C ~/.config/terraphim/system_operator pull --ff-only origin main
# Restart the server (or run `terraphim-agent config reload`) to re-index.
```

## Production deployment

### Environment variables

```bash
export TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8000"
export TERRAPHIM_SERVER_API_ENDPOINT="https://your-domain.com/api"
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
```

### Docker

```bash
docker build -t terraphim-system-operator .
docker run -p 8000:8000 \
  -v ~/.config/terraphim/system_operator:/data/system_operator:ro \
  terraphim-system-operator
```

Adjust the container's config path to match the mount point.

## Related

- [Personal Assistant role how-to](../docs/src/howto/personal-assistant-role.md) -- private, per-user companion pattern
- [Terraphim configuration guide](../docs/src/Configuration.md)
- [Knowledge graph documentation](../docs/src/kg/)
- [API reference](../docs/src/API.md)
- [System Operator repository](https://github.com/terraphim/system-operator)

## Contributing

1. Fork the repository.
2. Edit configuration files.
3. Run the integration test suite.
4. Open a pull request.

## Licence

This configuration is part of the Terraphim project and follows the same Apache-2.0 licence.
