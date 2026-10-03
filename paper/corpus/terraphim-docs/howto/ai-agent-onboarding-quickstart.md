# AI Agent Onboarding Quickstart

This quickstart is for AI coding agents working in `github.com/terraphim/terraphim-ai`. It describes the minimum safe workflow for becoming useful quickly without losing context, duplicating work, or bypassing project quality gates.

## First five minutes

1. Confirm where you are working.

   ```bash
   pwd
   git status --short
   git branch --show-current
   ```

2. Read the repository instructions before changing code.

   ```bash
   git ls-files 'AGENTS.md' 'CLAUDE.md' '.docs/summary.md' 'docs/src/howto/ai-agent-onboarding-quickstart.md'
   ```

3. Check whether the work is already in progress.

   ```bash
   git fetch origin
   git branch -r
   gtr list-pulls --owner terraphim --repo terraphim-ai --state open
   ```

4. Inspect existing tasks before creating new ones.

   ```bash
   gtr ready --owner terraphim --repo terraphim-ai
   gtr triage --owner terraphim --repo terraphim-ai
   ```

5. Search the codebase with Terraphim tools first.

   ```bash
   terraphim-grep "symbol or behaviour" --haystack code
   terraphim-grep "symbol or behaviour" --haystack code -C 3
   ```

## Working rules

- Treat Gitea as the source of truth for task tracking.
- Claim or update the relevant issue before substantial implementation work.
- Prefer the smallest correct change.
- Do not revert user or other-agent changes unless explicitly asked.
- Use `terraphim-grep` for content search before falling back to lower-level tools.
- Use `gtr` for issue and pull request operations.
- Keep secrets out of the repository; use 1Password injection or existing secrets configuration.
- Do not overwrite `.env` files.
- Do not use mocks in tests.
- Keep local, GitHub, and Gitea state aligned when publishing changes.

## Common commands

### Search

```bash
terraphim-grep "DeviceStorage::init" --haystack code -C 3
terraphim-grep "session persistence" --paths . --thesaurus .terraphim/thesaurus.json -n 8 -C 2
```

Use file-name lookup only when you are looking for paths rather than content:

```bash
git ls-files '*orchestrator*.rs'
fd 'config.toml' crates/
```

### Task management

```bash
gtr ready --owner terraphim --repo terraphim-ai
gtr view-issue --owner terraphim --repo terraphim-ai --index ISSUE_NUMBER
gtr comment --owner terraphim --repo terraphim-ai --index ISSUE_NUMBER --body "Progress update"
gtr list-pulls --owner terraphim --repo terraphim-ai --state open
```

### Rust backend

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For focused work, prefer package-scoped commands:

```bash
cargo test -p terraphim_service
cargo clippy -p terraphim_agent --all-targets -- -D warnings
```

### Frontend

```bash
cd desktop
bun install
bun run check
bun test
```

## Update and release checks

Terraphim client binaries are released from `terraphim/terraphim-clients`. Autoupdate validation should check both binaries:

```bash
terraphim-grep --version
terraphim-grep check-update
terraphim-grep update

terraphim-agent --version
terraphim-agent check-update
terraphim-agent update
```

Release archives used by autoupdate must be embedded-signed with zipsign. If update verification fails with `could not find read signatures in .tar.gz file`, the published archive is unsigned and the release pipeline must sign tar archives before upload.

## Quality gate before handoff

Before reporting completion, collect evidence rather than relying on intent:

```bash
git status --short
cargo fmt --check
cargo test -p CHANGED_PACKAGE
cargo clippy -p CHANGED_PACKAGE --all-targets -- -D warnings
```

For changed files, run the bug scanner where available:

```bash
ubs $(git diff --name-only --cached)
```

If code changed, check coverage with the most focused feasible command, for example:

```bash
cargo llvm-cov -p CHANGED_PACKAGE --summary-only
```

## Handoff checklist

- Summarise what changed and why.
- List tests, linters, coverage, and release checks that ran.
- State any blocked work or follow-up issues.
- Update the Gitea issue or pull request with the same evidence.
- Leave unrelated working tree changes untouched.
