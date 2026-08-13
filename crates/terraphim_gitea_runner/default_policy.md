# Runner Command Policy (Embedded Default)
#
# This file is compiled into the runner binary via include_str!.
# To override at runtime, set RUNNER_TAXONOMY_DIR to a directory
# containing a command_policy.md file.
#
## Repository-script convention (Refs #3222)
#
# Classification inspects the literal first token of a step's command, after
# stripping leading environment assignments. Repository-relative paths are not
# resolved, normalised, or followed. Workflows therefore invoke repository
# scripts through the allowlisted interpreter:
#
#     - run: bash ./scripts/check.sh      # supported
#     - run: ./scripts/check.sh           # rejected before execution
#
# This is a guardrail, not a shell sandbox. Because `bash` is allowlisted, the
# utilities a repository script runs internally are NOT independently inspected;
# the check bounds what a workflow author may name directly, and nothing more.
# Direct-path support is deliberately rejected: normalising repo-relative paths
# would need shell parsing plus path/symlink semantics while adding no real
# protection over the already allowed `bash`.

## Allowed Commands
allow:: cargo, make, bun, bunx, npm, yarn, pnpm, rch, sccache
allow:: echo, mkdir, git, ls, cat, cd, cp, mv, rm, chmod
allow:: sh, bash, test, export, source, true, set, rustup

## Denied Commands (security -- overrides allow)
deny:: docker, curl, wget, nc, ncat, python, python3, perl, ruby

## RCH Routing (cargo compilation subcommands offloaded to rch farm)
route_to:: rch, cargo, build check clippy doc
