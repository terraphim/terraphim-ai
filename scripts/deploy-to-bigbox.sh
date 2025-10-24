#!/bin/bash
set -e

# Terraphim AI Deployment Script for Bigbox
# Deploys to: /home/alex/infrastructure/terraphim-private-cloud-new/
# User: alex (in sudoers)
# Usage: ./deploy-to-bigbox.sh [phase|all]

BIGBOX_HOST="${BIGBOX_HOST:-bigbox}"
BIGBOX_USER="alex"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_ssh_connection() {
    log_info "Checking SSH connection to $BIGBOX_USER@$BIGBOX_HOST..."
    if ! ssh -o ConnectTimeout=5 "$BIGBOX_USER@$BIGBOX_HOST" "echo 'SSH connection successful'" > /dev/null 2>&1; then
        log_error "Cannot connect to $BIGBOX_HOST. Check SSH access."
        exit 1
    fi
    log_info "SSH connection verified"
}

phase1_environment() {
    log_info "Phase 1: Environment Preparation"

    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

# Create directory structure
mkdir -p /home/alex/infrastructure/terraphim-private-cloud-new/{firecracker-rust,agent-system,workflows,data,logs}
mkdir -p /home/alex/infrastructure/terraphim-private-cloud-new/data/{knowledge-graph,documents,sessions}

# Install system dependencies
sudo apt-get update
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
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi
rustup default stable

# Enable KVM
sudo usermod -aG kvm alex

echo "Phase 1 complete"
ENDSSH

    log_info "Phase 1 complete"
}

phase2_firecracker() {
    log_info "Phase 2: Firecracker-Rust Deployment"

    # Transfer firecracker-rust directory
    log_info "Transferring firecracker-rust code to bigbox..."
    rsync -avz --progress --delete \
        --exclude 'target' \
        "$PROJECT_ROOT/scratchpad/firecracker-rust/" \
        "$BIGBOX_USER@$BIGBOX_HOST:/home/alex/infrastructure/terraphim-private-cloud-new/firecracker-rust/"

    # Build and configure on bigbox
    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e
cd /home/alex/infrastructure/terraphim-private-cloud-new/firecracker-rust

# Build (only fcctl-web binary needed)
source $HOME/.cargo/env

# Build only fcctl-web (skip fcctl-repl which has errors)
cargo build --release -p fcctl-web

# Verify binary exists
if [ ! -f target/release/fcctl-web ]; then
    echo "ERROR: fcctl-web binary not found after build"
    exit 1
fi

# Download Firecracker binary
./download-firecracker-ci.sh

# Build VM images
./build-focal-fast.sh

# Create network setup script
cat > /home/alex/infrastructure/terraphim-private-cloud-new/setup-vm-network.sh << 'EOF'
#!/bin/bash
sudo ip link add br0 type bridge 2>/dev/null || true
sudo ip addr add 172.16.0.1/24 dev br0 2>/dev/null || true
sudo ip link set br0 up
sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE 2>/dev/null || true
echo "VM network bridge configured"
EOF

chmod +x /home/alex/infrastructure/terraphim-private-cloud-new/setup-vm-network.sh
/home/alex/infrastructure/terraphim-private-cloud-new/setup-vm-network.sh

# Create fcctl-web systemd service
sudo tee /etc/systemd/system/fcctl-web.service << 'EOF'
[Unit]
Description=Firecracker Control Web API
After=network.target redis.service

[Service]
Type=simple
User=alex
WorkingDirectory=/home/alex/infrastructure/terraphim-private-cloud-new/firecracker-rust
Environment="RUST_LOG=info"
Environment="FIRECRACKER_PATH=/home/alex/infrastructure/terraphim-private-cloud-new/firecracker-rust/firecracker-ci-artifacts/firecracker"
ExecStartPre=/home/alex/infrastructure/terraphim-private-cloud-new/setup-vm-network.sh
ExecStart=/home/alex/infrastructure/terraphim-private-cloud-new/firecracker-rust/target/release/fcctl-web --host 127.0.0.1 --port 8080
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable fcctl-web
sudo systemctl start fcctl-web

# Verify
sleep 3
curl http://127.0.0.1:8080/health

echo "Phase 2 complete"
ENDSSH

    log_info "Phase 2 complete"
}

