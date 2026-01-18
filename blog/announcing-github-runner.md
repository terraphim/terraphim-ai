# Announcing Terraphim GitHub Runner: AI-Powered CI/CD with Firecracker MicroVMs

**Date:** 2025-01-31
**Author:** Terraphim AI Team

We're thrilled to announce the **Terraphim GitHub Runner** - a revolutionary CI/CD system that combines LLM-powered workflow understanding with Firecracker microVM isolation for secure, private, and lightning-fast GitHub Actions execution.

## 🚀 Why Build a New GitHub Runner?

Traditional CI/CD runners face three fundamental challenges:

1. **Security**: Shared runners expose your code to other users
2. **Performance**: Cold VMs take minutes to boot
3. **Flexibility**: Static parsers can't understand complex workflows

Terraphim GitHub Runner solves all three with:
- **Isolated Execution**: Each workflow runs in its own Firecracker microVM
- **Sub-2 Second Boot**: MicroVMs start in under 2 seconds
- **AI-Powered Parsing**: LLM understands your workflow intent

## 🤖 AI-Powered Workflow Parsing

The magic starts with our LLM-based workflow parser. Instead of just extracting YAML structure, our system:

```yaml
# Your GitHub Actions workflow
name: Test CI
on: [pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Run tests
        run: cargo test --verbose
```

**Gets transformed by the LLM into:**

```json
{
  "name": "Test CI",
  "steps": [
    {
      "name": "Run tests",
      "command": "cargo test --verbose",
      "working_dir": "/workspace",
      "timeout_seconds": 300
    }
  ],
  "environment": {},
  "setup_commands": ["git clone $REPO_URL /workspace"],
  "cache_paths": ["target/"]
}
```

The LLM understands:
- **Action Translation**: Converts GitHub Actions to shell commands
- **Dependency Detection**: Identifies step dependencies automatically
- **Environment Extraction**: Finds required environment variables
- **Smart Caching**: Suggests cache paths for optimization

## 🔥 Firecracker MicroVM Isolation

Every workflow runs in its own Firecracker microVM with:

### Security Benefits
- **Kernel Isolation**: Separate Linux kernel per VM
- **No Network Access**: By default (configurable)
- **Resource Limits**: CPU and memory constraints enforced
- **Snapshot/Rollback**: Instant recovery from failures

