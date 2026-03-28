# Judge Scoring Configuration Optimization Report

**Date:** 2026-03-27  
**Analyst:** Carthos (Domain Architect)  
**System:** Terraphim Judge Quality Gate

## Executive Summary

Scored configuration optimization analysis against 11 historical verdicts (2026-02-20 to 2026-02-22). Generated 100 configuration variations using grid search and random perturbations. **Current baseline configuration is already optimal** with 100% agreement rate and zero false accepts/rejects.

**Key Finding:** No configuration variation achieved improvement >5% over baseline. **No PR recommended.**

## Methodology

### Current Baseline Configuration

```toml
accept_min_dimension = 3      # All dimensions must be >= 3 for accept
accept_min_average = 3.5      # Average must be >= 3.5 for accept
improve_min_dimension = 2     # All dimensions must be >= 2 for improve
reject_max_dimension = 2      # Any dimension < 2 triggers reject
```

### Verdict Decision Logic

```
if any(dimension < reject_max_dimension):
    return "reject"
elif any(dimension < accept_min_dimension) or average < accept_min_average:
    return "improve"
else:
    return "accept"
```

### Dataset Analysis

| Metric | Value |
|--------|-------|
| Total Verdicts | 11 |
| Time Range | Feb 20-22, 2026 |
| Accept | 6 (54.5%) |
| Improve | 4 (36.4%) |
| Reject | 1 (9.1%) |

**Score Distribution:**
| Dimension | Average | Min | Max |
|-----------|---------|-----|-----|
| Semantic | 3.64 | 1 | 5 |
| Pragmatic | 3.55 | 1 | 5 |
| Syntactic | 3.73 | 2 | 5 |

### Variation Generation Strategy

Two-phase approach:

1. **Systematic Grid Search** (50 variations)
   - Accept dimension thresholds: [2, 3, 4]
   - Accept average thresholds: [3.0, 3.25, 3.5, 3.75, 4.0]
   - Improve thresholds: [1, 2]
   - Reject thresholds: [1, 2]

2. **Random Perturbations** (50 variations)
   - Baseline +/- random offsets
   - Integers: +/- {0, 1}
   - Floats: +/- {0, 0.25, 0.5}

## Results

### Top 3 Candidate Configurations

#### Candidate #1: Baseline (Optimal)

```
Thresholds:
  accept_min_dimension: 3
  accept_min_average: 3.5
  improve_min_dimension: 2
  reject_max_dimension: 2

Metrics:
  Quality Score: 99.98
  Agreement Rate: 100.00%
  False Accepts: 0
  False Rejects: 0
  
Improvement vs Baseline: +0.00 (not significant)
```

#### Candidate #2: grid_3_3.0_2_1

```
Thresholds:
  accept_min_dimension: 3
  accept_min_average: 3.0    # Lowered from 3.5
  improve_min_dimension: 2
  reject_max_dimension: 1     # Raised from 2

Metrics:
  Quality Score: 90.89
  Agreement Rate: 90.91%
  False Accepts: 0
  False Rejects: 0
  Reject Accuracy: 0% (failed to catch the 1 reject case)

Improvement vs Baseline: -9.09 (worse)
```

#### Candidate #3: grid_3_3.25_2_1

```
Thresholds:
  accept_min_dimension: 3
  accept_min_average: 3.25   # Lowered from 3.5
  improve_min_dimension: 2
  reject_max_dimension: 1     # Raised from 2

Metrics:
  Quality Score: 90.89
  Agreement Rate: 90.91%
  False Accepts: 0
  False Rejects: 0
  Reject Accuracy: 0% (failed to catch the 1 reject case)

Improvement vs Baseline: -9.09 (worse)
```

## Analysis

### Why Baseline is Optimal

1. **Perfect Agreement Rate**: All 11 historical verdicts align with baseline thresholds
2. **Zero False Classifications**: No false accepts (critical for quality gates) and no false rejects
3. **Balanced Thresholds**: 
   - Accept threshold (3.5 avg) catches borderline cases
   - Reject threshold (2) correctly identifies low-quality outputs
   - Improve threshold provides clear remediation path

### Why Alternatives Fail

**Reject threshold of 1** (candidates #2 and #3):
- Would classify the single historical reject (scores: semantic=1, pragmatic=1, syntactic=2) as "improve"
- This is a severe quality failure that should be caught
- Reject accuracy drops to0%

**Lowered accept average** (3.0 or 3.25):
- No improvement since all accepts already have avg >= 3.5
- Creates risk of accepting lower-quality outputs without benefit

### Edge Cases Analyzed

The dataset contains one clear reject case with very low scores:
```json
{
  "verdict": "reject",
  "scores": {"semantic": 1, "pragmatic": 1, "syntactic": 2},
  "average": 1.33
}
```

This is correctly caught by the baseline reject threshold (2) but would be missed by a threshold of 1.

## Recommendations

### Short-term (Current Sprint)

1. **No configuration change recommended** - baseline is optimal
2. **Monitor verdict distribution** - track accept/improve/reject ratios over time
3. **Collect more data** - 11 verdicts is a small sample for statistical significance

### Medium-term (Next Sprint)

1. **Implement continuous monitoring**:
   - Alert if accept rate > 80% (potential gaming)
   - Alert if reject rate > 20% (potential model drift)
   - Track dimension score distributions

2. **Add quality metrics**:
   - Track time-in-review for "improve" verdicts
   - Measure iteration count before acceptance
   - Correlate verdict with downstream review outcomes

### Long-term (Monthly)

1. **Re-run optimization** when:
   - >= 100 new verdicts accumulated
   - Accept rate shifts significantly
   - New model versions deployed

2. **Consider dynamic thresholds**:
   - Adjust based on task complexity
   - Different thresholds for different dimensions
   - Context-aware quality gates

## Conclusion

The current judge scoring configuration demonstrates excellent calibration against historical verdicts. The thresholds balance strict quality gates with reasonable acceptance criteria. No configuration changes are warranted at this time.

**Improvement delta: +0.00% (not significant)**
**PR creation: NOT recommended**

---

## Appendix: Configuration Files

### Search Space Definition

```python
accept_dims = [2, 3, 4]           # Minimum dimension score for accept
accept_avgs = [3.0, 3.25, 3.5, 3.75, 4.0]  # Minimum average for accept
improve_mins = [1, 2]              # Minimum dimension score for improve
reject_thresholds = [1, 2]          # Maximum dimension score for reject trigger
```

### Quality Score Formula

```python
quality_score = (
    agreement_rate * 100 -
    false_accepts * 50 -    # Heavy penalty for false accepts
    false_rejects * 20 -    # Moderate penalty for false rejects
    avg_score_diff * 10     # Small penalty for average discrepancies
)
```

### Baseline Performance

```
Total Verdicts: 11
Agreements: 11/11 (100%)
False Accepts: 0
False Rejects: 0
Quality Score: 99.98/100
```

---

**Generated by:** Carthos Domain Architect  
**Timestamp:** 2026-03-27T01:33:50Z  
**Artifacts:** `/home/alex/terraphim-ai/automation/judge-optimizer/results.json`