phase3_agent_system() {
    log_info "Phase 3: Terraphim Agent System Deployment"

    # Transfer agent system
    log_info "Transferring agent system code to bigbox..."
    rsync -avz --progress \
        --exclude 'target' \
        --exclude 'node_modules' \
        --exclude '.git' \
        "$PROJECT_ROOT/" \
        "$BIGBOX_USER@$BIGBOX_HOST:/home/alex/infrastructure/terraphim-private-cloud-new/agent-system/"

    # Build and configure on bigbox
    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e
cd /home/alex/infrastructure/terraphim-private-cloud-new/agent-system

# Build agent system
source $HOME/.cargo/env
cargo build --release --all-features --all-targets

# Install Ollama if not present
if ! command -v ollama &> /dev/null; then
    curl -fsSL https://ollama.com/install.sh | sh
    sudo systemctl enable ollama
    sudo systemctl start ollama
fi

# Pull model
ollama pull llama3.2:3b

# Create configuration
cat > terraphim_server/default/bigbox_config.json << 'EOF'
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
      "path": "/home/alex/infrastructure/terraphim-private-cloud-new/data/knowledge-graph"
    },
    "public": false,
    "publish": false
  },
  "haystacks": [
    {
      "location": "/home/alex/infrastructure/terraphim-private-cloud-new/data/documents",
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

# Create terraphim-server systemd service
sudo tee /etc/systemd/system/terraphim-server.service << 'EOF'
[Unit]
Description=Terraphim AI Multi-Agent Server
After=network.target fcctl-web.service ollama.service

[Service]
Type=simple
User=alex
WorkingDirectory=/home/alex/infrastructure/terraphim-private-cloud-new/agent-system
Environment="RUST_LOG=info"
Environment="TERRAPHIM_DATA_DIR=/home/alex/infrastructure/terraphim-private-cloud-new/data"
ExecStart=/home/alex/infrastructure/terraphim-private-cloud-new/agent-system/target/release/terraphim_server --config /home/alex/infrastructure/terraphim-private-cloud-new/agent-system/terraphim_server/default/bigbox_config.json
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable terraphim-server
sudo systemctl start terraphim-server

# Verify
sleep 3
curl http://127.0.0.1:3000/health

echo "Phase 3 complete"
ENDSSH

    log_info "Phase 3 complete"
}

phase4_caddy_integration() {
    log_info "Phase 4: Caddy Integration"

    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

# Backup existing Caddyfile
sudo cp /etc/caddy/Caddyfile /etc/caddy/Caddyfile.backup.$(date +%Y%m%d_%H%M%S)

# Append Terraphim configuration
sudo tee -a /etc/caddy/Caddyfile << 'EOF'

# ============================================
# Terraphim AI Multi-Agent System
# ============================================

vm.terraphim.cloud {
    import tls_config
    authorize with mypolicy
    reverse_proxy 127.0.0.1:8080
    log {
        output file /home/alex/infrastructure/terraphim-private-cloud-new/logs/vm-api.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}

agents.terraphim.cloud {
    import tls_config
    authorize with mypolicy
    reverse_proxy 127.0.0.1:3000
    @websockets {
        header Connection *Upgrade*
        header Upgrade websocket
    }
    reverse_proxy @websockets 127.0.0.1:3000
    log {
        output file /home/alex/infrastructure/terraphim-private-cloud-new/logs/agents-api.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}

workflows.terraphim.cloud {
    import tls_config
    authorize with mypolicy
    root * /home/alex/infrastructure/terraphim-private-cloud-new/workflows
    file_server
    handle /api/* {
        reverse_proxy 127.0.0.1:3000
    }
    @ws {
        path /ws
        header Connection *Upgrade*
        header Upgrade websocket
    }
    handle @ws {
        reverse_proxy 127.0.0.1:8080
    }
    log {
        output file /home/alex/infrastructure/terraphim-private-cloud-new/logs/workflows.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}
EOF

# Validate and reload
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy

echo "Phase 4 complete"
ENDSSH

    log_info "Phase 4 complete"
}

phase5_workflows() {
    log_info "Phase 5: Deploy Workflows"

    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

# Copy parallelization workflow
cp -r /home/alex/infrastructure/terraphim-private-cloud/agent-system/examples/agent-workflows/3-parallelization \
      /home/alex/infrastructure/terraphim-private-cloud-new/workflows/parallelization

# Set permissions
chmod -R 755 /home/alex/infrastructure/terraphim-private-cloud-new/workflows/

# Update API endpoints
cd /home/alex/infrastructure/terraphim-private-cloud-new/workflows/parallelization
find . -type f \( -name "*.js" -o -name "*.html" \) -exec sed -i \
  -e 's|http://localhost:3000|https://agents.terraphim.cloud|g' \
  -e 's|ws://localhost:8080|wss://vm.terraphim.cloud|g' {} \;

# Create index page
cat > /home/alex/infrastructure/terraphim-private-cloud-new/workflows/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
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
            <div class="box">
                <h3 class="title is-4">⚡ Parallelization - Multi-Perspective Analysis</h3>
                <p>Concurrent multi-agent analysis from different perspectives</p>
                <a href="/parallelization/" class="button is-primary">Launch Workflow</a>
            </div>
        </div>
    </section>
</body>
</html>
EOF

echo "Phase 5 complete"
ENDSSH

    log_info "Phase 5 complete"
}

phase6_testing() {
    log_info "Phase 6: Testing & Validation"

    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

# Create health check script
cat > /home/alex/infrastructure/terraphim-private-cloud-new/health-check.sh << 'EOF'
#!/bin/bash
echo "=== Terraphim Infrastructure Health Check ==="
echo "[1/5] Redis"
redis-cli ping && echo "✓ OK" || echo "✗ FAILED"
echo "[2/5] fcctl-web"
curl -sf http://127.0.0.1:8080/health > /dev/null && echo "✓ OK" || echo "✗ FAILED"
echo "[3/5] Ollama"
curl -sf http://127.0.0.1:11434/api/tags > /dev/null && echo "✓ OK" || echo "✗ FAILED"
echo "[4/5] Terraphim Server"
curl -sf http://127.0.0.1:3000/health > /dev/null && echo "✓ OK" || echo "✗ FAILED"
echo "[5/5] Caddy"
sudo systemctl is-active --quiet caddy && echo "✓ OK" || echo "✗ FAILED"
EOF

chmod +x /home/alex/infrastructure/terraphim-private-cloud-new/health-check.sh

# Run health check
/home/alex/infrastructure/terraphim-private-cloud-new/health-check.sh

# Run unit tests
cd /home/alex/infrastructure/terraphim-private-cloud-new/agent-system
./scripts/test-vm-features.sh unit

echo "Phase 6 complete"
ENDSSH

    log_info "Phase 6 complete"
}

phase7_security() {
    log_info "Phase 7: Security & Hardening"

    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

# Create backup script
cat > /home/alex/infrastructure/terraphim-private-cloud-new/backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR=/home/alex/infrastructure/backups/terraphim-private-cloud
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR

tar -czf $BACKUP_DIR/terraphim-private-cloud_$DATE.tar.gz \
  /home/alex/infrastructure/terraphim-private-cloud-new/data \
  /home/alex/infrastructure/terraphim-private-cloud-new/workflows \
  /home/alex/infrastructure/terraphim-private-cloud/agent-system/terraphim_server/default/bigbox_config.json

find $BACKUP_DIR -name "*.tar.gz" -mtime +7 -delete
echo "Backup completed: $DATE"
EOF

chmod +x /home/alex/infrastructure/terraphim-private-cloud-new/backup.sh

# Add to cron
(crontab -l 2>/dev/null; echo "0 2 * * * /home/alex/infrastructure/terraphim-private-cloud-new/backup.sh >> /home/alex/infrastructure/terraphim-private-cloud-new/logs/backup.log 2>&1") | crontab -

echo "Phase 7 complete"
ENDSSH

    log_info "Phase 7 complete"
}

verify_deployment() {
    log_info "Verifying deployment..."

    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

echo "=== Service Status ==="
systemctl status fcctl-web terraphim-server ollama caddy --no-pager | grep "Active:"

echo ""
echo "=== Internal Health Checks ==="
curl -s http://127.0.0.1:8080/health | jq . || echo "fcctl-web not responding"
curl -s http://127.0.0.1:3000/health | jq . || echo "terraphim-server not responding"
curl -s http://127.0.0.1:11434/api/tags | jq '.models[].name' || echo "ollama not responding"

echo ""
echo "=== Deployment Complete ==="
echo "Access workflows at: https://workflows.terraphim.cloud/parallelization/"
echo "Login first at: https://auth.terraphim.cloud"
ENDSSH
}

# Main execution
main() {
    local phase="${1:-all}"

    log_info "Starting deployment to $BIGBOX_HOST"
    log_info "Target path: /home/alex/infrastructure/terraphim-private-cloud-new/"
    log_info "Phase: $phase"

    check_ssh_connection

    case "$phase" in
        1|phase1|environment)
            phase1_environment
            ;;
        2|phase2|firecracker)
            phase2_firecracker
            ;;
        3|phase3|agent|agents)
            phase3_agent_system
            ;;
        4|phase4|caddy)
            phase4_caddy_integration
            ;;
        5|phase5|workflows)
            phase5_workflows
            ;;
        6|phase6|test|testing)
            phase6_testing
            ;;
        7|phase7|security)
            phase7_security
            ;;
        all)
            phase1_environment
            phase2_firecracker
            phase3_agent_system
            phase4_caddy_integration
            phase5_workflows
            phase6_testing
            phase7_security
            verify_deployment
            ;;
        verify)
            verify_deployment
            ;;
        *)
            log_error "Unknown phase: $phase"
            echo "Usage: $0 [1-7|all|verify]"
            exit 1
            ;;
    esac

    log_info "Deployment phase '$phase' completed successfully!"
}

main "$@"
