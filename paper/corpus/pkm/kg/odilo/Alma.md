# Alma

Alma is the AI mentor persona of the Digital Learning Twin, delivered through Slack and the web app. Alma runs grounded conversations over assigned catalogue content (Ask Alma), issues briefings and acknowledgements, persists assessment greetings, and drives the learner through diagnostics, learning plans, and challenges. Learning plan approval is configurable per pilot: human approval defaults to true and manager auto-approve defaults to false (`ALMA_PILOT_MANAGER_AUTOAPPROVE`), with an operational runbook covering the auto-approve path. Grounded assigned-book responses carry source attribution chips; sources are omitted for refusals, challenge transitions, personal onboarding, and resume flows.

synonyms:: Alma, ask alma, AI mentor, mentor persona, alma conversation, alma briefing, alma acknowledgement, learning plan auto-approve, alma pilot

## Related Concepts
- Digital Learning Twin
- Content Bridge
- Real World Case
- Readiness Signalling

## Sources
- `.agent/handoffs/2026-07-10-oditech-625-alma-acknowledgements.md` (PR #853)
- `.agent/handoffs/2026-07-10-oditech-728-pilot-autoapprove-ops-runbook.md` (PR #849, runbook `docs/runbooks/alma-pilot-learning-plan-autoapprove.md`)
- `.agent/handoffs/2026-07-10-oditech-729-review-merge-pr777.md` (approval configurability, PR #777)
- `.agent/handoffs/2026-07-15-oditech-810-backend-source-attribution.md` (PR #910)
