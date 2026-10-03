# Real World Case

Real World Case (RWC) is the challenge format where a learner works a realistic scenario in a dedicated conversational thread, submitting evidence for scoring. The web app routes RWC to its own thread page (`/real-world-case/{skill_id}`) backed by mentor-service endpoints: `start_or_resume_thread`, `post_message`, `upload_attachment`, `submit_thread`. Scoring is multi-path — SALE diagnostic scoring, deterministic fallback, and LLM evaluator/judge/reconciled scoring — and all persisted paths are surfaced through the teacher-facing scoring report (`GET /api/v1/scoring-report`), which exposes readable answer text, confidence, model id, prompt hash, latency, score details, and evidence gaps. Readiness archetype scoring and diagnostic scoring calibration feed the same pipeline.

synonyms:: real world case, RWC, rwc thread, scoring report, teacher scoring, evidence gap, diagnostic scoring, archetype scoring, evaluator judge

## Related Concepts
- Challenge-based Evidence
- Confidence Gate
- Readiness Signalling
- Digital Learning Twin

## Sources
- `.agent/handoffs/2026-07-10-real-world-case-routing.md` (ZES-440, PR #864)
- `.agent/handoffs/2026-07-15-teacher-scoring-report.md` (PR #917)
- `.agent/handoffs/2026-07-13-pr886-real-world-case-progress-review.md`
- `.agent/handoffs/2026-07-02-oditech-689-readiness-archetype-scoring.md`
- `.agent/handoffs/2026-07-02-oditech-690-diagnostic-scoring-calibration.md`
