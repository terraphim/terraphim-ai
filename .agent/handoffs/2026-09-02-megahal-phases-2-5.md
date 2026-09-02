# Handover: MegaHAL port phases 2-5 (issues #3261-#3264, initiative #3259)

- Completed: 2026-09-02 ~01:15 UTC
- Branch: task/3263-megahal-wasm-demo (stacks on task/3262-* <- task/3261-*)
- PRs: #3318 (phase 2) <- #3319 (phase 3) <- #3321 (phases 4+5), all base-stacked, unmerged

## Known-good state
- terraphim_sooth 1.21.3 + terraphim_megahal 1.21.3 PUBLISHED to internal terraphim registry;
  consumer test from a fresh project reproduces the conformance reply.
- Demo live: https://terraphim-megahal-demo.pages.dev (Cloudflare Pages, zestic.ai,
  project terraphim-megahal-demo, deployed via wrangler from dist/).
- All tests green: megahal 12 lib + 3 conformance + 10 engine + 7 brain_evolution + 6 integration
  (features matrix: none/default/personalities/automata+persistence/all-features);
  sooth 16+3+1; wasm headless 6. clippy -D warnings; fmt; wasm32 check.
- Size: 394.7 KB gzipped (budget 500); latency ~6 ms (budget 50).

## Key artefacts
- crates/terraphim_megahal (engine, conformance fixtures + Ruby driver, CLI, changelog)
- crates/terraphim_megahal_wasm (Brain Lab demo, wasm tests, CI ci-megahal-wasm.yml, wrangler.toml)
- crates/terraphim_megahal/docs/demo/ (VHS gif + tape + fixture)
- scripts/publish-via-auth-proxy.sh + cargo_registry_proxy.py (internal-registry publish;
  works around cargo 1.96 bare-Authorization header vs Gitea 1.26 401)
- blog/2026-09-02-megahal-loebner-rust-wasm-knowledge-graph.md (DRAFT, CTO sign-off pending)
- .gitattributes: vendored ruby_vendor/** exempted from whitespace check

## Decisions recorded on #3264 (awaiting CTO)
1. Versions 1.21.3 (fleet convention) not 0.1.0
2. Apache-2.0 over Unlicense upstream, attribution kept
3. crates.io publication deferred (crate-name decision: terraphim_sooth vs sooth-rs)

## Resume steps
1. Merge #3318 -> retarget/merge #3319 -> #3321 (stacked; adf/build + adf/pr-reviewer gates)
2. Blog publish after sign-off (Zola site, blog/ dir)
3. Follow-up #3320: persistence crate must expose settings-based storage init for sqlite/redb round-trips

## Gotchas
- op inject requires --account zesticailtd.1password.com (TerraphimPlatform vault);
  env-file output needs eval. Fallback: git credential fill for git.terraphim.cloud.
- Root *.html gitignore rule silently drops new HTML files (hit index.html).
- cargo publish --registry terraphim 401s on cargo 1.96: use the proxy script.
- str.replace patches must be assert-guarded (the lab Enter bug).