### Performance Benefits
- **Sub-2 Second Boot**: VMs start in ~1.5 seconds
- **Sub-500ms Allocation**: New sessions in ~300ms
- **Minimal Overhead**: MicroVM kernels, not full OS
- **VM Pooling**: Reuse VMs for multiple workflows (coming soon)

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     GitHub Repository                        │
│                     ┌──────────────┐                         │
│                     │   Webhook    │                         │
│                     └──────┬───────┘                         │
└────────────────────────────┼────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│              Terraphim GitHub Runner Server                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  🔐 HMAC-SHA256 Signature Verification              │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │  🔍 Workflow Discovery (.github/workflows/*.yml)    │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │  🤖 LLM Workflow Parser (Ollama/OpenRouter)        │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │  🔧 Firecracker VM Provider                          │  │
│  │  ┌──────────────────────────────────────────────┐   │  │
│  │  │ 🎯 SessionManager (VM lifecycle)             │   │  │
│  │  │ ⚡ VmCommandExecutor (HTTP API)              │   │  │
│  │  │ 🧠 LearningCoordinator (pattern tracking)    │   │  │
│  │  └──────────────────────────────────────────────┘   │  │
│  └──────────────────┬───────────────────────────────────┘  │
└──────────────────────┼──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Firecracker API (fcctl-web)                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  fc-vm-1    │  │  fc-vm-2    │  │  fc-vm-3    │          │
│  │  UUID: abc  │  │  UUID: def  │  │  UUID: ghi  │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

## 📊 Performance Benchmarks

We've measured real-world performance:

| Metric | Value | Notes |
|--------|-------|-------|
| **VM Boot Time** | ~1.5s | Firecracker microVM with Ubuntu |
| **VM Allocation** | ~300ms | Including ID generation |
| **Workflow Parsing (LLM)** | ~500-2000ms | Depends on workflow complexity |
| **Workflow Parsing (Simple)** | ~1ms | YAML-only parsing |
| **End-to-End Latency** | ~2.5-4s | Webhook → VM execution |

**Throughput**: 10+ workflows/second per server instance

## 🎓 Key Features

### 1. Privacy-First Design
- **Local LLM**: Use Ollama for on-premises AI (no data leaves your infra)
- **Cloud Option**: OpenRouter for teams that prefer cloud LLMs
- **No Telemetry**: Zero data sent to external services (your choice)

### 2. Developer Experience
```bash
# Start server with Ollama
USE_LLM_PARSER=true \
OLLAMA_BASE_URL=http://127.0.0.1:11434 \
OLLAMA_MODEL=gemma3:4b \
GITHUB_WEBHOOK_SECRET=your_secret \
FIRECRACKER_API_URL=http://127.0.0.1:8080 \
./target/release/terraphim_github_runner_server
```

**That's it.** Your workflows now run in isolated VMs with AI optimization.

### 3. Pattern Learning
The system tracks execution patterns to optimize future runs:
- Success rate by command type
- Average execution time
- Common failure patterns
- Optimal cache paths
- Timeout recommendations

### 4. Comprehensive Documentation
- **Architecture Docs**: Full system design with Mermaid diagrams
- **Setup Guide**: Step-by-step deployment instructions
- **API Reference**: Complete endpoint documentation
- **Troubleshooting**: Common issues and solutions

## 🔧 Getting Started

### Prerequisites
- Linux system (Ubuntu 20.04+ recommended)
- Firecracker API server (fcctl-web recommended)
- Ollama with gemma3:4b model (optional, for LLM features)

### Installation

```bash
# Clone repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build with Ollama support
cargo build --release -p terraphim_github_runner_server --features ollama

# Install Ollama (if using LLM features)
curl -fsSL https://ollama.com/install.sh | sh
ollama pull gemma3:4b
```

### Configuration

Create `/etc/terraphim/github-runner.env`:

```bash
GITHUB_WEBHOOK_SECRET=your_webhook_secret_here
FIRECRACKER_API_URL=http://127.0.0.1:8080
USE_LLM_PARSER=true
OLLAMA_BASE_URL=http://127.0.0.1:11434
OLLAMA_MODEL=gemma3:4b
```

### GitHub Webhook Setup

```bash
gh api repos/OWNER/REPO/hooks \
  --method POST \
  -f name=terraphim-runner \
  -f active=true \
  -f events='[pull_request,push]' \
  -f config='{
    "url": "https://your-server.com/webhook",
    "content_type": "json",
    "secret": "YOUR_WEBHOOK_SECRET"  # pragma: allowlist secret
  }'
```

## 🎯 Use Cases

### Perfect For:
- **Privacy-Sensitive Projects**: Financial, healthcare, government code
- **Performance-Critical CI**: Need fast feedback loops
- **Complex Workflows**: Multi-stage builds, testing, deployment
- **Resource-Constrained Teams**: Optimize infrastructure costs

### Real-World Examples

#### Example 1: Rust Project CI
```yaml
name: Rust CI
on: [pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build
        run: cargo build --release
      - name: Test
        run: cargo test --verbose
```

**Terraphim executes this in an isolated Firecracker VM with:**
- Automatic workspace mounting
- Rust dependency caching
- Parallel test execution
- Sub-2 second VM provisioning

#### Example 2: Multi-Language Project
```yaml
name: Polyglot CI
on: [push]
jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - run: npm test
  backend:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
  integration:
    runs-on: ubuntu-latest
    steps:
      - run: docker-compose up --abort-on-container-exit
```

**Terraphim handles:**
- Parallel VM allocation for all jobs
- Language-specific environment setup
- Docker-in-Firecracker support
- Integrated result reporting

## 🔮 What's Next?

We're actively working on:

- [ ] **VM Pooling**: Reuse VMs for multiple workflows
- [ ] **Prometheus Metrics**: Comprehensive monitoring
- [ ] **GPU Passthrough**: Hardware acceleration for ML workloads
- [ ] **Distributed Execution**: Multi-server coordination
- [ ] **Custom Action Support**: Run third-party GitHub Actions
- [ ] **Web UI**: Dashboard for workflow monitoring

## 🤝 Contributing

We welcome contributions! See our [GitHub Issues](https://github.com/terraphim/terraphim-ai/issues) for areas where we need help.

**Areas of particular interest:**
- Additional LLM provider integrations
- Performance optimization
- Windows/macOS workflow support
- Documentation improvements
- Bug reports and testing

## 📚 Learn More

- **GitHub Repository**: [terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)
- **Pull Request**: [#381 - GitHub Runner Integration](https://github.com/terraphim/terraphim-ai/pull/381)
- **Architecture Docs**: [docs/github-runner-architecture.md](https://github.com/terraphim/terraphim-ai/blob/main/docs/github-runner-architecture.md)
- **Setup Guide**: [docs/github-runner-setup.md](https://github.com/terraphim/terraphim-ai/blob/main/docs/github-runner-setup.md)

## 🎉 Try It Today

Ready to revolutionize your CI/CD pipeline?

```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release -p terraphim_github_runner_server --features ollama
```

Join us in building the future of secure, AI-powered CI/CD!

---

**About Terraphim AI**

Terraphim AI is building privacy-first AI tools for developers. Our mission is to make powerful AI accessible without compromising on security or privacy. From semantic search to intelligent CI/CD, we're putting developers back in control of their tools.

**Follow Us**
- GitHub: [@terraphim](https://github.com/terraphim)
- Twitter: [@terraphim_ai](https://twitter.com/terraphim_ai) (coming soon)
- Discord: [Join our community](https://discord.gg/terraphim)
