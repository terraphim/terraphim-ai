# Private LSP producer recipe

This adds only a build recipe and its tests after the merged editor/LSP correction in PR #3484. Gitea remains the source and merge authority. A separately reviewed public GitHub control workflow obtains this exact source through a dedicated read-only binding, verifies the build-recipe SHA-256, and calls the recipe. No private Git history or source archive is published.

The recipe requires an existing source tag on reviewed main, checks the pinned toolchain and crate version through the existing guard, and retains raw logs in a private directory. Qualification runs recipe unit tests, formatting, Clippy with warnings denied, and full core/LSP tests. Successful lanes produce only their expected raw binaries and typed receipts. Native executable version checks and cross-target architecture checks remain distinct. macOS artifacts are explicitly unsigned; no broad signing credential is consumed.

The public control assembles exactly six binaries and checksums, pins the immutable workflow artifact in a reviewed handoff, and publishes only from a separate public tag. This private script creates no tags or releases. The actual build remains pending the dedicated source/registry environment bindings and exact reviewed source request.

Validation on 14 October 2026: five real subprocess/filesystem recipe tests pass. These tests are guard evidence, not six-target compilation. The prior stock-Zed candidate outputs retain their original source/binary identities; this recipe change does not relabel them as a release.
