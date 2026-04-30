# Design & Implementation Plan: Documentation and Website Alignment

## 1. Summary of Target Behavior

After implementation:
- Every command shown on terraphim.ai runs without error against terraphim-agent v1.17.0
- The Quickstart describes the actual REPL experience (slash commands, actual output)
- Version numbers are consistent: config.toml, download links, and release page all agree
- No documented features exist that are not implemented in the binary
- All internal links resolve without 404
- The terraphim-agent capability page matches the actual CLI subcommands

## 2. Key Invariants and Acceptance Criteria

| # | Invariant | Acceptance Criteria |
|---|-----------|---------------------|
| 1 | Quickstart REPL commands work | Each `/command` in Quickstart runs in `terraphim-agent repl` without error |
| 2 | CLI subcommands documented = CLI subcommands present | `terraphim-agent --help` output matches what docs describe |
| 3 | Version consistency | `config.toml` version matches the latest GitHub release tag |
| 4 | No 404s for internal links | Every `[link](/path)` in website content resolves to an existing page |
| 5 | Download URLs point to valid assets | Each download URL on installation page returns 200 |

## 3. High-Level Design and Boundaries

### Two Repositories, Two Deploys

```
terraphim.ai/ (Zola website)
  content/docs/quickstart.md        <-- REWRITE
  content/docs/installation.md       <-- UPDATE version refs
  content/capabilities/terraphim-agent.md  <-- REMOVE evaluate
  content/releases.md                <-- UPDATE version
  config.toml                        <-- UPDATE version
  content/_index.md                  <-- UPDATE download links

terraphim-ai/ (main repo)
  docs/src/tui.md                    <-- ALREADY ACCURATE (reference)
```

### Changes Inside Existing Components

- **config.toml**: Bump `version` and `release_version`
- **quickstart.md**: Full rewrite based on actual REPL output
- **terraphim-agent.md**: Remove `evaluate` bullet point
- **installation.md**: Update version refs in download URLs
- **releases.md**: Update "Latest Stable" version
- **_index.md**: Update download links on homepage

### No New Components

All changes are edits to existing files.

## 4. File/Module-Level Change Plan

### Repository: terraphim.ai

| File | Action | Before | After |
|------|--------|--------|-------|
| `config.toml` | Modify | `version = "1.16.0"`, `release_version = "1.16.31"` | `version = "1.17.0"`, `release_version = "1.17.0"` |
| `content/docs/quickstart.md` | Rewrite | Fabricated REPL with `> search`, `> import`, `> connect` | Actual REPL with `/search`, `/role select`, `/config show`, actual output |
| `content/capabilities/terraphim-agent.md` | Modify | Documents `evaluate` subcommand | Remove `evaluate` bullet; add `listen` and `extract` |
| `content/docs/installation.md` | Modify | Version refs `1.16.32` in URLs and file names | Update to current release version; use `/latest/download/` pattern |
| `content/releases.md` | Modify | "Latest Stable: v1.16.31" | "Latest Stable: v1.17.0" |
| `content/_index.md` | Modify | Download links with v1.16.31 | Update to current version |

### Repository: terraphim-ai

| File | Action | Before | After |
|------|--------|--------|-------|
| No changes needed | -- | Local docs (`docs/src/howto/*.md`) are already accurate | -- |

## 5. Step-by-Step Implementation Sequence

### Step 1: Update config.toml version numbers
**Purpose**: Single source of truth for version across the site
**File**: `terraphim.ai/config.toml`
**Deployable?**: Yes -- Zola rebuilds from config

Change:
```toml
version = "1.17.0"
release_version = "1.17.0"
```

### Step 2: Rewrite the Quickstart guide
**Purpose**: Replace fabricated REPL experience with actual commands
**File**: `terraphim.ai/content/docs/quickstart.md`
**Deployable?**: Yes

Target content structure (based on actual REPL v1.17.0):

