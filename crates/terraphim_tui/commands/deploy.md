---
name: deploy
description: Deploy application to production with safety checks
usage: "deploy <environment> [--dry-run] [--force]"
category: Deployment
version: "1.2.0"
risk_level: High
execution_mode: Firecracker
permissions:
  - execute
  - write
knowledge_graph_required:
  - deployment
  - production
  - continuous_integration
parameters:
  - name: environment
    type: string
    required: true
    allowed_values: ["staging", "production", "demo"]
    description: Target deployment environment
  - name: dry_run
    type: boolean
    required: false
    default_value: false
    description: Show what would be deployed without executing
  - name: force
    type: boolean
    required: false
    default_value: false
    description: Skip safety checks and force deployment
resource_limits:
  max_memory_mb: 1024
  max_cpu_time: 600
  network_access: true
timeout: 900
---

# Deploy Command

Deploy application to specified environment with comprehensive safety checks and rollback capabilities.

## Safety Features

- Pre-deployment validation
- Environment-specific configurations
- Rollback capability
- Health checks
- Resource usage monitoring

## Examples

```bash
# Deploy to staging (safe)
deploy staging

# Production deployment with dry run
deploy production --dry-run

# Force deployment (bypass some checks)
deploy demo --force
```

## Environment Configurations

### Staging
- Full integration tests
- Reduced resource limits
- Staging database
- Mock external services

### Production
- Comprehensive health checks
- Resource monitoring
- Automated rollback on failure
- Performance baseline validation

### Demo
- Limited data scope
- Enhanced logging
- Quick rollback capability

## Requirements

This command requires:
- Valid deployment credentials
- Environment access permissions
- Sufficient system resources
- Network connectivity for health checks