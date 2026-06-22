# Runner Command Policy (Embedded Default)
#
# This file is compiled into the runner binary via include_str!.
# To override at runtime, set RUNNER_TAXONOMY_DIR to a directory
# containing a command_policy.md file.

## Allowed Commands
allow:: cargo, make, bun, bunx, npm, yarn, pnpm, rch, sccache
allow:: echo, mkdir, git, ls, cat, cd, cp, mv, rm, chmod
allow:: sh, bash, test, export, source, true, set, rustup

## Denied Commands (security -- overrides allow)
deny:: docker, curl, wget, nc, ncat, python, python3, perl, ruby

## RCH Routing (cargo compilation subcommands offloaded to rch farm)
route_to:: rch, cargo, build check clippy doc