```markdown
## Step 1: Install (keep existing -- accurate)

## Step 2: Start the REPL
terraphim-agent repl
(show actual welcome banner)

## Step 3: Search
/search rolegraph
(show actual output pattern)

## Step 4: Explore roles
/role list
/role select <name>
/search <query>

## Step 5: CLI subcommands (automation)
terraphim-agent search "query" --role "AI Engineer" --limit 5
terraphim-agent learn list
terraphim-agent sessions sources

## Step 6: Next steps
(link to howtos, command rewriting, MCP integration)
```

Key changes from current:
- Remove all `> search`, `> import`, `> connect`, `> export`, `> source add` commands
- Remove `terraphim_server` requirement (REPL works offline)
- Remove `terraphim-cli` references (not published)
- Use actual `/slash` command syntax
- Show actual output patterns, not fabricated ones

### Step 3: Fix terraphim-agent capability page
**Purpose**: Remove documented-but-missing `evaluate` subcommand
**File**: `terraphim.ai/content/capabilities/terraphim-agent.md`
**Deployable?**: Yes

Remove the bullet:
```
- **Evaluation framework**: `evaluate` measures automata classification accuracy...
```

Add bullets for actual undocumented features:
```
- **Listener dispatch**: the listener executes `terraphim-agent` subcommands...
```
(This bullet already exists -- just remove the evaluate one)

### Step 4: Update installation page version refs
**Purpose**: Download URLs point to correct release assets
**File**: `terraphim.ai/content/docs/installation.md`
**Deployable?**: Yes

Strategy: Replace hardcoded version numbers in download URLs with a note:
"Replace X.Y.Z with the [latest release](https://github.com/terraphim/terraphim-ai/releases/latest) version."

Or use the `/latest/download/` GitHub URL pattern which always resolves.

### Step 5: Update releases page
**Purpose**: Current "Latest Stable" matches reality
**File**: `terraphim.ai/content/releases.md`
**Deployable?**: Yes

Update "Latest Stable" to current version.

### Step 6: Update homepage download links
**Purpose**: Download buttons on homepage point to correct assets
**File**: `terraphim.ai/content/_index.md`
**Deployable?**: Yes (if download links are in template, check templates/)

Check if download links are hardcoded in `_index.md` or in `templates/`. Update accordingly.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Method |
|---------------------|-----------|-------------|
| Quickstart commands work | Manual REPL test | Start `terraphim-agent repl`, run each `/command` shown |
| No 404 internal links | Automated | `zola build` + link checker, or manual crawl |
| Version consistency | Grep | `rg "1\\.16\\." content/` returns 0 results |
| Download URLs valid | Manual | Click each download link, verify 200 |
| `evaluate` not mentioned | Grep | `rg "evaluate" content/` returns 0 results in agent page |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Next release invalidates docs again | Add "as of vX.Y.Z" footnotes; add version to release checklist | Low -- process improvement |
| Quickstart still doesn't cover all features | Focus on 5 most common commands; link to detailed docs | Low |
| Download URL pattern changes | Use `/latest/download/` pattern instead of versioned | Low |
| Homepage download links in template not content | Check `templates/` directory for hardcoded versions | None once identified |

## 8. Open Questions / Decisions for Human Review

1. **Version to pin**: Should docs say v1.17.0 (latest build) or wait for a formal release tag?
2. **`evaluate` subcommand**: Remove from docs entirely, or keep with a "coming soon" note?
3. **`terraphim-cli`**: Should installation page still mention `cargo install terraphim-cli` if it's not published?
4. **Server requirement**: Should Quickstart mention `terraphim_server` at all, or focus purely on offline REPL?
5. **Homepage download section**: Are download links in `content/_index.md` or in a template file under `templates/`?

## 9. Separate Code Bug to File

The thesaurus panic at `thesaurus/mod.rs:53` is a code bug, not a documentation issue. It should be filed as a GitHub issue:

**Title**: `terraphim-agent search` panics when `default_role` not in roles map
**Root cause**: `unwrap_or(&roles[&default_role])` panics because `default_role = "Terraphim Engineer"` but the roles map only contains `"AI Engineer"` on Linux
**Fix**: Replace with `.unwrap_or_else(|| roles.values().next().expect("at least one role"))` or similar safe fallback
