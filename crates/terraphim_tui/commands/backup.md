---
name: backup
description: Create system backups with compression and verification
usage: "backup <target> [destination] [--compress] [--verify]"
category: System Administration
version: "2.0.1"
risk_level: Medium
execution_mode: Hybrid
permissions:
  - read
  - write
aliases:
  - archive
knowledge_graph_required:
  - backup
  - data_protection
parameters:
  - name: target
    type: string
    required: true
    description: Path or resource to backup
  - name: destination
    type: string
    required: false
    description: Backup destination path
  - name: compress
    type: boolean
    required: false
    default_value: true
    description: Enable compression
  - name: verify
    type: boolean
    required: false
    default_value: true
    description: Verify backup integrity
resource_limits:
  max_memory_mb: 2048
  max_disk_mb: 10240
  max_cpu_time: 1800
timeout: 3600
---

# Backup Command

Creates secure backups with compression, verification, and integrity checking.

## Backup Features

- **Compression**: Multiple compression algorithms supported
- **Verification**: Automatic integrity checking
- **Incremental**: Only backup changed files
- **Encryption**: Optional encryption for sensitive data
- **Scheduling**: Automated backup scheduling

## Examples

```bash
# Backup project directory
backup ./project

# Backup with custom destination
backup /etc/config /backups/system

# Uncompressed backup
backup ./data --compress=false

# Skip verification for faster backup
backup ./logs --verify=false
```

## Backup Types

### Local File Backup
- Files and directories
- Preserves permissions and timestamps
- Supports symbolic links

### Database Backup
- Database dumps and schema
- Transaction consistency
- Point-in-time recovery

### System Configuration
- System settings and configs
- Package lists and versions
- Service configurations

## Security Considerations

- Backups are automatically validated for integrity
- Sensitive data can be encrypted
- Access controls prevent unauthorized backups
- Audit trail of backup operations

## Verification Process

1. **File Integrity**: Checksum verification
2. **Restore Test**: Automated restore validation
3. **Size Verification**: Expected size validation
4. **Permission Check**: Access rights verification