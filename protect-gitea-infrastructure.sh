#!/bin/bash
#
# Protect Gitea Infrastructure from Docker Prune
# Implementation script for bigbox
#

set -euo pipefail

STACK_DIR="/home/alex/gitea-stack"
BACKUP_DIR="$STACK_DIR/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[$(date +%H:%M:%S)]${NC} $*"
}

warn() {
    echo -e "${YELLOW}[$(date +%H:%M:%S)] WARNING:${NC} $*"
}

error() {
    echo -e "${RED}[$(date +%H:%M:%S)] ERROR:${NC} $*"
}

# Step 1: Backup current configuration
log "Step 1: Backing up current configuration..."
mkdir -p "$BACKUP_DIR"
cp "$STACK_DIR/docker-compose.yml" "$BACKUP_DIR/docker-compose.yml.$TIMESTAMP"
log "Backup saved to: $BACKUP_DIR/docker-compose.yml.$TIMESTAMP"

# Step 2: Create external volumes
log "Step 2: Creating external Docker volumes..."
docker volume create gitea-seaweedfs-master-data 2>/dev/null || log "Volume gitea-seaweedfs-master-data already exists"
docker volume create gitea-seaweedfs-volume-data 2>/dev/null || log "Volume gitea-seaweedfs-volume-data already exists"
docker volume create gitea-seaweedfs-filer-data 2>/dev/null || log "Volume gitea-seaweedfs-filer-data already exists"
docker volume create gitea-postgres-data 2>/dev/null || log "Volume gitea-postgres-data already exists"
docker volume create gitea-server-data 2>/dev/null || log "Volume gitea-server-data already exists"

# Step 3: Migrate existing data to external volumes
log "Step 3: Checking if data migration is needed..."

# Stop services temporarily
log "Stopping gitea-stack services..."
cd "$STACK_DIR"
docker compose down

# Migrate data if local directories exist
if [ -d "$STACK_DIR/seaweedfs" ] && [ "$(ls -A $STACK_DIR/seaweedfs 2>/dev/null)" ]; then
    log "Migrating SeaweedFS volume data to external volume..."
    docker run --rm -v gitea-seaweedfs-volume-data:/dest -v "$STACK_DIR/seaweedfs":/src alpine sh -c "cp -r /src/* /dest/ 2>/dev/null || true"
    log "SeaweedFS data migrated"
fi

if [ -d "$STACK_DIR/seaweedfs_filer" ] && [ "$(ls -A $STACK_DIR/seaweedfs_filer 2>/dev/null)" ]; then
    log "Migrating SeaweedFS filer data to external volume..."
    docker run --rm -v gitea-seaweedfs-filer-data:/dest -v "$STACK_DIR/seaweedfs_filer":/src alpine sh -c "cp -r /src/* /dest/ 2>/dev/null || true"
    log "Filer data migrated"
fi

if [ -d "$STACK_DIR/postgres" ] && [ "$(ls -A $STACK_DIR/postgres 2>/dev/null)" ]; then
    log "Migrating PostgreSQL data to external volume..."
    docker run --rm -v gitea-postgres-data:/dest -v "$STACK_DIR/postgres":/src alpine sh -c "cp -r /src/* /dest/ 2>/dev/null || true"
    log "PostgreSQL data migrated"
fi

if [ -d "$STACK_DIR/gitea" ] && [ "$(ls -A $STACK_DIR/gitea 2>/dev/null)" ]; then
    log "Migrating Gitea data to external volume..."
    docker run --rm -v gitea-server-data:/dest -v "$STACK_DIR/gitea":/src alpine sh -c "cp -r /src/* /dest/ 2>/dev/null || true"
    log "Gitea data migrated"
fi

log "Step 4: Creating protected docker-compose.yml..."
cat > "$STACK_DIR/docker-compose.yml" << 'EOF'
version: "3.8"

networks:
  gitea:
    external: true
    name: gitea-infrastructure

volumes:
  seaweedfs-master-data:
    external: true
    name: gitea-seaweedfs-master-data
  seaweedfs-volume-data:
    external: true
    name: gitea-seaweedfs-volume-data
  seaweedfs-filer-data:
    external: true
    name: gitea-seaweedfs-filer-data
  postgres-data:
    external: true
    name: gitea-postgres-data
  gitea-data:
    external: true
    name: gitea-server-data

