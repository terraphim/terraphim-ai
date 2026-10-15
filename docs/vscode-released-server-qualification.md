# Released VS Code extension with a real Terraphim server

Terraphim It 0.1.4 was checked on 15 October 2026 in actual VS Code 1.126.0 on Linux ARM64 under Xvfb, using the public Terraphim server 1.20.5. The extension was installed from its released VSIX into a disposable profile. These checks use native extension APIs and actual saved files; they do not qualify the visible completion popup or all 41 Alternative Control UI requirements.

## Server image and dependencies

The public workflow publishes to `ghcr.io/terraphim/terraphim-ai`, as configured by `IMAGE_NAME: ${{ github.repository }}` in [docker-multiarch.yml](../.github/workflows/docker-multiarch.yml). The older README image name, `ghcr.io/terraphim/terraphim-server:latest`, returned `manifest unknown` during this check.

The exact tested ARM64 image is:

```text
ghcr.io/terraphim/terraphim-ai@sha256:022c6fff5fa9ab08648306cf880216c046dd8e2f1f00f403efbb920115cbe3ff
```

Its label identifies source `e392973caa5ada2a32739d38be065e9a00cecdb0`, version `1.20.5-ubuntu22.04`; the executable reports `terraphim_server 1.20.5` and has SHA-256 `6ff480b445644eebfdbfac77eade4e5ac1897b054cdf0356208948c6ddfb5de5`. This is a pinned server release, not a claim about the latest server source.

The image lacked the external `rg` command. The test image installed Ubuntu's `ripgrep` package for its `Ripgrep` haystack, plus Xvfb and the normal VS Code GUI libraries. It did not modify the server executable. Supply an explicit `--config /fixture/config.json`; the image's default command references a configuration file that was absent in the inspected image. Use disposable container storage rather than mounting your normal Terraphim settings or credentials.

## Configure roles intentionally

A graph-backed role provides the autocomplete vocabulary. A title-scored role can provide document-search fallback even when it has no autocomplete graph. Use lowercase, matching role keys and `name` values.

For the tested `projectalpha` role, `/fixture/projectalpha/kg/Risk.md` contained:

```markdown
# Risk
synonyms:: Concern
```

The separate corpus file `/fixture/projectalpha/corpus/supplier.md` contained:

```markdown
# Supplier plan

Supplier risk requires a backup supplier.
```

The role configuration was:

```json
{
  "name": "projectalpha",
  "shortname": "projectalpha",
  "relevance_function": "terraphim-graph",
  "terraphim_it": true,
  "theme": "lumen",
  "kg": {
    "automata_path": null,
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "/fixture/projectalpha/kg"
    },
    "public": false,
    "publish": false
  },
  "haystacks": [{
    "location": "/fixture/projectalpha/corpus",
    "service": "Ripgrep",
    "read_only": true,
    "atomic_server_secret": null,
    "extra_parameters": {}
  }],
  "llm_auto_summarize": false,
  "extra": {}
}
```

Place that object in the server config's `roles.projectalpha`, with `id: "Server"`, `default_role: "projectalpha"`, and `selected_role: "projectalpha"`. The second graph role, `projectbeta`, used its own corpus and `Cost.md` with `synonyms:: Expense`. The `supplierfallback` role used the first corpus with `relevance_function: "title-scorer"` and `kg: null`. No provider credentials or synthesis were enabled.

## Extension settings and actual results

In the disposable VS Code profile, set:

```json
{
  "terraphimIt.serverUrl": "http://127.0.0.1:8000",
  "terraphimIt.enableLocalServer": true,
  "terraphimIt.selectedRole": "projectalpha"
}
```

The server and VS Code ran inside the same network-disabled container, so this loopback address reached the real service without exposing a host port. When running them separately, configure an explicit reachable server URL; disabling local-server access requires a non-loopback URL.

Run **Terraphim Server Autocomplete**. The native check invoked that actual command and then `vscode.executeCompletionItemProvider` on real files. It selected returned text through a native workspace edit and saved each file. The outcomes were:

| Role | Typed query | Actual completion and saved text |
|---|---|---|
| `projectalpha` | `co` | `concern` |
| `projectbeta` | `co` | `cost` |
| `projectalpha` again | `co` | `concern` |
| `supplierfallback` | `suppl` | `supplier` |

The last row exercised the actual `/autocomplete/supplierfallback/suppl` response followed by `POST /documents/search`. Its completion detail was `# Supplier plan`, and its documentation linked the real corpus file. The legacy **Terraphim Search Autocomplete (Server)** command also activated and saved the same fallback result. Unchanged query text across role changes did not reuse stale suggestions.

Both commands activated only after a successful real `/health` response. During verification, title-scored roles returned “Role not found” from autocomplete while document search worked. Choose a graph role when you expect KG autocomplete; do not mistake an empty suggestion list for a successful graph lookup.

The released VSIX SHA-256 was `f29d23a37683127c1050002fee83045027f9dda5642f892eaf7d2b1fbc8b0af5`. All 440 installed payload files matched, allowing only installer metadata in `package.json`. Five completion JSON files and five actual saved Markdown files were checked. Vendor Node deprecation and offline-host warnings were retained. Visible suggestion selection, health notifications, other editor hosts and the full Alternative Control UI remain separate acceptance checks.
