# Sign-off Checklist for Live-User Skills Validation

This checklist is the definition of "done" for validating every Terraphim Skill with a live user.

## Prerequisites
- [ ] Live-user environment runbook followed (`docs/runbooks/live-user-skills-validation.md`)
- [ ] `bash scripts/skill-installer-prereqs.sh` returns exit code 0
- [ ] All 5 required CLIs present: tsm, claude, codex, pi, pi-rust, grok
- [ ] Token present (length > 0) or `~/.terraphim/token` file present

## Evidence Capture
Run the evidence script:

```bash
TERRAPHIM_SKILLS_CATALOGUE=docs/runbooks/skills-catalogue.json \
TERRAPHIM_SKILLS_MANAGER_BIN=$(which tsm) \
TERRAPHIM_SKILLS_OWNER=terraphim \
TERRAPHIM_SKILLS_REPO=terraphim-ai \
TERRAPHIM_SKILLS_GITEA_ISSUE=2707 \
  scripts/skill-installer-evidence.sh
```

## Artefact Review
- [ ] `01-prereqs.txt` -- no FAIL rows for required items
- [ ] `02-sentinel.json` / `03-sentinel.md` -- sentinel smoke produced matrix
- [ ] `04-full.json` / `05-full.md` -- full catalogue produced matrix
- [ ] `06-environment.txt` -- no token values printed, only presence and length
- [ ] `07-summary.md` -- checklist complete
- [ ] `08-followups.txt` -- follow-up issues for every FAIL/UNSUPPORTED cell

## Cell Classification Rules

| Cell Status | Meaning | Sign-off Action |
|-------------|---------|-----------------|
| PASS | tsm install + verify + CLI-specific probe all green | No action |
| FAIL | install, verify, or probe failed with non-zero exit | Follow-up issue required |
| SKIP | CLI binary not on PATH or other optional dependency missing | Note in evidence; mark expected if not installing that CLI |
| UNSUPPORTED | No `tsm --agent` mapping and no `--install-dir` configured | Follow-up issue required to add support |

## Posting Evidence to Gitea

The evidence script outputs a `gtr comment` command in `07-summary.md`. Run it to post the full Markdown matrix as a comment on the epic issue.

If `gtr` is not available or `GITEA_TOKEN` is not set, manually copy the contents of `05-full.md` to a comment on issue `#2707`.

## Closing the Epic

Issue `#2707` can be closed only when:

- [ ] All required CLIs installed and verified
- [ ] Sentinel smoke run completed
- [ ] Full catalogue run completed
- [ ] Every FAIL cell has a linked follow-up issue
- [ ] Every UNSUPPORTED cell has either a follow-up issue or an approved waiver
- [ ] Evidence package posted as Gitea comment
- [ ] Live user has reviewed and signed off

## Waiver Process

If a cell is intentionally UNSUPPORTED (e.g., Grok does not have a first-class `Agent::Grok` mapping), document the waiver in the epic with:
- Cell (skill, CLI) coordinates
- Reason for waiver
- Owner of the waiver
- Expiry date or review date

## Follow-up Issue Format

Each auto-created follow-up issue contains:
- Skill name
- CLI name
- Status (FAIL or UNSUPPORTED)
- Evidence string from the validation cell
- Reference to the parent epic `#2707`

These follow-up issues are triaged independently of the epic.