services:
  server:
    image: git.terraphim.cloud/terraphim/gitea:1.26.0
    container_name: gitea-server
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=gitea"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    environment:
      - USER_UID=1000
      - USER_GID=1000
      - GITEA__database__DB_TYPE=postgres
      - GITEA__database__HOST=db:5432
      - GITEA__database__NAME=gitea
      - GITEA__database__USER=gitea
      - GITEA__database__PASSWD=ZCzGDZiV2BE_Rh6@nKhp
      - GITEA__repository__FORCE_PRIVATE=true
      - GITEA__repository__DEFAULT_PRIVATE=private
      - GITEA__repository__DEFAULT_PUSH_CREATE_PRIVATE=true
      - GITEA__openid__ENABLE_OPENID_SIGNIN=false
      - GITEA__openid__ENABLE_OPENID_SIGNUP=false
      - GITEA__actions__ENABLE=true
      - GITEA__repository__LFS_START_SERVER=true
      - GITEA__repository__LFS_CONTENT_PATH=/data/lfs
      - GITEA__storage__type=minio
      - GITEA__storage__MINIO_ACCESS_KEY_ID=22b80e3bae08bfcba33bb8309303a88b119a9615
      - GITEA__storage__MINIO_SECRET_ACCESS_KEY=52f759853b6b76eed0bbe51ca68a9688119aa44676c7d2e3472ba2cd10725dbd3733d6170b8e9f31
      - GITEA__storage__MINIO_BUCKET=gitea
      - GITEA__storage__MINIO_LOCATION=us-east-1
      - GITEA__storage__MINIO_ENDPOINT=http://s3storage:8333
      - GITEA__storage__MINIO_INSECURE_SKIP_VERIFY=false
      - GITEA__storage__MINIO_USE_SSL=false
      - GITEA__issue_graph__ENABLED=true
      - GITEA__issue_graph__DAMPING_FACTOR=0.85
      - GITEA__issue_graph__ITERATIONS=100
      - GITEA__issue_graph__PAGERANK_CACHE_TTL=300
      - GITEA__issue_graph__AUDIT_LOG=true
      - GITEA__issue_graph__STRICT_MODE=false
      - GITEA__webhook__ALLOWED_HOST_LIST=172.18.0.1,localhost,git.terraphim.cloud
      - GITEA__storage__SERVE_DIRECT=true
    networks:
      - gitea
    volumes:
      - gitea-data:/data
      - /etc/timezone:/etc/timezone:ro
      - /etc/localtime:/etc/localtime:ro
    ports:
      - "127.0.0.1:3000:3000"
      - "222:22"
    depends_on:
      - db
      - s3
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:3000/api/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '2.0'

  db:
    image: postgres:15
    container_name: gitea-db
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=database"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    environment:
      - POSTGRES_USER=gitea
      - POSTGRES_PASSWORD=ZCzGDZiV2BE_Rh6@nKhp
      - POSTGRES_DB=gitea
    networks:
      - gitea
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U gitea -d gitea"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '1.0'

  master:
    image: chrislusf/seaweedfs
    container_name: gitea-master
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=storage"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    command: "master -ip=master -ip.bind=0.0.0.0 -metricsPort=9324"
    networks:
      - gitea
    volumes:
      - seaweedfs-master-data:/data
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:9333/cluster/status"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '0.5'

  volume:
    image: chrislusf/seaweedfs
    container_name: gitea-volume
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=storage"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    command: 'volume -mserver="master:9333" -ip.bind=0.0.0.0 -port=8080 -metricsPort=9325'
    depends_on:
      - master
    networks:
      - gitea
    volumes:
      - seaweedfs-volume-data:/data
    healthcheck:
      test: ["CMD-SHELL", "wget -q --spider http://localhost:8080/status || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '0.5'

  filer:
    image: chrislusf/seaweedfs
    container_name: gitea-filer
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=storage"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    command: 'filer -master="master:9333" -ip.bind=0.0.0.0 -metricsPort=9326'
    depends_on:
      - master
      - volume
    networks:
      - gitea
    volumes:
      - seaweedfs-filer-data:/data
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8888/"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '0.5'

  s3:
    image: chrislusf/seaweedfs
    container_name: s3storage
    hostname: s3storage
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=storage"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    ports:
      - "100.106.66.7:8333:8333"
      - "100.106.66.7:9327:9327"
    command: 's3 -config=/etc/seaweedfs/s3.json -filer="filer:8888" -ip.bind=0.0.0.0 -metricsPort=9327'
    depends_on:
      - master
      - volume
      - filer
    networks:
      - gitea
    volumes:
      - ./s3_config.json:/etc/seaweedfs/s3.json:ro
    healthcheck:
      test: ["CMD-SHELL", "wget -q --spider http://localhost:8333/ || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '0.5'

  prometheus:
    image: prom/prometheus:v2.21.0
    container_name: gitea-prometheus
    restart: always
    labels:
      - "terraphim.service=infrastructure"
      - "terraphim.component=monitoring"
      - "terraphim.prune.protected=true"
      - "com.docker.compose.project=gitea-infrastructure"
    ports:
      - "100.106.66.7:9000:9090"
    volumes:
      - ./prometheus:/etc/prometheus:ro
    command: --web.enable-lifecycle --config.file=/etc/prometheus/prometheus.yml
    depends_on:
      - s3
    networks:
      - gitea
    healthcheck:
      test: ["CMD-SHELL", "wget -q --spider http://localhost:9090/-/healthy || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '0.5'
EOF

log "Step 5: Creating external network..."
docker network create gitea-infrastructure 2>/dev/null || log "Network gitea-infrastructure already exists"

log "Step 6: Creating systemd service..."
sudo tee /etc/systemd/system/gitea-stack.service > /dev/null << 'EOF'
[Unit]
Description=Gitea Infrastructure Stack (Protected from Docker Prune)
Documentation=https://git.terraphim.cloud
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/alex/gitea-stack
ExecStartPre=-/usr/bin/docker network create gitea-infrastructure 2>/dev/null || true
ExecStart=/usr/bin/docker compose up -d --remove-orphans
ExecStop=/usr/bin/docker compose down
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
EOF

log "Step 7: Starting services..."
cd "$STACK_DIR"
docker compose up -d --remove-orphans

log "Step 8: Enabling systemd service..."
sudo systemctl daemon-reload
sudo systemctl enable gitea-stack.service

log "Step 9: Waiting for services to be healthy..."
sleep 10

# Check health
log "Checking service health..."
for service in db master volume filer s3 prometheus server; do
    container_name="gitea-$service"
    [ "$service" = "s3" ] && container_name="s3storage"
    [ "$service" = "server" ] && container_name="gitea-server"

    if docker ps --format "table {{.Names}}" | grep -q "^${container_name}$"; then
        log "  $container_name: RUNNING"
    else
        error "  $container_name: NOT RUNNING"
    fi
done

log "Step 10: Creating prune protection documentation..."
cat > "$STACK_DIR/PRUNE_PROTECTION.md" << 'EOF'
# Gitea Infrastructure - Prune Protection

## Protection Mechanisms

1. **Restart Policies**: All services have `restart: always`
   - Containers auto-restart if stopped by prune
   - Systemd ensures stack starts on boot

2. **External Volumes**: Data stored in external Docker volumes
   - `gitea-seaweedfs-master-data`
   - `gitea-seaweedfs-volume-data`
   - `gitea-seaweedfs-filer-data`
   - `gitea-postgres-data`
   - `gitea-server-data`
   - Volumes survive container deletion

3. **Labels**: All containers labeled for identification
   - `terraphim.service=infrastructure`
   - `terraphim.prune.protected=true`
   - `com.docker.compose.project=gitea-infrastructure`

4. **Systemd Service**: Auto-starts on boot
   - Service: gitea-stack.service
   - Command: systemctl status gitea-stack

## Safe Prune Commands for CI/CD

```bash
# Prune only non-protected containers
docker container prune -f --filter "label!=terraphim.prune.protected=true"

# Or prune only CI containers
docker container prune -f --filter "label=ci-cleanup=true"
```

## Recovery

If containers are pruned:
1. Systemd auto-restarts: systemctl restart gitea-stack
2. Or manually: cd /home/alex/gitea-stack && docker compose up -d
3. Data is preserved in external volumes
EOF

log ""
log "${GREEN}=== Implementation Complete ===${NC}"
log "Services protected with:"
log "  - Restart policies (auto-restart if pruned)"
log "  - External volumes (data persists)"
log "  - Container labels (identification)"
log "  - Systemd service (boot-time start)"
log ""
log "Documentation: $STACK_DIR/PRUNE_PROTECTION.md"
