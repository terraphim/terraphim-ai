# Runner Command Policy
#
# Deployed override for the Gitea runner. Set RUNNER_TAXONOMY_DIR to this
# directory to use this file instead of the embedded default.
#
# Edit this file to change which commands the runner allows without requiring
# a Rust code change, PR, rebuild, or redeploy. Restart the runner after
# editing.

## Allowed Commands
allow:: cargo, make, bun, bunx, npm, yarn, pnpm, rch, sccache
allow:: echo, mkdir, git, ls, cat, cd, cp, mv, rm, chmod
allow:: sh, bash, test, export, source, true, set, rustup

## Denied Commands (security -- overrides allow)
deny:: docker, curl, wget, nc, ncat, python, python3, perl, ruby

## RCH Routing (cargo compilation subcommands offloaded to rch farm)
route_to:: rch, cargo, build check clippy doc
