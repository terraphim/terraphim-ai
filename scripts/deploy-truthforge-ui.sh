#!/bin/bash

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIGBOX_HOST="bigbox.terraphim.cloud"
BIGBOX_USER="alex"
DEPLOY_PATH="/home/alex/infrastructure/terraphim-private-cloud-new"

log_info() {
    echo "✓ $1"
}

log_error() {
    echo "✗ $1" >&2
}

phase1_copy_files() {
    log_info "Phase 1: Copy TruthForge UI files to bigbox"
    
    rsync -avz --delete \
        "$PROJECT_ROOT/examples/truthforge-ui/" \
        "$BIGBOX_USER@$BIGBOX_HOST:$DEPLOY_PATH/truthforge-ui/"
    
    log_info "Phase 1 complete"
}

phase2_caddy_integration() {
    log_info "Phase 2: Add Caddy configuration for TruthForge UI"
    
    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

sudo cp /etc/caddy/Caddyfile /etc/caddy/Caddyfile.backup.$(date +%Y%m%d_%H%M%S)

sudo tee -a /etc/caddy/Caddyfile << 'EOF'

alpha.truthforge.terraphim.cloud {
    import tls_config
    authorize with mypolicy
    root * /home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui
    file_server
    handle /api/* {
        reverse_proxy 127.0.0.1:8090
    }
    @ws {
        path /ws
        header Connection *Upgrade*
        header Upgrade websocket
    }
    handle @ws {
        reverse_proxy 127.0.0.1:8090
    }
    log {
        output file /home/alex/infrastructure/terraphim-private-cloud-new/logs/truthforge-alpha.log {
            roll_size 10MiB
            roll_keep 10
            roll_keep_for 168h
        }
        level INFO
    }
}
EOF

sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy

ENDSSH
    
    log_info "Phase 2 complete"
}

phase3_update_endpoints() {
    log_info "Phase 3: Update API endpoints in UI files"
    
    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

cd /home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui

find . -type f \( -name "*.js" -o -name "*.html" \) -exec sed -i \
  -e 's|http://localhost:8080|https://alpha.truthforge.terraphim.cloud|g' \
  -e 's|ws://localhost:8080|wss://alpha.truthforge.terraphim.cloud|g' {} \;

chmod -R 755 /home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui/

ENDSSH
    
    log_info "Phase 3 complete"
}

phase4_start_backend() {
    log_info "Phase 4: Start TruthForge backend with 1Password secrets"
    
    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

cd /home/alex/infrastructure/terraphim-private-cloud-new/truthforge-backend

sudo tee /etc/systemd/system/truthforge-backend.service << 'EOF'
[Unit]
Description=TruthForge Backend API
After=network.target

[Service]
Type=simple
User=alex
WorkingDirectory=/home/alex/infrastructure/terraphim-private-cloud-new/truthforge-backend
ExecStart=/usr/bin/op run --env-file=.env -- /home/alex/.cargo/bin/cargo run --release -- --config truthforge_config.json
Restart=on-failure
RestartSec=10
StandardOutput=append:/home/alex/infrastructure/terraphim-private-cloud-new/logs/truthforge-backend.log
StandardError=append:/home/alex/infrastructure/terraphim-private-cloud-new/logs/truthforge-backend-error.log

[Install]
WantedBy=multi-user.target
EOF

cat > .env << 'EOF'
op://Shared/OpenRouterClaudeCode/api-key
EOF

sudo systemctl daemon-reload
sudo systemctl enable truthforge-backend
sudo systemctl restart truthforge-backend

ENDSSH
    
    log_info "Phase 4 complete"
}

phase5_verify_deployment() {
    log_info "Phase 5: Verify deployment"
    
    ssh "$BIGBOX_USER@$BIGBOX_HOST" bash << 'ENDSSH'
set -e

sleep 5

if systemctl is-active --quiet truthforge-backend; then
    echo "✓ TruthForge backend is running"
else
    echo "✗ TruthForge backend failed to start"
    journalctl -u truthforge-backend -n 50 --no-pager
    exit 1
fi

if curl -s https://alpha.truthforge.terraphim.cloud | grep -q "TruthForge"; then
    echo "✓ TruthForge UI is accessible"
else
    echo "✗ TruthForge UI is not accessible"
    exit 1
fi

if curl -s https://alpha.truthforge.terraphim.cloud/api/health | grep -q "ok"; then
    echo "✓ TruthForge API is responding"
else
    echo "⚠ TruthForge API health check failed (may need backend setup)"
fi

ENDSSH
    
    log_info "Phase 5 complete"
}

main() {
    log_info "Starting TruthForge UI deployment to bigbox"
    
    phase1_copy_files
    phase2_caddy_integration
    phase3_update_endpoints
    phase4_start_backend
    phase5_verify_deployment
    
    log_info "Deployment complete! TruthForge UI available at: https://alpha.truthforge.terraphim.cloud"
}

main "$@"
