# Task: Monitor PR #426 Production Deployment (24-48 Hours)

**Task ID**: TASK-PR426-MONITOR-001  
**Priority**: HIGH  
**Status**: PENDING  
**Created**: 2026-03-19  
**Due**: 2026-03-21 (48 hours from deployment)  
**Assignee**: Operations Team / On-call Engineer  
**Related**: PR #426, Deployment f63f114d

---

## Objective

Monitor the fcctl-core adapter production deployment on bigbox for 24-48 hours to ensure stable operation and collect performance metrics for potential pool configuration optimization.

---

## Monitoring Checklist

### Hour 0-4 (Immediate Post-Deployment)

- [ ] **Verify deployment marker**
  ```bash
  ssh bigbox "cat /home/alex/terraphim-ai/.deployment-marker"
  ```
  Expected: `Status: PRODUCTION`

- [ ] **Check Firecracker daemon status**
  ```bash
  ssh bigbox "pgrep -a firecracker | head -5"
  ```
  Expected: Firecracker processes running

- [ ] **Verify KVM access**
  ```bash
  ssh bigbox "ls -la /dev/kvm && id | grep kvm"
  ```
  Expected: `/dev/kvm` accessible

- [ ] **Check library installation**
  ```bash
  ssh bigbox "ls -la /usr/local/lib/libterraphim_rlm.rlib"
  ```
  Expected: Library present (5.5MB)

- [ ] **Initial smoke test**
  ```bash
  ssh bigbox "cd /home/alex/terraphim-ai/crates/terraphim_rlm && FIRECRACKER_TESTS=1 cargo test test_session_lifecycle --release -- --nocapture 2>&1 | tail -20"
  ```
  Expected: Test passes

### Hour 4-24 (First Day)

- [ ] **Monitor VM allocation latency**
  - Collect metrics every hour
  - Alert threshold: >500ms
  - Target: <267ms (current benchmark)
  
- [ ] **Track pool utilization**
  - Current config: min=2, max=10 VMs
  - Monitor: Active VMs, warm VMs, idle VMs
  - Alert if pool exhausted (all 10 VMs in use)

- [ ] **Check error rates**
  - Adapter errors
  - Firecracker errors  
  - VM lifecycle failures
  - Alert threshold: >1% error rate

- [ ] **Verify snapshot directory**
  ```bash
  ssh bigbox "du -sh /var/lib/terraphim/snapshots && ls /var/lib/terraphim/snapshots | wc -l"
  ```
  - Monitor disk usage growth
  - Alert threshold: >80% disk usage

### Hour 24-48 (Second Day)

- [ ] **Collect performance metrics**
  - Average allocation latency
  - P50, P95, P99 latency percentiles
  - Pool hit rate (pre-warmed VM usage)
  - VM reuse ratio

- [ ] **Analyze resource usage**
  - CPU utilization during peak
  - Memory usage by pool
  - Network I/O (if applicable)
  - Disk I/O for snapshots

- [ ] **Review logs for anomalies**
  ```bash
  ssh bigbox "journalctl -u terraphim* --since '24 hours ago' | grep -i 'error\|warn\|panic' | head -20"
  ```

- [ ] **Compile monitoring report**
  - Metrics summary
  - Any issues encountered
  - Recommendations for pool config

---

## Metrics to Collect

### Performance Metrics

| Metric | Target | Alert Threshold | Collection Method |
|--------|--------|----------------|-------------------|
| VM Allocation (p50) | <300ms | >400ms | Application logs |
| VM Allocation (p95) | <400ms | >500ms | Application logs |
| VM Allocation (p99) | <500ms | >600ms | Application logs |
| Pool Hit Rate | >80% | <60% | Pool metrics |
| VM Reuse Ratio | >70% | <50% | Pool metrics |
| Error Rate | <0.1% | >1% | Error logs |

### Resource Metrics

| Resource | Current | Alert Threshold |
|----------|---------|-----------------|
| Active VMs | 2-10 | >10 (pool exhausted) |
| Memory per VM | ~380MB | >512MB |
| Disk (snapshots) | ~200MB | >1GB |
| CPU usage | <50% | >80% sustained |

---

## Pool Configuration Adjustment Guidelines

### Current Configuration
```rust
PoolConfig {
    min_vms: 2,
    max_vms: 10,
    warmup_threshold: 0.8,
}
```

### Adjustment Scenarios

#### Scenario A: High Pool Exhaustion
**Symptoms**: Max VMs (10) frequently in use, allocation latency >400ms

**Actions**:
1. Increase max_vms: 10 → 15 or 20
2. Monitor for 24 more hours
3. If still exhausted, increase further or investigate load patterns

**Code change**:
```rust
// In src/executor/firecracker.rs
let pool_config = PoolConfig {
    min_vms: 2,
    max_vms: 15,  // Increased from 10
    warmup_threshold: 0.8,
};
```

#### Scenario B: Low Utilization
**Symptoms**: Average <3 VMs used, pool hit rate <60%

