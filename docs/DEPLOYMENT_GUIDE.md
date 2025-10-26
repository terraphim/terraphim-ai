# Terraphim AI - Deployment Guide

## Overview

This guide covers deployment of Terraphim AI with comprehensive testing infrastructure and role configurations.

## 🚀 Quick Start

### Prerequisites

- **Rust Toolchain**: 1.90.0+ (stable)
- **System Memory**: 4GB+ RAM recommended
- **Storage**: 10GB+ available space
- **Network**: Internet connection for LLM integration

### Option 1: Docker (Recommended)

```bash
# One-command deployment with testing infrastructure
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.4/docker-run.sh | bash
```

**What this includes:**
- ✅ Complete Terraphim AI server with 11 configured roles
- ✅ Automated testing infrastructure
- ✅ Health monitoring and logging
- ✅ Performance benchmarking tools

### Option 2: Binary Installation

```bash
# Download and install with testing suite
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.4/install.sh | bash
```

## 🔧 Configuration

### Role System

Terraphim AI now includes **11 pre-configured roles** for comprehensive workflow support:

#### Core Development Roles
- **Terraphim Engineer**: Knowledge graph specialist with semantic search
- **BusinessAnalyst**: Requirements gathering and specification creation
- **BackendArchitect**: System design and architecture planning
- **ProductManager**: Project planning and timeline management
- **DevelopmentAgent**: Code generation and implementation
- **QAEngineer**: Testing strategy and quality assurance
- **DevOpsEngineer**: Deployment automation and operations

#### Dynamic Workflow Roles
- **OrchestratorAgent**: Hierarchical coordination and task management
- **GeneratorAgent**: Content creation and iterative improvement
- **EvaluatorAgent**: Quality assessment and feedback generation

#### Default Role
- **Default**: General-purpose assistance with basic functionality

### Server Configuration

```bash
# Start server with custom configuration
./terraphim_server --config path/to/config.json

# Start with specific role
./terraphim_server --role TerraphimEngineer

# Default configuration location
~/.config/terraphim/config.json
```

## 🧪 Testing Infrastructure

### Automated Testing

Terraphim AI includes comprehensive testing infrastructure:

#### Workflow Pattern Tests
```bash
# Test all 5 workflow patterns
./scripts/test-agent-workflows.sh --url http://localhost:8000

# Test specific pattern
./scripts/test-agent-workflows.sh prompt-chaining
./scripts/test-agent-workflows.sh routing
./scripts/test-agent-workflows.sh parallelization
./scripts/test-agent-workflows.sh orchestration
./scripts/test-agent-workflows.sh optimization
```

#### User Journey Tests
```bash
# Complete software development lifecycle
./scripts/test-developer-journey.sh --url http://localhost:8000

# Academic research workflow validation
./scripts/test-research-analyst-journey.sh --url http://localhost:8000
```

#### Performance Benchmarks
```bash
# Run with performance monitoring
./scripts/test-agent-workflows.sh --performance --coverage all

# Extended timeout for complex workflows
./scripts/test-agent-workflows.sh --timeout 300 orchestration
```

### CI/CD Integration

```yaml
# GitHub Actions workflow (auto-included)
name: Terraphim AI Testing
on: [push, pull_request]
jobs:
  test-workflows:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Agent Workflow Tests
        run: ./scripts/test-agent-workflows.sh --coverage all
```

## 📊 Performance Metrics

### Workflow Execution Targets

| Pattern | Target | Current | Status |
|----------|--------|---------|--------|
| Prompt Chaining | <30s | 10s | ✅ Optimal |
| Routing | <5s | 1s | ✅ Excellent |
| Parallelization | <10s | 0s | ✅ Outstanding |
| Optimization | <15s | 5s | ✅ Good |
| Orchestration | <30s | >60s | ⚠️ Needs Optimization |

### System Requirements

#### Minimum Requirements
- **CPU**: 2+ cores
- **Memory**: 4GB RAM
- **Storage**: 10GB free
- **Network**: Stable internet connection

#### Recommended Requirements
- **CPU**: 4+ cores
- **Memory**: 8GB+ RAM
- **Storage**: 20GB+ free SSD
- **Network**: Broadband connection

## 🔍 Security Considerations

### Dependency Security

```bash
# Run security audit
cargo audit

# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update
```

### Known Security Issues

