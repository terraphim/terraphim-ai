# Structured PR Review Template

## Review Output Format

Write review findings to the issue's review-findings.md with this structure:

```
---
stage: review-findings
issue: {{issue}}
timestamp: [UTC timestamp]
review_complete: true
confidence_score: [1-5]
total_findings: [number]
p0_count: [number]
p1_count: [number]
p2_count: [number]
---

## Review Summary
[Overall assessment: 2-3 sentences]

## Architecture Diagram
```mermaid
[Simple diagram of changed components]
```

## P0 Critical (Must Fix Before Merge)
| # | File:Line | Description | Suggested Fix |
|---|-----------|-------------|---------------|
| [or "None"] |

## P1 Important (Should Fix Before Merge)
| # | File:Line | Description | Suggested Fix |
|---|-----------|-------------|---------------|
| [or "None"] |

## P2 Minor (Consider Fixing)
| # | File:Line | Description | Suggested Fix |
|---|-----------|-------------|---------------|
| [or "None"] |

## Positive Observations
- [What was done well]

## Recommendations
1. [Recommendation]
```

## Rules
- Every finding MUST reference a specific file and line number
- Confidence score MUST be 1-5
- If code is clean, state "No findings" in the relevant severity section
- Do NOT produce generic/vague findings - every finding must be actionable
