# Installing Terraphim AI

Terraphim AI is distributed through two supported channels. There is no
in-repo Homebrew formula — installation is handled by the official Homebrew
tap or directly from crates.io / source.

## Option 1: Homebrew tap (recommended)

The official tap lives in a dedicated repository and covers the shipped CLI
tools (`terraphim-grep`, `terraphim-agent`, `terraphim_rlm`):

```bash
brew tap terraphim/terraphim
brew install terraphim-ai
```

> The earlier in-repo `scripts/terraphim-ai.rb` placeholder (pinned to
> `v0.1.0` with a zeroed SHA-256 and referencing binaries that have since
> moved to the polyrepo split) has been removed. Use the tap above, which
> tracks real releases. See issue #2895.

## Option 2: cargo install (from crates.io)

The CLI tools are published to crates.io:

```bash
cargo install terraphim_grep    # semantic code search CLI
cargo install terraphim_agent   # agent + REPL binary
cargo install terraphim_rlm     # role/routing language model bridge
```

## Option 3: build from source

```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release
```

Binaries are emitted to `target/release/`. See the workspace `Cargo.toml`
for the current set of buildable crates.

## Release channels — source of truth

| Channel | Canonical location | Notes |
|---------|-------------------|-------|
| crates.io | `crates.io/crates/terraphim_{grep,agent,rlm}` | Public; the canonical release channel for the CLI tools |
| GitHub release assets | `github.com/terraphim/terraphim-ai/releases` | Prebuilt signed binaries for `terraphim-grep` and `terraphim-agent` |
| Homebrew tap | `github.com/terraphim/homebrew-terraphim` | Official tap; replaces the former in-repo placeholder formula |
| Private Gitea cargo registry | `git.terraphim.cloud/.../cargo/` | Internal mirror; may lag crates.io — prefer crates.io for current versions |

**Note on mirror state:** the GitHub mirror can run *ahead* of the Gitea
mirror for `terraphim/terraphim-ai` releases. crates.io is the authoritative
source for the published CLI tool versions.
