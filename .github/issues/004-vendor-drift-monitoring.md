---
title: "LOW: Implement vendor API drift monitoring and alerting"
labels: ["priority/P2", "type/enhancement", "component/observability", "echo/monitoring"]
assignees: []
milestone: ""
---

## Summary

**Echo recommends** implementing automated vendor API drift detection to prevent future synchronization failures.

## Problem

Current drift detection is manual and reactive:
- No automated changelog scanning
- No version drift alerts
- Breaking changes discovered late
- Coordinated upgrades difficult

## Solution

Implement automated monitoring for critical vendor APIs.

## Proposed Implementation

### 1. Weekly Changelog Scanner

```bash
#!/bin/bash
# .github/scripts/check-vendor-drift.sh

VENDORS=(
    "jeremychone/rust-genai:CHANGELOG.md"
    "modelcontextprotocol/rust-sdk:CHANGELOG.md"
    "firecracker-microvm/firecracker:CHANGELOG.md"
)

for vendor in "${VENDORS[@]}"; do
    repo="${vendor%%:*}"
    file="${vendor##*:}"
    
    # Fetch latest changelog
    curl -s "https://raw.githubusercontent.com/$repo/main/$file" | \
        grep -E "^## v[0-9]" | head -5
    
    # Compare with current version
    # Alert if major/minor version differs
done
```

### 2. Version Tracking File

Create `.vendor-versions.toml`:

```toml
[vendors.genai]
name = "rust-genai"
repo = "https://github.com/jeremychone/rust-genai"
current = "0.4.4"
target = "0.6.0"
last_checked = "2026-03-23"
priority = "critical"

[vendors.rmcp]
name = "rmcp"
repo = "https://github.com/modelcontextprotocol/rust-sdk"
current = "0.9.1"
target = "1.2.0"
last_checked = "2026-03-23"
priority = "critical"

[vendors.firecracker]
name = "firecracker"
repo = "https://github.com/firecracker-microvm/firecracker"
current = "1.10.0"
target = "1.11.0"
last_checked = "2026-03-23"
priority = "moderate"
```

### 3. CI/CD Integration

```yaml
# .github/workflows/vendor-drift-check.yml
name: Vendor Drift Check
on:
  schedule:
    - cron: '0 0 * * 1'  # Weekly on Monday
  workflow_dispatch:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Check for drift
        run: |
          ./.github/scripts/check-vendor-drift.sh > drift-report.md
          
      - name: Create issue if drift detected
        if: contains(steps.check.outputs.report, 'DRIFT DETECTED')
        uses: actions/github-script@v7
        with:
          script: |
            // Create GitHub/Gitea issue
```

### 4. Dashboard

Create simple drift dashboard:

```markdown
# Vendor Drift Dashboard

| Vendor | Current | Latest | Drift | Status |
|--------|---------|--------|-------|--------|
| rust-genai | 0.4.4 | 0.6.0 | 2 minor | 🔴 |
| rmcp | 0.9.1 | 1.2.0 | 3 major | 🔴 |
| Firecracker | 1.10.0 | 1.11.0 | 1 minor | 🟡 |

Last updated: 2026-03-23
```

### 5. Alerting Rules

```yaml
alerts:
  - name: critical-vendor-drift
    condition: drift >= 2 minor versions OR >= 1 major version
    severity: critical
    action: create_issue
    
  - name: moderate-vendor-drift
    condition: drift >= 1 minor version
    severity: warning
    action: notify_slack
    
  - name: security-advisory
    condition: security advisory published
    severity: critical
    action: create_issue + notify
```

## Implementation Tasks

- [ ] Create `.vendor-versions.toml` tracking file
- [ ] Implement `check-vendor-drift.sh` script
- [ ] Add GitHub Actions workflow
- [ ] Create drift dashboard
- [ ] Setup alerting (Slack/email)
- [ ] Document process

## Benefits

1. **Early Detection:** Catch drift before it becomes critical
2. **Planning:** Time to plan coordinated upgrades
3. **Security:** Rapid response to security advisories
4. **Documentation:** Clear upgrade path

## References

- Current drift report: `docs/vendor-api-drift-report.md`
- Epic tracking: #0

## Definition of Done

- [ ] Automated weekly checks running
- [ ] Drift dashboard accessible
- [ ] Alerts configured for critical drift
- [ ] Documentation complete
- [ ] First automated issue created

---

**Echo's Recommendation:** Proactive monitoring prevents reactive scrambling. Implement before next sprint.
