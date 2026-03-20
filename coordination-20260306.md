# AI Dark Factory Coordination Summary - 20260306

Generated: 2026-03-06T20:47:47+01:00

## Health Snapshot
- Telemetry file: 2,897 runs from 2026-03-06T11:27:30+01:00 to 2026-03-06T19:36:50+01:00.
- Critical alerts: none (`/opt/ai-dark-factory/logs/alerts.log` is empty).
- Orchestrator process: running (`adf /opt/ai-dark-factory/orchestrator.toml`).
- System load/memory: load avg 0.18 / 0.24 / 0.63; RAM 3.8 GiB used of 125 GiB.
- Disk: `/` is 97% used (121 GiB free).

## Anomalies Detected
- Missing runs (critical): `meta-coordinator` last run at 2026-03-06T19:36:50+01:00; expected cadence ~10.2s; stale by ~69m.
- Missing runs (warning): `security-sentinel` last run at 2026-03-06T19:27:48+01:00; expected cadence ~60m; stale by ~18m.
- Possible stalled schedule: `upstream-synchronizer` has only one run today (2026-03-06T11:27:34+01:00), so cadence cannot be validated.
- Repeated failures: none found in telemetry (all exits are 0).
- Duration anomalies: none severe (max duration 4s; most runs are 0-2s).
- Related warning signal: `adf.log` shows `security-sentinel` output lag warnings at 2026-03-06T19:30:48+01:00 to 2026-03-06T19:31:48+01:00 (skipped events up to 336).

## Immediate Actions
1. Restart or reconcile `meta-coordinator` and `security-sentinel`; confirm new telemetry within 2 minutes.
2. Reduce root filesystem usage below 90% to lower risk from log/output growth.
3. Route orchestrator WARN events (lag, repeated restarts) into `alerts.log` so critical state is visible without parsing `adf.log`.
4. Define/verify expected cadence for `upstream-synchronizer` and alert when stale > 2x interval.
