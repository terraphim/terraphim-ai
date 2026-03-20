# AI Dark Factory Daily Coordination Summary (2026-03-06)

Generated: 2026-03-06T20:00:01+01:00

## Health Snapshot
- Telemetry events analyzed: 2,897
- Critical alerts log: no entries in alerts.log
- Overall: degraded (scheduler cadence issue + disk pressure)

## Detected Anomalies
- Missing runs: meta-coordinator appears stalled.
  - Last run: 2026-03-06T19:36:50+01:00
  - Historical cadence: about 10.2 seconds between runs
  - Current staleness: about 22 minutes (well beyond expected)
- Repeated failures: none detected (no non-zero exits)
- Unusual durations: none detected (max duration observed: 4 seconds)

## Critical Alerts
- alerts.log is empty (no critical alert lines found)
- adf.log contains minor drift warnings for security-sentinel around 18:58-18:59 (non-critical)

## System Resources
- Disk: root filesystem at 97% used (/dev/md2, 3.2T of 3.5T, 121G free) -> high risk
- Memory: 8.4Gi used of 125Gi total (healthy)
- Load average: 7.36 / 6.11 / 4.09 on 24 cores (acceptable)
- Top CPU consumer: ollama (about 698% CPU)

## Immediate Actions
1. Restore meta-coordinator scheduling (check process/service and /opt/ai-dark-factory/logs/adf.log around 19:36).
2. Reduce disk pressure on root filesystem (prune old artifacts/logs; target below 90% usage).
3. Add or verify stale-run alerting for meta-coordinator (trigger if no run for more than 5 minutes).
