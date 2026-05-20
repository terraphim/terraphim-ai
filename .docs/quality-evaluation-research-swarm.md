# Document Quality Evaluation Report

## Metadata
- **Document**: `.docs/research-implementation-swarm.md`
- **Type**: Phase 1 Research
- **Evaluated**: 2026-05-20

## Decision: GO

**Average Score**: 4.0 / 5.0
**Blocking Dimensions**: None

## Dimension Scores

| Dimension | Score | Weight | Weighted | Status |
|-----------|-------|--------|----------|--------|
| Syntactic | 4/5 | 1.0 | 4.0 | Pass |
| Semantic | 4/5 | 1.5 | 6.0 | Pass |
| Pragmatic | 4/5 | 1.2 | 4.8 | Pass |
| Social | 4/5 | 1.0 | 4.0 | Pass |
| Physical | 4/5 | 1.0 | 4.0 | Pass |
| Empirical | 4/5 | 1.0 | 4.0 | Pass |
| **Total** | | **6.7** | **26.8** | |
| **Weighted Average** | | | **4.0** | Pass |

## Detailed Findings

### Syntactic Quality (4/5)

**Strengths:**
- All 7 required sections present and well-structured (Section 1-7)
- Terminology is consistent: `supports_stdin`, `oom_score_adj`, `provider_probe` used uniformly
- Table formats are consistent across sections 3, 4, and 5

**Weaknesses:**
- "Provider" and "model" are used somewhat interchangeably in the Constraints table (Section 4) -- e.g., "Claude CLI rate limits" is a provider-level constraint but the implication talks about "KG router falls back to opencode" (a different provider, not a model)

**Suggested Revisions:**
- [ ] Clarify in Section 4 whether "Claude CLI rate limits" is a provider-level or model-level constraint

### Semantic Quality (4/5)

**Strengths:**
- Domain-accurate: correctly identifies `terraphim_spawner`, `terraphim_orchestrator`, worktree manager, and their roles
- Facts verified against actual system logs (swarm-B run at 19:10 UTC, wall_time_secs=1168, branch `gitea/task/1719-redact-env-vars-debug` created)
- IN/OUT scope boundaries are explicit and appropriate
- The description of the stdin issue matches the observed behaviour (opencode hangs on stdin for >50KB)

**Weaknesses:**
- Missing explicit mention that the `supports_stdin` fix was deployed to bigbox at 19:01 CEST and that the 19:10 UTC swarm run was the first test of the fix
- The document states "Claude CLI probes are currently failing" but doesn't note the time-critical nature (rate limit resets at 2am CEST)

**Suggested Revisions:**
- [ ] Add a "Recent Events" subsection under Section 1 noting the 19:10 UTC successful run and the `supports_stdin` deployment
- [ ] Note the 2am CEST reset time for Claude rate limits

### Pragmatic Quality (4/5)

**Strengths:**
- Highly actionable: specific branches (`gitea/task/1719-redact-env-vars-debug`), specific times, specific exit codes
- Questions for reviewer (Section 7) are all specific and decision-oriented
- Simplification opportunities (Section 6) are concrete and implementable
- Risks have clear severity ratings and mitigations

**Weaknesses:**
- The document could more explicitly answer "What should we do next?" -- the research is thorough but the transition to Phase 2 design is not explicitly signposted
- Question 4 ("Why was no PR created?") is a good question but the document doesn't offer a hypothesis

**Suggested Revisions:**
- [ ] Add a "Hypothesis for Missing PR" bullet under Unknowns: the agent task script may be failing at `gtr create-pull` because the worktree's `origin` remote points to gitea, not GitHub, causing the PR creation to fail silently
- [ ] Add an explicit "Recommended Next Steps" list at the end

### Social Quality (4/5)

**Strengths:**
- Assumptions are explicitly marked ("ASSUMPTION:")
- Risks are categorized by severity
- The document distinguishes between observed facts and inferred causes
- Jargon is minimal or explained

**Weaknesses:**
- "opencode" and "claude" are used without first explaining what they are (CLI tools for AI agents) -- though this may be acceptable for the target audience

**Suggested Revisions:**
- [ ] Add a one-line parenthetical on first use: "opencode (the Moonshot/Minimax/ZAI CLI tool)"

### Physical Quality (4/5)

**Strengths:**
- Clear section headings with numbered sections
- Tables used effectively in Sections 3, 4, and 5
- Markdown formatting is clean and navigable
- Document length is appropriate (~100 lines)

**Weaknesses:**
- No diagram or visual representation of the agent lifecycle (spawn -> worktree -> branch -> PR)
- The system elements table in Section 3 is wide and may wrap poorly on narrow screens

**Suggested Revisions:**
- [ ] Add a simple mermaid or ASCII diagram showing the agent lifecycle

### Empirical Quality (4/5)

**Strengths:**
- Good information chunking: tables break up dense information
- Sentence structure is clear and concise
- Complex technical details are isolated in tables

**Weaknesses:**
- Section 5 (Risks/Unknowns/Assumptions) is information-dense and could benefit from sub-section headers
- The branch creation event (`gitea/task/1719-redact-env-vars-debug`) is buried in an unknown rather than highlighted as a key finding

**Suggested Revisions:**
- [ ] Promote the branch creation finding to Section 1 or 2 as evidence that the swarms ARE partially working

## Revision Checklist

Priority order based on impact:

- [ ] **Medium**: Add "Recent Events" note about 19:10 UTC successful run and `supports_stdin` deployment
- [ ] **Medium**: Promote branch creation finding from Unknown to key finding in Section 2
- [ ] **Medium**: Add hypothesis for missing PR (worktree remote misconfiguration)
- [ ] **Low**: Add explicit "Recommended Next Steps" list
- [ ] **Low**: Add one-line parentheticals for "opencode" and "claude" on first use

## Next Steps

[GO]: Document approved for Phase 2. Proceed with `disciplined-design` to create an implementation plan for:
1. Verifying the `supports_stdin` fix works end-to-end across all opencode agents
2. Fixing the missing PR creation (investigating `gtr create-pull` failure in worktrees)
3. Adding provider probe backoff for rate-limited providers
4. Adding observability hooks to track branch/PR creation

Address medium-priority revisions before or during Phase 2.
