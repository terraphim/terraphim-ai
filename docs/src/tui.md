# Terraphim TUI

Terraphim includes a terminal user interface (TUI) that mirrors key features of the desktop app and exposes a CI-friendly CLI.

## Installation

Build from the workspace:

```bash
cargo build -p terraphim_tui --release
```

Binary: `terraphim-tui`

Set the server URL (defaults to `http://localhost:8000`):

```bash
export TERRAPHIM_SERVER=http://localhost:8000
```

## Interactive mode

```bash
terraphim-tui
```

- Input box: type a query and press Enter to search `/documents/search`.
- Suggestions: shows top matches from rolegraph node labels for quick completion.
- Results: renders ranks and titles in-pane.

## CLI subcommands

- Search
  ```bash
  terraphim-tui search --query "terraphim-graph" --role "Default" --limit 10
  ```
- Roles
  ```bash
  terraphim-tui roles list
  terraphim-tui roles select "Default"
  ```
- Config
  ```bash
  terraphim-tui config show
  terraphim-tui config set selected_role=Default
  terraphim-tui config set global_shortcut=Ctrl+X
  terraphim-tui config set role.Default.theme=spacelab
  ```
- Rolegraph (ASCII)
  ```bash
  terraphim-tui graph --role "Default" --top-k 10
  # Prints: - [rank] label -> neighbor1, neighbor2, ...
  ```
- Chat (optional OpenRouter)
  ```bash
  terraphim-tui chat --role "Default" --prompt "Summarize terraphim graph" --model anthropic/claude-3-sonnet
  ```

## Behavior

- Uses `/config`, `/config/selected_role`, `/documents/search`, and `/rolegraph` endpoints.
- Chat posts to `/chat` (requires server compiled with openrouter feature and configured role or `OPENROUTER_KEY`).
- Suggestions source labels from `/rolegraph` for the selected role.

## Roadmap

- Expand `config set` key coverage and validation.
- ASCII graph filters and alternative sort metrics.
- Streaming chat into the TUI pane.
- Thesaurus-backed suggestions when published by role config.

