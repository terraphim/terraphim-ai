# Bigbox Deployment Plan: Firecracker-Rust + Terraphim Multi-Agent System

**Target Server**: bigbox (SSH access required)
**Date**: 2025-10-06
**Objective**: Deploy complete Terraphim AI multi-agent system with Firecracker VM execution, integrated with existing Caddy infrastructure

---

## 🏗️ Infrastructure Overview

### Existing Infrastructure (Reused)
- ✅ **Caddy Server** with OAuth (GitHub) + JWT authentication
- ✅ **Redis** for session/state management
- ✅ **Cloudflare DNS/TLS** for `*.terraphim.cloud` domains
- ✅ **~/infrastructure/** directory structure
- ✅ **Log rotation** configured in Caddy

### New Components to Deploy
- 🆕 **fcctl-web** (Firecracker VM management HTTP API)
- 🆕 **Terraphim Server** (Multi-agent system with LLM integration)
- 🆕 **Ollama** (Local LLM: llama3.2:3b)
- 🆕 **Agent Workflows** (Starting with parallelization demo)

---

## 📂 Deployment Directory Structure

```
/home/alex/infrastructure/terraphim-private-cloud/
├── firecracker-rust/           # Firecracker VM management
│   ├── fcctl-web/              # Web API binary
│   ├── firecracker-ci-artifacts/  # Firecracker binary
│   ├── ubuntu-focal-*.ext4     # VM root filesystem images
│   └── vmlinux*                # Linux kernels
├── agent-system/               # Terraphim multi-agent codebase
│   ├── target/release/         # Compiled binaries
│   ├── terraphim_server/       # Server with configs
│   ├── crates/                 # Library crates
│   └── examples/               # Workflow examples
├── workflows/                  # Static workflow frontends
│   └── parallelization/        # Multi-perspective analysis demo
├── data/                       # Runtime data
│   ├── knowledge-graph/        # KG data
│   ├── documents/              # Document haystacks
│   └── sessions/               # VM session data
└── logs/                       # Application logs
    ├── fcctl-web.log
    ├── terraphim-server.log
    ├── vm-api.log
    ├── agents-api.log
    └── workflows.log
```

---

## 🌐 Domain/URL Configuration

### Public Endpoints (via Caddy with OAuth)
- **Authentication**: https://auth.terraphim.cloud (existing)
- **Workflows UI**: https://workflows.terraphim.cloud/parallelization/
- **Agent API**: https://agents.terraphim.cloud/
- **VM Management**: https://vm.terraphim.cloud/ (admin-only)

### Internal Endpoints (localhost only)
- **fcctl-web**: http://127.0.0.1:8080
- **Terraphim Server**: http://127.0.0.1:3000
- **Ollama**: http://127.0.0.1:11434
- **Redis**: localhost:6379

---

## 📋 Phase-by-Phase Deployment Steps

### Phase 1: Environment Preparation

#### 1.1 SSH Access & Directory Setup
```bash
# Connect to bigbox
ssh user@bigbox

# Create deployment structure
mkdir -p /home/alex/infrastructure/terraphim-private-cloud/{firecracker-rust,agent-system,workflows,data,logs}
cd ~/infrastructure/terraphim-ai
```

#### 1.2 System Dependencies
```bash
# Update packages
sudo apt-get update && sudo apt-get upgrade -y

# Install Firecracker prerequisites
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  curl \
  git \
  bridge-utils \
  iproute2 \
  jq

# Verify/Install Rust
rustc --version || {
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  rustup default stable
}

# Enable KVM for current user
sudo usermod -aG kvm $USER
newgrp kvm  # Or logout/login
```

#### 1.3 Verify Existing Services
```bash
# Check what's already running
systemctl status caddy redis-server

# Verify Caddy config location
ls -la /etc/caddy/Caddyfile

# Check existing domains
caddy list-modules | grep http
```

---

### Phase 2: Firecracker-Rust Deployment

#### 2.1 Clone/Transfer Repository
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/firecracker-rust

# Option A: Git clone (if repo accessible from bigbox)
git clone https://github.com/terraphim/firecracker-rust.git .

# Option B: SCP from development machine (run from dev machine)
# scp -r /home/alex/projects/terraphim/terraphim-ai/scratchpad/firecracker-rust/* user@bigbox:/home/alex/infrastructure/terraphim-private-cloud/firecracker-rust/
```

#### 2.2 Build Firecracker Components
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/firecracker-rust

# Build all workspace components
cargo build --release --workspace

# Verify builds
ls -lh target/release/{fcctl,fcctl-web}
```

#### 2.3 Download Firecracker Binary
```bash
# Download latest Firecracker release
./download-firecracker-ci.sh

# Verify
./firecracker-ci-artifacts/firecracker --version
```

#### 2.4 Build VM Images
```bash
# Build Ubuntu Focal (20.04) image - recommended for stability
./build-focal-fast.sh

# This creates:
# - ubuntu-focal-rootfs.ext4 (base root filesystem)
# - ubuntu-focal-vmlinux (Linux kernel)
# - ubuntu-focal-ssh.ext4 (SSH-enabled variant)

# Verify images
ls -lh *.ext4 vmlinux*
du -sh *.ext4  # Check sizes
```

#### 2.5 Network Setup for Firecracker VMs
```bash
# Create network setup script
cat > /home/alex/infrastructure/terraphim-private-cloud/setup-vm-network.sh << 'EOF'
#!/bin/bash
# Firecracker VM networking via bridge

# Create bridge
sudo ip link add br0 type bridge 2>/dev/null || true
sudo ip addr add 172.16.0.1/24 dev br0 2>/dev/null || true
sudo ip link set br0 up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE 2>/dev/null || true

echo "VM network bridge configured: br0 (172.16.0.1/24)"
EOF

chmod +x /home/alex/infrastructure/terraphim-private-cloud/setup-vm-network.sh
./setup-vm-network.sh
```

#### 2.6 Create fcctl-web Systemd Service
```bash
# Get current user for service
CURRENT_USER=$(whoami)

sudo tee /etc/systemd/system/fcctl-web.service << EOF
[Unit]
Description=Firecracker Control Web API
After=network.target redis.service

[Service]
Type=simple
User=$CURRENT_USER
WorkingDirectory=/home/alex/infrastructure/terraphim-private-cloud/firecracker-rust
Environment="RUST_LOG=info"
Environment="FIRECRACKER_PATH=/home/alex/infrastructure/terraphim-private-cloud/firecracker-rust/firecracker-ci-artifacts/firecracker"
ExecStartPre=/home/alex/infrastructure/terraphim-private-cloud/setup-vm-network.sh
ExecStart=/home/alex/infrastructure/terraphim-private-cloud/firecracker-rust/target/release/fcctl-web --host 127.0.0.1 --port 8080
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable fcctl-web
sudo systemctl start fcctl-web

# Verify
sudo systemctl status fcctl-web
sleep 3
curl http://127.0.0.1:8080/health
```

---

### Phase 3: Terraphim Agent System Deployment

#### 3.1 Clone/Transfer Agent System
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/agent-system

# Option A: Git clone
git clone https://github.com/terraphim/terraphim-ai.git .

# Option B: SCP from dev machine (run from dev machine)
# scp -r /home/alex/projects/terraphim/terraphim-ai/* user@bigbox:/home/alex/infrastructure/terraphim-private-cloud/agent-system/
```

#### 3.2 Build Agent System
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/agent-system

# Build with all features
cargo build --release --all-features --all-targets

# Verify
ls -lh target/release/terraphim_server
```

#### 3.3 Install Ollama (Local LLM)
```bash
# Check if already installed
if ! command -v ollama &> /dev/null; then
  curl -fsSL https://ollama.com/install.sh | sh
fi

# Enable and start service
sudo systemctl enable ollama
sudo systemctl start ollama

# Pull model
ollama pull llama3.2:3b

# Verify
ollama list
curl http://127.0.0.1:11434/api/tags
```

#### 3.4 Create Agent Configuration
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/agent-system

# Create bigbox-specific config
CURRENT_USER=$(whoami)
cat > terraphim_server/default/bigbox_config.json << EOF
{
  "name": "Bigbox Multi-Agent System",
  "shortname": "BigboxAgent",
  "relevance_function": "terraphim-graph",
  "terraphim_it": true,
  "theme": "lumen",
  "kg": {
    "automata_path": null,
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "/home/alex/infrastructure/terraphim-private-cloud/data/knowledge-graph"
    },
    "public": false,
    "publish": false
  },
  "haystacks": [
    {
      "location": "/home/alex/infrastructure/terraphim-private-cloud/data/documents",
      "service": "Ripgrep",
      "read_only": true,
      "atomic_server_secret": null,
      "extra_parameters": {}
    }
  ],
  "extra": {
    "llm_provider": "ollama",
    "llm_model": "llama3.2:3b",
    "llm_base_url": "http://127.0.0.1:11434",
    "llm_auto_summarize": true,
    "vm_execution": {
      "enabled": true,
      "api_base_url": "http://127.0.0.1:8080",
      "vm_pool_size": 5,
      "default_vm_type": "ubuntu-focal",
      "execution_timeout_ms": 60000,
      "allowed_languages": ["python", "javascript", "bash", "rust"],
      "auto_provision": true,
      "code_validation": true,
      "max_code_length": 10000,
      "history": {
        "enabled": true,
        "snapshot_on_execution": true,
        "snapshot_on_failure": true,
        "auto_rollback_on_failure": false,
        "max_history_entries": 100,
        "persist_history": true,
        "integration_mode": "direct"
      }
    }
  }
}
EOF

# Create data directories
mkdir -p /home/alex/infrastructure/terraphim-private-cloud/data/{knowledge-graph,documents,sessions}
```

#### 3.5 Create Terraphim Server Systemd Service
```bash
CURRENT_USER=$(whoami)

sudo tee /etc/systemd/system/terraphim-server.service << EOF
[Unit]
Description=Terraphim AI Multi-Agent Server
After=network.target fcctl-web.service ollama.service

[Service]
Type=simple
User=$CURRENT_USER
WorkingDirectory=/home/alex/infrastructure/terraphim-private-cloud/agent-system
Environment="RUST_LOG=info"
Environment="TERRAPHIM_DATA_DIR=/home/alex/infrastructure/terraphim-private-cloud/data"  # pragma: allowlist secret
ExecStart=/home/alex/infrastructure/terraphim-private-cloud/agent-system/target/release/terraphim_server --config /home/alex/infrastructure/terraphim-private-cloud/agent-system/terraphim_server/default/bigbox_config.json
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable terraphim-server
sudo systemctl start terraphim-server

# Verify
sudo systemctl status terraphim-server
sleep 3
curl http://127.0.0.1:3000/health
```

---

### Phase 4: Caddy Integration

#### 4.1 Add Terraphim Subdomains to Caddyfile
```bash
# Backup existing Caddyfile
sudo cp /etc/caddy/Caddyfile /etc/caddy/Caddyfile.backup.$(date +%Y%m%d)

# Add Terraphim configuration
CURRENT_USER=$(whoami)

sudo tee -a /etc/caddy/Caddyfile << EOF

# ============================================
# Terraphim AI Multi-Agent System
# ============================================

# VM Management API (admin only)
vm.terraphim.cloud {
    import tls_config
    authorize with mypolicy

    reverse_proxy 127.0.0.1:8080

    log {
        output file /home/alex/infrastructure/terraphim-private-cloud/logs/vm-api.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}

# Agent API (authenticated users)
agents.terraphim.cloud {
    import tls_config
    authorize with mypolicy

    reverse_proxy 127.0.0.1:3000

    # WebSocket support for streaming responses
    @websockets {
        header Connection *Upgrade*
        header Upgrade websocket
    }
    reverse_proxy @websockets 127.0.0.1:3000

    log {
        output file /home/alex/infrastructure/terraphim-private-cloud/logs/agents-api.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}

# Workflow Frontend (authenticated users)
workflows.terraphim.cloud {
    import tls_config
    authorize with mypolicy

    # Serve static workflow files
    root * /home/alex/infrastructure/terraphim-private-cloud/workflows
    file_server

    # API proxy for workflow backend
    handle /api/* {
        reverse_proxy 127.0.0.1:3000
    }

    # WebSocket for VM execution real-time updates
    @ws {
        path /ws
        header Connection *Upgrade*
        header Upgrade websocket
    }
    handle @ws {
        reverse_proxy 127.0.0.1:8080
    }

    log {
        output file /home/alex/infrastructure/terraphim-private-cloud/logs/workflows.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}
EOF
```

#### 4.2 Validate and Reload Caddy
```bash
# Validate configuration
sudo caddy validate --config /etc/caddy/Caddyfile

# Reload Caddy (no downtime)
sudo systemctl reload caddy

# Verify
sudo systemctl status caddy
```

---

### Phase 5: Parallelization Workflow Deployment

#### 5.1 Deploy Workflow Frontend
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/workflows

# Copy parallelization workflow
cp -r /home/alex/infrastructure/terraphim-private-cloud/agent-system/examples/agent-workflows/3-parallelization ./parallelization

# Set correct permissions
chmod -R 755 parallelization/
```

#### 5.2 Configure Workflow API Endpoints
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/workflows/parallelization

# Update API endpoints in workflow config
# Replace localhost with agents.terraphim.cloud
find . -type f -name "*.js" -o -name "*.html" | while read file; do
  sed -i 's|http://localhost:3000|https://agents.terraphim.cloud|g' "$file"
  sed -i 's|ws://localhost:8080|wss://vm.terraphim.cloud|g' "$file"
done
```

#### 5.3 Create Workflow Index Page
```bash
cat > /home/alex/infrastructure/terraphim-private-cloud/workflows/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Terraphim AI Workflows</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css">
</head>
<body>
    <section class="hero is-primary">
        <div class="hero-body">
            <h1 class="title">Terraphim AI Workflows</h1>
            <p class="subtitle">Multi-Agent System Demonstrations</p>
        </div>
    </section>

    <section class="section">
        <div class="container">
            <h2 class="title is-3">Available Workflows</h2>

            <div class="box">
                <h3 class="title is-4">⚡ Parallelization - Multi-Perspective Analysis</h3>
                <p class="content">
                    Demonstrates concurrent execution of multiple AI agents analyzing
                    a topic from different perspectives (Analytical, Creative, Practical,
                    Critical, Strategic, User-Centered).
                </p>
                <a href="/parallelization/" class="button is-primary">Launch Workflow</a>
            </div>

            <div class="box">
                <h3 class="title is-4">🔧 Agent API</h3>
                <p class="content">
                    Direct access to the Terraphim multi-agent API for custom integrations.
                </p>
                <a href="https://agents.terraphim.cloud/api/docs" class="button is-link">API Documentation</a>
            </div>
        </div>
    </section>
</body>
</html>
EOF
```

---

### Phase 6: Testing & Validation

#### 6.1 Create Health Check Script
```bash
cat > /home/alex/infrastructure/terraphim-private-cloud/health-check.sh << 'EOF'
#!/bin/bash
set -e

echo "========================================="
echo "Terraphim Infrastructure Health Check"
echo "========================================="
echo ""

# Internal Services
echo "[1/5] Redis Status"
redis-cli ping && echo "✓ Redis OK" || echo "✗ Redis FAILED"
echo ""

echo "[2/5] fcctl-web Health"
curl -sf http://127.0.0.1:8080/health > /dev/null && echo "✓ fcctl-web OK" || echo "✗ fcctl-web FAILED"
curl -s http://127.0.0.1:8080/health | jq . 2>/dev/null || true
echo ""

echo "[3/5] Ollama Status"
curl -sf http://127.0.0.1:11434/api/tags > /dev/null && echo "✓ Ollama OK" || echo "✗ Ollama FAILED"
curl -s http://127.0.0.1:11434/api/tags | jq '.models[].name' 2>/dev/null || true
echo ""

echo "[4/5] Terraphim Server Health"
curl -sf http://127.0.0.1:3000/health > /dev/null && echo "✓ Terraphim Server OK" || echo "✗ Terraphim Server FAILED"
curl -s http://127.0.0.1:3000/health | jq . 2>/dev/null || true
echo ""

echo "[5/5] Caddy Status"
sudo systemctl is-active --quiet caddy && echo "✓ Caddy OK" || echo "✗ Caddy FAILED"
echo ""

# Public Endpoints (via Caddy)
echo "========================================="
echo "Public Endpoint Status (via Caddy)"
echo "========================================="

check_endpoint() {
    local url=$1
    local name=$2
    if curl -sf -k "$url" > /dev/null 2>&1 || curl -k "$url" 2>&1 | grep -q "401\|403"; then
        echo "✓ $name accessible (auth required)"
    else
        echo "✗ $name NOT accessible"
    fi
}

check_endpoint "https://vm.terraphim.cloud/health" "VM API"
check_endpoint "https://agents.terraphim.cloud/health" "Agents API"
check_endpoint "https://workflows.terraphim.cloud/" "Workflows Frontend"

echo ""
echo "========================================="
echo "Health Check Complete"
echo "========================================="
EOF

chmod +x /home/alex/infrastructure/terraphim-private-cloud/health-check.sh
```

#### 6.2 Run Health Check
```bash
/home/alex/infrastructure/terraphim-private-cloud/health-check.sh
```

#### 6.3 Run VM Execution Tests
```bash
cd /home/alex/infrastructure/terraphim-private-cloud/agent-system

# Unit tests (no fcctl-web required)
./scripts/test-vm-features.sh unit

# Integration tests (requires fcctl-web)
./scripts/test-vm-features.sh integration

# All tests
./scripts/test-vm-features.sh all
```

---

### Phase 7: Security & Hardening

#### 7.1 Firewall Configuration
```bash
# Ensure UFW allows only Caddy ports
sudo ufw status
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# All internal services (fcctl-web:8080, terraphim:3000, ollama:11434)
# are bound to 127.0.0.1 only - not exposed externally
```

#### 7.2 Service Permissions Check
```bash
# Verify services run as non-root
ps aux | grep fcctl-web
ps aux | grep terraphim_server
ps aux | grep ollama

# All should run as your user, not root
```

#### 7.3 Automated Backup
```bash
cat > /home/alex/infrastructure/terraphim-private-cloud/backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR=/home/$USER/infrastructure/backups/terraphim-ai
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR

# Backup configuration and data
tar -czf $BACKUP_DIR/terraphim-ai_$DATE.tar.gz \
  /home/alex/infrastructure/terraphim-private-cloud/data \
  /home/alex/infrastructure/terraphim-private-cloud/workflows \
  /home/alex/infrastructure/terraphim-private-cloud/agent-system/terraphim_server/default/bigbox_config.json

# Keep only last 7 days
find $BACKUP_DIR -name "*.tar.gz" -mtime +7 -delete

echo "Backup completed: $DATE"
ls -lh $BACKUP_DIR/terraphim-ai_$DATE.tar.gz
EOF

chmod +x /home/alex/infrastructure/terraphim-private-cloud/backup.sh

# Add to crontab (daily at 2 AM)
(crontab -l 2>/dev/null; echo "0 2 * * * /home/alex/infrastructure/terraphim-private-cloud/backup.sh >> /home/alex/infrastructure/terraphim-private-cloud/logs/backup.log 2>&1") | crontab -
```

---

### Phase 8: Monitoring (Caddy Metrics)

#### 8.1 Enable Caddy Metrics
```bash
# Caddy already exposes Prometheus metrics at :2019/metrics by default
# Verify
curl http://127.0.0.1:2019/metrics | head -20
```

#### 8.2 Log Locations
```bash
# Service logs (journalctl)
sudo journalctl -fu fcctl-web
sudo journalctl -fu terraphim-server
sudo journalctl -fu ollama

# Application logs (Caddy-managed)
tail -f /home/alex/infrastructure/terraphim-private-cloud/logs/vm-api.log
tail -f /home/alex/infrastructure/terraphim-private-cloud/logs/agents-api.log
tail -f /home/alex/infrastructure/terraphim-private-cloud/logs/workflows.log
```

---

## 🎯 Post-Deployment Verification

### Step 1: Verify All Services Running
```bash
systemctl status fcctl-web terraphim-server ollama caddy redis
```

### Step 2: Test Internal Endpoints
```bash
curl http://127.0.0.1:8080/health      # fcctl-web
curl http://127.0.0.1:3000/health      # terraphim-server
curl http://127.0.0.1:11434/api/tags   # ollama
```

### Step 3: Test Public Endpoints (Requires OAuth Login)
1. **Login**: Navigate to https://auth.terraphim.cloud
2. **Authenticate**: Use GitHub OAuth
3. **Access Workflows**: https://workflows.terraphim.cloud/parallelization/
4. **Test Agent API**: https://agents.terraphim.cloud/health

### Step 4: Run Parallelization Workflow
1. Open https://workflows.terraphim.cloud/parallelization/
2. Enter topic: "Impact of AI on software development"
3. Select perspectives: Analytical, Creative, Practical
4. Select domains: Technical, Business
5. Click "Start Analysis"
6. Verify parallel execution with real-time progress

---

## 📊 System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Internet (HTTPS)                        │
│                  *.terraphim.cloud                          │
└────────────────────────┬────────────────────────────────────┘
                         │
                    ┌────▼─────┐
                    │  Caddy   │ :80/:443
                    │  Server  │ OAuth + JWT + TLS
                    └────┬─────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
    ┌────▼─────┐   ┌────▼────┐    ┌────▼─────┐
    │ vm.      │   │agents.  │    │workflows.│
    │terraphim │   │terraphim│    │terraphim │
    │.cloud    │   │.cloud   │    │.cloud    │
    └────┬─────┘   └────┬────┘    └────┬─────┘
         │              │              │
    ┌────▼─────┐   ┌────▼────┐    ┌────▼─────┐
    │fcctl-web │   │Terraphim│    │ Static   │
    │:8080     │   │Server   │    │ Files    │
    │(localhost)   │:3000    │    │(workflows)│
    │          │   │(localhost)   │          │
    └────┬─────┘   └────┬────┘    └──────────┘
         │              │
         │         ┌────▼─────┐
         │         │  Ollama  │
         │         │  :11434  │
         │         │(localhost)│
         │         └──────────┘
         │
    ┌────▼────────────┐
    │  Firecracker    │
    │  MicroVMs       │
    │  (br0 network)  │
    └─────────────────┘
```

---

## 🔧 Troubleshooting

### Service Not Starting
```bash
# Check logs
sudo journalctl -xeu fcctl-web
sudo journalctl -xeu terraphim-server

# Check port conflicts
sudo lsof -i :8080
sudo lsof -i :3000
sudo lsof -i :11434
```

### VM Creation Fails
```bash
# Check Firecracker binary
/home/alex/infrastructure/terraphim-private-cloud/firecracker-rust/firecracker-ci-artifacts/firecracker --version

# Check KVM access
ls -l /dev/kvm
groups | grep kvm

# Check network bridge
ip addr show br0
```

### Caddy 502 Bad Gateway
```bash
# Verify backend services running
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:3000/health

# Check Caddy error logs
sudo journalctl -fu caddy
```

### OAuth/JWT Issues
```bash
# Verify JWT shared key is set
echo $JWT_SHARED_KEY

# Check GitHub OAuth credentials
echo $GITHUB_CLIENT_ID
echo $GITHUB_CLIENT_SECRET

# Verify cookie domain
grep "cookie domain" /etc/caddy/Caddyfile
```

---

## 📝 Next Steps After Deployment

### Short Term (Week 1)
1. ✅ Deploy additional workflows (routing, orchestrator-workers)
2. ✅ Configure monitoring dashboards (Grafana + Prometheus)
3. ✅ Set up alerting (PagerDuty/Slack)
4. ✅ Create user documentation

### Medium Term (Month 1)
1. Scale VM pool based on workload
2. Implement distributed tracing (Jaeger)
3. Add more LLM providers (OpenRouter, local Mixtral)
4. Create workflow templates library

### Long Term (Quarter 1)
1. Multi-region deployment
2. High availability setup (Caddy cluster)
3. Advanced workflow orchestration
4. ML model fine-tuning pipeline

---

## 📞 Support & Resources

### Documentation
- **Terraphim AI**: https://github.com/terraphim/terraphim-ai
- **Firecracker**: https://firecracker-microvm.github.io/
- **Caddy**: https://caddyserver.com/docs/
- **Ollama**: https://ollama.com/

### Logs Locations
- **fcctl-web**: `journalctl -fu fcctl-web`
- **terraphim-server**: `journalctl -fu terraphim-server`
- **Caddy access**: `/home/alex/infrastructure/terraphim-private-cloud/logs/*.log`

### Health Check
```bash
/home/alex/infrastructure/terraphim-private-cloud/health-check.sh
```

---

**Deployment Checklist**:
- [ ] Phase 1: Environment preparation complete
- [ ] Phase 2: Firecracker-rust deployed
- [ ] Phase 3: Agent system deployed
- [ ] Phase 4: Caddy integration complete
- [ ] Phase 5: Workflows deployed
- [ ] Phase 6: Tests passing
- [ ] Phase 7: Security hardened
- [ ] Phase 8: Monitoring configured
- [ ] Post-deployment verification successful
