# Build-Runner-LLM Proof

**Issue**: 1423
**Flow**: build-runner-llm-proof
**Generated**: 2026-05-30 00:12 BST

## What Was Proved

1. **KG-first detection**: `scripts/build-runner-llm.sh` detects build commands
   from BUILD.md > GitHub Actions > Cargo.toml fallback.

2. **Command transformation**: terraphim-agent replace transforms tool commands
   (npm → bun, pip → uv) via the Terraphim Engineer KG.

3. **Whitelist validation**: Commands are validated against an allowlist before
   execution. Blocked: sudo, curl|sh, rm -rf /.

4. **LLM fallback on failure**: When a build step fails, claude --model haiku
   diagnoses the failure and suggests a correction.

5. **Self-healing BUILD.md**: The LLM correction is appended to BUILD.md
   under an "Auto-corrected" timestamped section.

## Script Location

`scripts/build-runner-llm.sh` — 150-line bash script.

## Architecture

```
Git Push → build-runner-llm.sh
  ├─ detect_commands()     → BUILD.md / ci-pr.yml / Cargo.toml
  ├─ transform_command()   → terraphim-agent replace (KG)
  ├─ is_allowed()          → whitelist check
  ├─ eval                  → execute
  └─ on failure:
       ├─ llm_correct_command() → claude --model haiku
       └─ update_build_md()     → append to BUILD.md
```

## Known Gaps

| Gap | Status |
|-----|--------|
| DevOpsRunner role not configured | KG transforms use general Terraphim Engineer role |
| yq not installed locally | Falls back to hardcoded cargo commands |
| BUILD.md has no ```bash build block | Detection falls through to fallback |
| No cost tracking | bash.00 currently (no LLM cost logging) |
| No commit status posting in local mode | GITEA_TOKEN not set for local runs |

## Next Steps for Production

1. Create DevOpsRunner role in `~/.config/terraphim/config.json`
2. Populate `~/.config/terraphim/docs/src/kg/devops/` with build ontology
3. Create dedicated bash block in BUILD.md: ```bash\ncargo fmt ...\n```
4. Install yq for GitHub Actions parsing
5. Wire cost tracking metrics
6. Deploy to bigbox and restart orchestrator
