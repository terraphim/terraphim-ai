---
name: security-audit
description: Perform comprehensive security audit and vulnerability scanning
usage: "security-audit [target] [--deep] [--report] [--fix]"
category: Security
version: "1.0.0"
risk_level: Critical
execution_mode: Firecracker
permissions:
  - read
  - execute
knowledge_graph_required:
  - security
  - vulnerability_assessment
  - compliance
parameters:
  - name: target
    type: string
    required: false
    default_value: "."
    description: Target path or component to audit
  - name: deep
    type: boolean
    required: false
    default_value: false
    description: Perform deep analysis (longer runtime)
  - name: report
    type: boolean
    required: false
    default_value: true
    description: Generate detailed security report
  - name: fix
    type: boolean
    required: false
    default_value: false
    description: Attempt automatic fixes for common issues
resource_limits:
  max_memory_mb: 4096
  max_cpu_time: 3600
  network_access: false
timeout: 7200
---

# Security Audit Command

Comprehensive security vulnerability scanning and compliance checking for applications and infrastructure.

## Security Checks

### Code Analysis
- Dependency vulnerability scanning
- Static code analysis
- Secret detection (API keys, passwords)
- Code quality assessment

### Infrastructure Security
- Configuration security review
- Network exposure analysis
- Permission and access control audit
- Encryption verification

### Compliance Checks
- OWASP Top 10 assessment
- Security best practices validation
- Industry standard compliance
- Regulatory requirement checking

## Examples

```bash
# Basic security audit
security-audit

# Deep analysis with detailed reporting
security-audit --deep --report

# Audit specific component
security-audit ./src/auth

# Auto-fix common security issues
security-audit --fix
```

## Report Categories

### High Severity
- Remote code execution vulnerabilities
- Authentication bypass issues
- Data exposure risks
- System compromise vectors

### Medium Severity
- Information disclosure
- Cross-site scripting (XSS)
- Insecure configurations
- Weak cryptographic implementations

### Low Severity
- Missing security headers
- Outdated dependencies
- Debug information exposure
- Non-critical best practice violations

## Auto-Fix Capabilities

When `--fix` is enabled, the system can automatically:

- Update vulnerable dependencies
- Fix common misconfigurations
- Add missing security headers
- Implement input validation
- Enable secure defaults

## Security Requirements

- Requires elevated privileges for system-level scanning
- Network access for vulnerability database updates
- Sufficient disk space for report generation
- Write access for auto-fix functionality

## Report Format

Reports are generated in multiple formats:
- **JSON**: Machine-readable for CI/CD integration
- **HTML**: Interactive web report with visualizations
- **PDF**: Printable compliance documentation
- **SARIF**: Standardized security findings format