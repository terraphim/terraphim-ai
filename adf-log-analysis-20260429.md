# ADF Log Analysis Report -- 2026-04-29

**Analyst:** Conduit (DevOps Engineer)  
**Index:** adf-logs (Quickwit)  
**Period:** 2026-04-05 to 2026-04-08 (latest available data)  
**Total Events:** 17,859 documents

---

## Executive Summary

Pipeline health is **degraded but stable**. Overall hard failure rate is 2.3% (20/853 spawns), but concentrated in a small subset of agents. All failures are unclassified ("unknown" exit class, 0% confidence), indicating a gap in root cause attribution. Wall time outliers are the most significant throughput concern.

**Note:** Gitea issue #328 at `terraphim/terraphim-ai` was inaccessible (404 Not Found). This report is saved locally instead.

---

## 1. Failure Clusters

### 1.1 Hard Failures (exit_code = 1)

| Agent | Failures | Spawns | Failure Rate | Severity |
|-------|----------|--------|--------------|----------|
| compliance-watchdog | 2 | 17 | **11.8%** | HIGH |
| documentation-generator | 2 | 20 | **10.0%** | HIGH |
| product-owner | 2 | 25 | **8.0%** | MEDIUM |
| log-analyst | 2 | 31 | **6.5%** | MEDIUM |
| merge-coordinator | 4 | 100 | **4.0%** | MEDIUM |
| spec-validator | 1 | 30 | 3.3% | LOW |
| quality-coordinator | 1 | 30 | 3.3% | LOW |
| implementation-swarm | 1 | 39 | 2.6% | LOW |
| test-guardian | 1 | 41 | 2.4% | LOW |
| drift-detector | 3 | 261 | 1.1% | LOW |
| security-sentinel | 1 | 205 | 0.5% | LOW |

**Root cause pattern:** All 20 failures show exit_class "unknown" with 0 confidence and matched_patterns []. The orchestrator cannot classify these exits. Wall times cluster tightly at 27-31 seconds, suggesting a **timeout boundary** rather than a logic error.

### 1.2 Wall Time Outliers (>120s)

| Agent | Outlier Events | Average (s) | Max (s) | Severity |
|-------|----------------|-------------|---------|----------|
| implementation-swarm | 8 | 754.6 | **1,619.2** | CRITICAL |
| merge-coordinator | 11 | 319.1 | 1,136.3 | HIGH |
| security-sentinel | 37 | 195.6 | 480.3 | MEDIUM |
| drift-detector | 21 | 168.7 | 279.3 | MEDIUM |
| test-guardian | 5 | 514.6 | 689.2 | MEDIUM |
| product-owner | 3 | 620.4 | 1,171.4 | MEDIUM |
| spec-validator | 4 | 392.4 | 857.9 | MEDIUM |
| quality-coordinator | 4 | 190.3 | 328.4 | LOW |

**Observation:** 100 total outlier events. implementation-swarm averages 12.6 minutes per outlier run. This is a major pipeline throughput bottleneck.

### 1.3 Stderr Warnings

- 19 WARN-level stderr events total
- Pattern: `permission requested: external_directory (...); auto-rejecting`
- Affected agents: meta-coordinator (9), drift-detector (3), security-sentinel (3), product-owner (2), quality-coordinator (1), test-guardian (1)
- **Assessment:** Sandbox operating correctly. Not a failure mode.

### 1.4 Rate Limiting

- 164 messages containing "rate limit" or "timed out"
- Spread across multiple agents (security-sentinel, merge-coordinator, log-analyst, drift-detector)
- **Assessment:** Expected API throttling. No agent is severely rate-limited.

---

## 2. Trend Comparison

| Metric | Apr 5 | Apr 7 | Apr 8 (partial) | Trend |
|--------|-------|-------|-----------------|-------|
| Exit events | 88 | 201 | (data incomplete) | Increasing |
| Spawn events | ~280/day | ~280/day | ~280/day | Stable |
| Hard failures | 1 | 5 | 10+ | **Worsening** |

**Note:** Hard failures accelerated on Apr 8. The cluster of 5 failures at 2026-04-07T13:43 (test-guardian, spec-validator, compliance-watchdog, security-sentinel, quality-coordinator) within 60 seconds suggests an **infrastructure event** (network blip, API outage, or orchestrator restart) rather than independent agent faults.

---

## 3. Top 3 Remediation Recommendations

### R1: Implement Exit Classification for Timeout Detection
**Confidence:** HIGH  
**Action:** Add pattern matching for exit code 1 + wall_time ~30s to classify as "probable_timeout". Update the orchestrator's exit_class logic.  
**Impact:** Transforms 20 "unknown" failures into actionable data. Enables targeted fixes.

### R2: Cap implementation-swarm and merge-coordinator Wall Time
**Confidence:** HIGH  
**Action:** Introduce a hard wall_time limit of 600s (10 min) for implementation-swarm and 300s (5 min) for merge-coordinator. Kill and retry with degraded scope.  
**Impact:** Prevents 1,619s runs from blocking the pipeline. Reduces average pipeline latency.

### R3: Disable or De-schedule High-Failure Agents
**Confidence:** MEDIUM  
**Action:** Temporarily reduce schedule frequency for:
- compliance-watchdog (11.8% failure rate)
- documentation-generator (10.0% failure rate)
- product-owner (8.0% failure rate)

Investigate whether these agents share a common dependency (model, tool, or filesystem path) that is failing.  
**Impact:** Reduces blast radius while root cause is investigated.

---

## 4. Agents Requiring Schedule Adjustment

| Agent | Current Status | Recommended Action |
|-------|---------------|-------------------|
| compliance-watchdog | 11.8% failure rate | **Reduce frequency 50%** until root cause found |
| documentation-generator | 10.0% failure rate | **Reduce frequency 50%** until root cause found |
| product-owner | 8.0% failure rate | **Reduce frequency 25%** until root cause found |
| implementation-swarm | 754s avg outlier | **Add 600s hard timeout** |
| merge-coordinator | 319s avg outlier | **Add 300s hard timeout** |

---

## 5. Anomalies Detected

1. **Synchronised Failure Cluster:** 5 agents failed within 60 seconds on Apr 7 at 13:43. Correlation suggests external trigger.
2. **Exit Event Count Exceeds Spawns:** 956 exit events vs 853 spawns. Some agents may be exiting multiple times per spawn, or spawn events are being dropped from logs.
3. **quality-coordinator Outlier:** One failure at 181.9s wall time (6x typical failure duration). Suggests hung process rather than clean timeout.

---

## Appendix: Query Log

```
exit_code:>0                      -> 20 hits
level:ERROR                       -> 0 hits
level:WARN                        -> 40 hits
wall_time_secs:>120               -> 100 hits
message:timeout                   -> 2,437 hits (mostly stdout)
source:orchestrator AND message:spawned  -> 853 hits
source:orchestrator AND message:exited   -> 956 hits
```

---

*Report generated by Conduit. Next review recommended in 24 hours or after infrastructure changes.*