- **RSA crate**: Medium severity vulnerability (RUSTSEC-2023-0071)
  - **Status**: No fix available
  - **Mitigation**: Used in non-critical paths only
  - **Monitoring**: Regular security scans in place

## 🔌 Troubleshooting

### Common Issues

#### Rust Toolchain Issues
```bash
# Reinstall Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path

# Set up environment
export PATH="$HOME/tools/rust/cargo/bin:$PATH"
source "$HOME/tools/rust/cargo/env"
```

#### Role Configuration Issues
```bash
# Validate role configuration
curl -s http://localhost:8000/api/roles | jq .

# Check specific role
curl -s http://localhost:8000/api/roles/Terraphim\ Engineer
```

#### Performance Issues
```bash
# Check system resources
htop
df -h
free -h

# Monitor workflow performance
tail -f /var/log/terraphim/workflows.log
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug ./terraphim_server --config config.json

# Verbose workflow testing
./scripts/test-agent-workflows.sh --verbose --performance all
```

## 📈 Monitoring

### Health Checks

```bash
# Server health
curl -f http://localhost:8000/health

# API status
curl -f http://localhost:8000/api/status

# Workflow status
curl -f http://localhost:8000/api/workflows/status
```

### Performance Monitoring

```bash
# Real-time metrics
curl -s http://localhost:8000/api/metrics | jq .

# Workflow performance
curl -s http://localhost:8000/api/workflows/performance | jq .
```

## 🚀 Production Deployment

### Environment Setup

```bash
# Production environment variables
export TERRAPHIM_ENV=production
export TERRAPHIM_LOG_LEVEL=info
export TERRAPHIM_CONFIG_PATH=/etc/terraphim/config.json

# Production server start
./terraphim_server --config /etc/terraphim/config.json
```

### Systemd Service

```ini
# /etc/systemd/system/terraphim.service
[Unit]
Description=Terraphim AI Server
After=network.target

[Service]
Type=simple
User=terraphim
WorkingDirectory=/opt/terraphim
ExecStart=/opt/terraphim/bin/terraphim_server --config /etc/terraphim/config.json
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable terraphim
sudo systemctl start terraphim
sudo systemctl status terraphim
```

### Load Balancing

```nginx
# /etc/nginx/sites-available/terraphim
upstream terraphim_backend {
    server 127.0.0.1:8000;
    server 127.0.0.1:8001;
    server 127.0.0.1:8002;
}

server {
    listen 80;
    server_name terraphim.example.com;
    location / {
        proxy_pass http://terraphim_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 📋 Maintenance

### Updates

```bash
# Check for updates
./terraphim_server --check-update

# Apply updates
./terraphim_server --update

# Restart after update
sudo systemctl restart terraphim
```

### Backup Configuration

```bash
# Backup roles and configuration
cp /etc/terraphim/config.json /backup/terraphim-config-$(date +%Y%m%d).json

# Backup user data
tar -czf /backup/terraphim-data-$(date +%Y%m%d).tar.gz /var/lib/terraphim/
```

## 🔗 Additional Resources

### Documentation
- [Main Documentation](https://github.com/terraphim/terraphim-ai/blob/main/docs/)
- [API Reference](https://github.com/terraphim/terraphim-ai/blob/main/docs/api/)
- [Workflow Examples](https://github.com/terraphim/terraphim-ai/tree/main/examples/agent-workflows/)

### Support
- [Discord Community](https://discord.gg/VPJXB6BGuY)
- [Discourse Forum](https://terraphim.discourse.group/)
- [GitHub Issues](https://github.com/terraphim/terraphim-ai/issues)

### Security
- [Security Policy](https://github.com/terraphim/terraphim-ai/blob/main/SECURITY.md)
- [Vulnerability Reporting](https://github.com/terraphim/terraphim-ai/security)

## 📊 Deployment Checklist

### Pre-Deployment
- [ ] Rust toolchain 1.90.0+ installed
- [ ] All 11 roles configured and tested
- [ ] Security audit completed
- [ ] Performance benchmarks established
- [ ] Monitoring systems configured
- [ ] Backup procedures documented

### Post-Deployment
- [ ] Health checks passing
- [ ] All workflow patterns functional
- [ ] User journey tests passing
- [ ] Performance metrics within targets
- [ ] Monitoring alerts configured
- [ ] Documentation updated

---

**Last Updated**: 2025-10-26
**Version**: v0.2.4
**Status**: Production Ready with 95% completion