# Terraphim TUI Custom Commands

This directory contains custom command definitions for the Terraphim TUI enhanced slash command system.

## Command System Overview

The Terraphim TUI supports custom commands defined in markdown files with YAML frontmatter. This provides a flexible, extensible command system with:

- **Markdown-based definitions**: Easy to write and maintain
- **Rich metadata**: Parameters, permissions, risk levels, execution modes
- **Security validation**: Knowledge graph integration, rate limiting, blacklisting
- **Multiple execution modes**: Local, Firecracker VM, Hybrid intelligent selection
- **Parameter validation**: Type checking, required fields, allowed values

## Command Structure

Each command is defined in a `.md` file with the following structure:

```yaml
---
name: command-name
description: Human-readable description
usage: "command <required> [optional] [--flag]"
category: Category
version: "1.0.0"
risk_level: Low|Medium|High|Critical
execution_mode: Local|Firecracker|Hybrid
permissions:
  - read
  - write
  - execute
aliases:
  - alt-name
  - another-name
knowledge_graph_required:
  - concept
  - synonym
parameters:
  - name: param-name
    type: string|number|boolean
    required: true|false
    default_value: default
    allowed_values: [option1, option2]
    description: Parameter description
resource_limits:
  max_memory_mb: 512
  max_cpu_time: 300
  network_access: false
timeout: 60
---
```

## Available Commands

### 📁 File Operations
- **search**: Search files and content using ripgrep
- **backup**: Create system backups with compression

### 🔧 Development
- **test**: Run test suites with various test runners

### 🚀 Deployment
- **deploy**: Deploy applications with safety checks

### 🔒 Security
- **security-audit**: Comprehensive security vulnerability scanning

### 🧪 Testing
- **hello-world**: Simple greeting command for testing

## Command Categories

### Risk Levels

- **Low**: Safe commands (read-only operations)
- **Medium**: Commands with write access
- **High**: Commands with system impact
- **Critical**: Commands requiring full isolation

### Execution Modes

- **Local**: Direct execution on host machine (safe commands only)
- **Firecracker**: Isolated execution in microVM (high-risk commands)
- **Hybrid**: Intelligent mode selection based on risk assessment

### Permissions

- **read**: Access files and read data
- **write**: Modify files and system state
- **execute**: Run programs and system commands

## Security Features

### Knowledge Graph Validation
Commands must exist in the knowledge graph as concepts or synonyms to be executable. This ensures only authorized and documented commands can be used.

### Rate Limiting
Commands are rate-limited to prevent abuse:
- **search**: 100 requests per minute
- **deploy**: 5 requests per hour
- Custom limits can be defined per command

### Time Restrictions
Commands can be restricted to specific:
- Business hours (9 AM - 5 PM)
- Weekdays only (Monday - Friday)
- Custom maintenance windows

### Blacklisting
Dangerous command patterns are automatically blocked:
- `rm -rf /`
- `dd if=/dev/zero`
- `mkfs`, `fdisk`
- System-level destructive operations

### Audit Logging
All command executions are logged with:
- User identity
- Command details
- Security validation results
- Timestamp and outcome

## Usage Examples

### List Available Commands
```bash
/commands list
```

### Initialize Command System
```bash
/commands init
```

### Execute a Command
```bash
/search "TODO" --type rs
/deploy staging --dry-run
/backup ./project --verify=true
```

### Get Command Help
```bash
/commands help search
```

### Search Commands
```bash
/commands search "backup"
/commands suggest dep --limit 5
```

### View Statistics
```bash
/commands stats
```

## Writing Custom Commands

1. Create a new `.md` file in this directory
2. Add YAML frontmatter with command metadata
3. Write detailed documentation in markdown
4. Test with `/commands reload`

### Example Command

```markdown
---
name: my-command
description: My custom command description
usage: "my-command <input> [--option]"
category: Custom
version: "1.0.0"
risk_level: Low
execution_mode: Local
permissions:
  - read
parameters:
  - name: input
    type: string
    required: true
    description: Input parameter
  - name: option
    type: boolean
    required: false
    default_value: false
    description: Optional flag
timeout: 30
---

# My Custom Command

Detailed description of what this command does...

## Examples

```bash
my-command "some data" --option
```
```

## Best Practices

### Security
- Use appropriate risk levels
- Request minimal permissions
- Validate all inputs
- Consider execution mode carefully

### Documentation
- Provide clear examples
- Document all parameters
- Include usage notes
- Add troubleshooting section

### Validation
- Use allowed values when possible
- Set sensible defaults
- Include parameter descriptions
- Test validation rules

## Integration with Terraphim

This command system integrates seamlessly with:
- **Knowledge Graph**: Commands validated against concepts
- **Role-based Access**: Different permissions per user role
- **API Client**: Remote command execution
- **Firecracker**: Isolated VM execution when needed
- **Audit System**: Complete security logging