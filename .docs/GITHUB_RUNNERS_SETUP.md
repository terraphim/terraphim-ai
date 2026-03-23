# GitHub Actions Multi-Runner Setup Summary

## Overview
Successfully configured 4 concurrent self-hosted GitHub Actions runners on `bigbox` for the terraphim/terraphim-ai repository.

## Runners Configured

| Runner Name | Directory | Status | Service |
|-------------|-----------|--------|---------|
| terraphim-ai-runner-2 | `/home/alex/actions-runner-2` | ✅ Online | `actions.runner.terraphim-terraphim-ai.terraphim-ai-runner-2.service` |
| terraphim-ai-runner-3 | `/home/alex/actions-runner-terraphim-2` | ✅ Online | `actions.runner.terraphim-terraphim-ai.terraphim-ai-runner-3.service` |
| terraphim-ai-runner-4 | `/home/alex/actions-runner-terraphim-4` | ✅ Online | `actions.runner.terraphim-terraphim-ai.terraphim-ai-runner-4.service` |
| terraphim-ai-runner-5 | `/home/alex/actions-runner-terraphim-5` | ✅ Online | `actions.runner.terraphim-terraphim-ai.terraphim-ai-runner-5.service` |

## System Resources

- **CPU**: 24 cores
- **RAM**: 125GB
- **Storage**: 3.5TB (288GB available)
- **Concurrent Jobs**: 4 (can be increased to 6-8 if needed)

## What Was Done

1. **Revived existing runner-2**: Re-registered with GitHub (old registration expired)
2. **Created runner-4**: New instance from scratch
3. **Created runner-5**: New instance from scratch
4. **Runner-3**: Already active (was restarted earlier)

## Commands Used

### Check Runner Status
```bash
ssh bigbox "sudo systemctl list-units --type=service | grep terraphim.*runner"
```

### View Runner Logs
```bash
ssh bigbox "sudo journalctl -u actions.runner.terraphim-terraphim-ai.terraphim-ai-runner-2.service -f"
```

### Restart a Runner
```bash
ssh bigbox "sudo systemctl restart actions.runner.terraphim-terraphim-ai.terraphim-ai-runner-2.service"
```

### List All Runners (GitHub-side)
```bash
gh api repos/terraphim/terraphim-ai/actions/runners | jq -r '.runners[] | "\(.name): \(.status)"'
```

## Benefits

With 4 concurrent runners:
- **Parallel CI jobs**: Up to 4 workflows can run simultaneously
- **Faster PR validation**: Multiple jobs in a workflow run in parallel
- **No queue waiting**: Less time waiting for runners to become available
- **Scalable**: Can add more runners (up to 6-8) given current hardware

## Monitoring

All runners are now active and listening for jobs. You can monitor them via:
1. GitHub Actions UI (Settings > Actions > Runners)
2. Systemd service status on bigbox
3. GitHub CLI: `gh api repos/terraphim/terraphim-ai/actions/runners`

## Future Scaling

To add more runners:
1. Create new directory: `mkdir /home/alex/actions-runner-terraphim-N`
2. Extract runner: `tar xzf actions-runner-linux-x64-*.tar.gz`
3. Get token: `gh api -X POST repos/terraphim/terraphim-ai/actions/runners/registration-token`
4. Configure: `./config.sh --url https://github.com/terraphim/terraphim-ai --token TOKEN --name terraphim-ai-runner-N`
5. Install service: `sudo ./svc.sh install alex`
6. Start service: `sudo systemctl start actions.runner...`

Recommended max: 6-8 runners given current hardware specs.
