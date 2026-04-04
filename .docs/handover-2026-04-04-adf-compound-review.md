# ADF Handover: Compound Review Auto-File Fix

**Date:** 2026-04-04  
**Time:** ~17:00 CEST  
**Author:** AI Assistant  
**Status:** ✅ Fix Deployed & Verified

---

## Executive Summary

Successfully fixed the Gitea API 422 error that was blocking auto-filed issues from compound review findings. The fix has been **deployed to bigbox and verified working** - 5 CRITICAL issues were automatically created as separate Gitea issues.

**Key Issue:** Gitea fork expects `labels` as int64 IDs, not strings. Removed labels field from `create_issue` payload.

---

## What Was Fixed

### Problem
```
API error: Gitea create_issue error 422 Unprocessable Entity: 
{"message":"[]: json: cannot unmarshal number \" into Go struct field 
CreateIssueOption.Labels of type int64"}
```

### Root Cause
Their Gitea fork at `git.terraphim.cloud` has a modified API that expects `labels` as **int64 IDs**, not string names. Standard Gitea API accepts string label names.

### Solution
**File:** `crates/terraphim_tracker/src/gitea.rs`

Removed the `labels` field from the JSON payload in `create_issue()`:

```rust
// BEFORE (broken):
.json(&serde_json::json!({
    "title": title,
    "body": body,
    "labels": labels,  // <-- Removed this line
}))

// AFTER (working):
.json(&serde_json::json!({
    "title": title,
    "body": body,
}))
```

### Verification
- **Deployed:** 16:36 CEST to bigbox
- **Tested:** 16:40 CEST compound review
- **Result:** ✅ 5 CRITICAL issues auto-filed as issues #266-270
- **No 422 errors** since fix deployed

---

## Command Collision Bug Discovered

### Issue
When someone writes `@adf:compound-review`, the terraphim-automata parser matches **TWO** patterns:

1. **Special Command:** `AdfCommand::CompoundReview` → triggers full 6-agent swarm
2. **Agent Spawn:** `AdfCommand::SpawnAgent` → spawns single agent named "compound-review"

This happens because there's an agent named `compound-review` in the config that matches `@adf:compound-review` pattern.

### Evidence
```
14:40:38 - "compound review triggered via @adf:compound-review mention"
14:40:38 - "starting compound review swarm" (correlation_id: bf145dd0)
```

But also:
```
14:40:38 - "dispatching mention-driven agent via terraphim-automata parser agent=compound-review"
```

### Recommendation
**Options to fix:**
1. **Rename the agent** from `compound-review` to `quality-coordinator` or similar
2. **Add priority logic** in parser to skip SpawnAgent if CompoundReview matched
3. **Remove standalone agent** - compound review should ONLY run via swarm

**Suggested fix (Option 1 - safest):**
```toml
# In orchestrator.toml
# Change:
name = "compound-review"
# To:
name = "quality-coordinator"
```

---

## System Status (as of 17:00 CEST)

### Processes
| PID | Started | Type | Status |
|-----|---------|------|--------|
| 1164270 | 16:27 | ADF | ⚠️ Duplicate |
| 1184418 | 16:36 | ADF | ✅ Primary |
| 12922xx | 16:40 | Agents | 🔄 Active (compound review) |

### Current Activity
- **Compound review running** (started 16:40)
- 2 agents active: rust-reviewer, security-sentinel
- ~19 minutes runtime so far

### Issues to Address
1. **Multiple ADF instances** - 3 processes running, may cause contention
2. **Command collision** - `@adf:compound-review` triggers both swarm + single agent
3. **Missing remediation agents** - configured but not spawning (check `auto_remediate = true`)

---

## Configuration Reference

### Compound Review Config
```toml
[compound_review]
schedule = "0 * * * *"  # Hourly
gitea_issue = 108
auto_file_issues = true   # ✅ Working
auto_remediate = true     # ⚠️ Needs verification
base_branch = "main"
```

### Key File Locations
- **Config:** `/opt/ai-dark-factory/orchestrator.toml`
- **Logs:** `/opt/ai-dark-factory/logs/adf.log`
- **Binary:** `/usr/local/bin/adf`
- **Source:** `~/terraphim-ai/crates/terraphim_tracker/src/gitea.rs`

---

## Lessons Learned

### 1. API Compatibility
**Lesson:** Always test against the actual Gitea instance, not just standard API docs. Forks may have modifications.

**Impact:** Lost time debugging 422 errors that were due to API differences, not code bugs.

### 2. Process Hygiene
**Lesson:** Multiple ADF instances can accumulate after restarts. Need to verify single instance.

**Check:** `ps aux | grep "adf orchestrator" | grep -v grep`

### 3. Command Parser Collisions
**Lesson:** terraphim-automata matches ALL patterns. Special commands must exclude agent names.

**Fix:** Reserve `@adf:compound-review` exclusively for the swarm command.

### 4. Verification Strategy
**Lesson:** End-to-end testing requires patience (compound review takes 3-5 minutes).

**Approach:** Use hourly schedule OR trigger manually and wait for completion logs.

### 5. Label Format Discovery
```bash
# Working format (no labels):
curl -X POST -d '{"title":"...","body":"..."}'

# Broken format:
curl -X POST -d '{"title":"...","body":"...","labels":["bug"]}'
```

---

## Next Steps

### Immediate (Next Hour)
1. ✅ Monitor 17:00 CEST compound review for completion
2. ✅ Verify no 422 errors in logs
3. ⏳ Kill duplicate ADF processes (keep PID 1184418)

### Short Term (Today)
1. Fix command collision - rename `compound-review` agent
2. Verify remediation agents spawn for CRITICAL findings
3. Clean up old agent processes if any are stuck

### Medium Term (This Week)
1. Wire SharedLearningStore into orchestrator (Issue #242)
2. Add labels support back (if Gitea API fixed or label IDs fetched)
3. Implement deduplication for auto-filed issues

---

## Test Commands

### Check Auto-File Works
```bash
ssh bigbox 'grep "filed finding issue" /opt/ai-dark-factory/logs/adf.log | tail -3'
```

### Check for 422 Errors
```bash
ssh bigbox 'grep "422" /opt/ai-dark-factory/logs/adf.log | tail -3'
```

### Trigger Manual Compound Review
```bash
# Post to Gitea issue #108
curl -X POST \
  -H "Authorization: token $GITEA_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"body": "@adf:compound-review"}' \
  "https://git.terraphim.cloud/api/v1/repos/terraphim/terraphim-ai/issues/108/comments"
```

---

## Related Issues

- **#186:** Cursor-based mention polling (✅ Fixed)
- **#242:** SharedLearningStore integration (⏳ In Progress)
- **Auto-file:** Compound review findings loop (✅ Working)

---

## Contact

For questions about this handover, check:
1. Gitea issue #108 for compound review summaries
2. Gitea issues #266-270 for auto-filed CRITICAL findings
3. `/opt/ai-dark-factory/logs/adf.log` on bigbox
