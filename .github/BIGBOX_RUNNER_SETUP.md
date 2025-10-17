# Bigbox GitHub Actions Runner Setup

This document explains how to set up the bigbox server as a self-hosted GitHub Actions runner for faster CI/CD pipeline execution.

## Overview

The bigbox runner is configured to use the following labels:
- `self-hosted` - Indicates it's not a GitHub-hosted runner
- `linux` - Indicates the operating system
- `bigbox` - Custom label for this specific runner

## Prerequisites

- Ubuntu 24.04 LTS server (bigbox.terraphim.cloud)
- Docker and Docker Compose installed
- Rust toolchain (1.87.0+)
- Node.js (18+)
- Sufficient disk space for caching
- SSH access for management

## Runner Registration

The runner should be registered with these labels:
```bash
./config.sh --url https://github.com/terraphim/terraphim-ai --token <TOKEN> --labels self-hosted,linux,bigbox
```

## System Requirements

### Minimum Resources
- **CPU**: 4 cores
- **Memory**: 8GB RAM
- **Storage**: 50GB available space
- **Network**: Stable internet connection

### Recommended Resources
- **CPU**: 8+ cores
- **Memory**: 16GB+ RAM
- **Storage**: 100GB+ SSD
- **Network**: High bandwidth for dependency downloads

## Installed Dependencies

The runner should have these dependencies pre-installed:

#### System Dependencies
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    libssl-dev \
    python3 \
    make \
    g++ \
    libcairo2-dev \
    libpango1.0-dev \
    libjpeg-dev \
    libgif-dev \
    librsvg2-dev \
    libnss3-dev \
    libatk-bridge2.0-dev \
    libdrm2 \
    libxkbcommon-dev \
    libxcomposite-dev \
    libxdamage-dev \
    libxrandr-dev \
    libgbm-dev \
    libxss-dev \
    libasound2-dev
```

#### Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
rustup default 1.87.0
rustup component add rustfmt clippy
```

#### Node.js
```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs
```

## Workflow Optimizations

The bigbox runner enables several optimizations:

1. **Faster Build Times**: Local dependencies and toolchain
2. **Better Caching**: Persistent disk storage for Cargo and npm caches
3. **Reduced Network Overhead**: No need to download dependencies each run
4. **Parallel Execution**: Multiple jobs can run simultaneously

## Monitoring

### Runner Status
Check runner status in GitHub Actions settings or via:
```bash
./run.sh --status
```

### System Monitoring
Monitor system resources:
```bash
htop
df -h
free -h
```

## Troubleshooting

### Common Issues

1. **Runner not picking up jobs**
   - Check runner status: `./run.sh --status`
   - Verify labels match workflow requirements
   - Check network connectivity to GitHub

2. **Out of disk space**
   - Clean cargo cache: `cargo clean`
   - Clean npm cache: `npm cache clean --force`
   - Remove old Docker images: `docker system prune -a`

3. **Permission issues**
   - Ensure runner has proper file permissions
   - Check Docker socket access: `sudo usermod -aG docker $USER`

### Logs

Runner logs are located at:
```
/home/runner/_diag/
```

System logs:
```bash
journalctl -u github-runner -f
```

## Security Considerations

- Runner runs with the permissions of the configured user
- Ensure the runner user has minimum required permissions
- Regularly update runner software
- Monitor runner access and usage
- Use fine-grained personal access tokens (PATs)

## Performance Tuning

### Cargo Configuration
Create `~/.cargo/config.toml`:
```toml
[build]
jobs = 4

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]
```

### npm Configuration
```bash
npm config set cache ~/.npm-cache
npm config set prefer-offline true
```

### Docker Configuration
```bash
{
  "storage-driver": "overlay2",
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  }
}
```

## Maintenance

### Weekly Tasks
- Update runner software
- Clean old caches and artifacts
- Check disk space usage
- Review runner performance metrics

### Monthly Tasks
- Update system dependencies
- Review and rotate access tokens
- Audit runner permissions
- Update Rust and Node.js versions

## Contact

For issues with the bigbox runner setup:
- Check GitHub Actions documentation
- Review system logs
- Contact the infrastructure team
