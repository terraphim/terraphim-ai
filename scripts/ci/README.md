# scripts/ci/ -- CI and Infrastructure Health Scripts

Host: bigbox (terraphim-ai development server)

## runner-health-check.sh

Checks Gitea act-runner health via API and process detection.

```bash
# One-off check
./scripts/ci/runner-health-check.sh

# With custom stale threshold
./scripts/ci/runner-health-check.sh --stale-minutes 15

# Cron (every 10 minutes)
*/10 * * * * /home/alex/projects/terraphim/terraphim-ai/scripts/ci/runner-health-check.sh
```

## runner-service-drop-in.conf

Systemd restart policy for act-runner. Apply with:

```bash
sudo mkdir -p /etc/systemd/system/act-runner.service.d/
sudo cp scripts/ci/runner-service-drop-in.conf /etc/systemd/system/act-runner.service.d/restart.conf
sudo systemctl daemon-reload
sudo systemctl restart act-runner
```

## memory-alert.sh

Memory usage alerting for adf-orchestrator and other services.

```bash
# Check adf-orchestrator at 80% threshold
./scripts/ci/memory-alert.sh --service adf-orchestrator --threshold 80

# Cron (every 5 minutes)
*/5 * * * * /home/alex/projects/terraphim/terraphim-ai/scripts/ci/memory-alert.sh --service adf-orchestrator --threshold 80
```