**Actions**:
1. Decrease max_vms: 10 → 6 or 8
2. Save resources while maintaining burst capacity

**Code change**:
```rust
let pool_config = PoolConfig {
    min_vms: 2,
    max_vms: 6,  // Decreased from 10
    warmup_threshold: 0.8,
};
```

#### Scenario C: Slow Allocation Despite Available VMs
**Symptoms**: Latency >400ms even with available VMs in pool

**Actions**:
1. Check Firecracker/KVM performance
2. Increase min_vms: 2 → 4 (more pre-warmed VMs)
3. Review adapter overhead

**Code change**:
```rust
let pool_config = PoolConfig {
    min_vms: 4,  // Increased from 2
    max_vms: 10,
    warmup_threshold: 0.8,
};
```

#### Scenario D: Optimal Performance
**Symptoms**: Latency <300ms, pool hit rate >80%, utilization 40-70%

**Actions**:
- No changes needed
- Current config (2-10) is optimal
- Document as baseline

---

## Data Collection Script

Save to `/tmp/collect_metrics.sh` on bigbox:

```bash
#!/bin/bash
# Collect PR #426 metrics

TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
LOG_FILE="/var/log/terraphim/metrics-$(date +%Y%m%d).log"

# Create log directory if needed
mkdir -p /var/log/terraphim

# Collect metrics
echo "=== $TIMESTAMP ===" >> $LOG_FILE

# VM allocation latency (if exposed via API/metrics)
# TODO: Add actual metric collection based on exposed metrics

# Pool status
echo "Active VMs: $(pgrep -c firecracker)" >> $LOG_FILE

# Resource usage
echo "Memory: $(free -h | grep Mem)" >> $LOG_FILE
echo "Disk: $(df -h /var/lib/terraphim)" >> $LOG_FILE

# Error count (last hour)
ERRORS=$(journalctl --since '1 hour ago' | grep -c terraphim)
echo "Errors (1h): $ERRORS" >> $LOG_FILE

echo "" >> $LOG_FILE
```

---

## Reporting Template

### 24-Hour Report

```markdown
## PR #426 Production Monitoring - 24 Hour Report

**Date**: [Date]  
**Deployment**: f63f114d  
**Status**: [Stable/ Issues Found]

### Metrics Summary
- Average Allocation Latency: [X]ms
- P95 Latency: [X]ms
- Pool Hit Rate: [X]%
- Error Rate: [X]%
- Peak VM Usage: [X]/10

### Issues Found
- [List any issues or "None"]

### Recommendations
- [Pool config adjustments or "No changes needed"]

### Next Steps
- [Continue monitoring / Adjust config / Investigate issues]
```

### 48-Hour Final Report

```markdown
## PR #426 Production Monitoring - 48 Hour Final Report

**Status**: [APPROVED FOR FULL PRODUCTION / NEEDS OPTIMIZATION]

### Performance Summary
- 48h Average Latency: [X]ms
- Peak Latency: [X]ms
- Pool Utilization: [X]%
- Error Rate: [X]%

### Configuration Decision
- Current: min=2, max=10
- Recommended: [Same / Adjusted values]
- Justification: [Explanation]

### Action Items
- [ ] Implement config changes (if any)
- [ ] Set up ongoing monitoring
- [ ] Document lessons learned
```

---

## Rollback Trigger Conditions

**IMMEDIATE ROLLBACK if**:
- [ ] VM allocation consistently >1000ms
- [ ] Error rate >5%
- [ ] Pool exhaustion causing service degradation
- [ ] Firecracker crashes or instability
- [ ] Memory leaks detected

**Rollback Procedure**:
```bash
ssh bigbox
cd /home/alex/terraphim-ai
git checkout HEAD~1 -- crates/terraphim_rlm/src/executor/firecracker.rs
cargo build --release -p terraphim_rlm
sudo cp target/release/libterraphim_rlm.rlib /usr/local/lib/
sudo systemctl restart terraphim*  # if systemd service exists
```

---

## Communication Plan

### Hour 4
- Post initial status to team channel
- Report any immediate issues

### Hour 24  
- Send 24-hour report to stakeholders
- Include metrics and recommendations

### Hour 48
- Send final report
- Get approval for configuration changes (if any)
- Close monitoring task

---

## Success Criteria

- [ ] No critical issues in 48 hours
- [ ] VM allocation <500ms (p95)
- [ ] Error rate <1%
- [ ] Pool utilization 40-80% (optimal range)
- [ ] Final configuration approved

---

## Related Documentation

- Deployment Record: `cto-executive-system/deployments/PR426-fcctl-adapter-deployment.md`
- Architecture: `cto-executive-system/decisions/ADR-001-fcctl-adapter-pattern.md`
- Project Status: `cto-executive-system/projects/PR426-fcctl-adapter-status.md`
- Handover: `terraphim-ai/HANDOVER-2026-03-19.md`

---

**Task Status**: Ready to execute  
**Estimated Effort**: 2-3 hours over 48 hours  
**Priority**: HIGH - Production monitoring required
