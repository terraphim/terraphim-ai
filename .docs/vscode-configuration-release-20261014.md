# VS Code configuration fixes and Continue 0.1.1

Refs terraphim/terraphim-ai#3488 (Gitea).

The released VS Code 0.1.2 package ignored the configured Atomic agent on first autocomplete activation, retained a Store after endpoint changes, and did not consult `enableLocalServer` when sending requests. Its README also attributed a two-character trigger to the Atomic provider, which only responds after a space.

## Changes

- `src/helpers/getStore.ts` reads current settings and keys the actual `@tomic/lib` Store by endpoint and agent. Empty credentials use anonymous access; invalid credentials do not replace a valid cache entry or claim successful activation.
- `src/extension.ts` resolves the current Store for commands and completion requests. A response from an old Store is discarded. Configuration failures produce actionable notifications.
- `src/helpers/serverConnection.ts` uses the configured HTTP(S) URL for every request. With local mode disabled it refuses literal localhost/loopback URLs; it never invents a public fallback. This is URL selection, not a DNS/network security boundary. Requests have a five-second timeout and endpoint/role changes advance a revision.
- `src/helpers/completionCache.ts` tracks both request generation and configuration revision. An old response cannot populate a newer query's cache, and an old query cannot fall back to a newly configured endpoint.
- Packaged README/settings/changelog describe this behavior and the actual Atomic space trigger. Version 0.1.3 includes the previously merged runnable examples for the separately installed `terraphim-grep` and `terraphim-agent` CLIs.
- Continue 0.1.1 packages those CLI examples and the tested graph-restart instructions. Its actual MCP initialization version is checked against the integration manifest. This release does not add a Continue Hub listing or change model configuration.

## Verification

Before bumping versions: TypeScript compile and ESLint pass; 63 package/integrity tests, 29 existing actual-WASM/protocol/export/link tests, four request-cache state regressions, and six real Continue SDK/install/package/source tests pass. Continue transport uses released `terraphim-grep` 1.21.20 locally; CI retains its pinned Linux 1.21.19 binary.

`tests/nativeConfiguration.cjs` runs through the existing packaged native-host suite. It updates only that suite's disposable profile, exercises the actual VS Code API and packaged Atomic Store, restores settings, and saves a separate `configuration.json`. It proves configured credentials on first activation, Store replacement after endpoint/credential changes, anonymous reset, malformed-agent refusal, canonical loopback refusal, explicit remote URL selection, real unavailable-server handling, and role revision changes. It does not claim authenticated remote service access.

Local VS Code 1.141.2 passed those configuration checks and actual saved-file/KG outputs. One earlier candidate also passed native history/reopen; the later candidate refused the history assertion because its window was unfocused. That failed attempt is retained and does not qualify the final package. The immutable release workflow must pass the complete lifecycle on the exact packaged bytes on Linux, macOS and Windows before publication. No all-41 or visual-UI qualification is added by these tests.

The first PR matrix passed Linux/macOS but failed Windows' local-mode assertion. Its log initialized two Stores at the same endpoint: dynamically requiring a Windows URI-derived path can instantiate another copy of a singleton. The native test now selects exactly one already loaded production module by actual file identity, records requested/loaded paths, and retains all refusal assertions. A fresh complete matrix must confirm the correction; the test never refreshes the singleton itself or substitutes its API.

No release asset replacement, tag reuse, Open VSX readiness change or branch-protection change is part of this amendment.
