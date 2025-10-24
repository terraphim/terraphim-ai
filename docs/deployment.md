# Terraphim AI Deployment Guide

This guide covers production deployment scenarios, from single-server setups to distributed architectures.

## 🏗️ Production Architecture

### Components Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer  │    │   Nginx/Proxy   │    │  Terraphim AI    │
│   (Optional)     │────│   Reverse       │────│   Server(s)     │
│                 │    │   Proxy         │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                       ┌─────────────────┐             │
                       │   Data Store    │─────────────┤
                       │  (File/Cloud)   │             │
                       │                 │             │
                       └─────────────────┘             │
                                                        │
                       ┌─────────────────┐             │
                       │   AI Provider   │─────────────┘
                       │ (Ollama/OpenAI) │
                       │                 │
                       └─────────────────┘
```

### Deployment Scenarios

#### 1. Single Server Deployment
- **Use Case**: Small teams, personal use, development
- **Resources**: 1-2 CPU cores, 4-8GB RAM, 50GB SSD
- **Setup**: Direct installation or Docker

#### 2. High Availability Deployment
- **Use Case**: Production teams, critical applications
- **Resources**: 2+ servers, load balancer, shared storage
- **Setup**: Multiple instances with load balancing

#### 3. Distributed Deployment
- **Use Case**: Enterprise, large-scale applications
- **Resources**: Multiple regions, CDN, microservices
- **Setup**: Kubernetes, container orchestration

## 🐳 Docker Production Deployment

### Production Dockerfile

```dockerfile
# Multi-stage build for smaller production image
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build dependencies first (cache layer)
RUN cargo build --release --package terraphim_server && \
    rm -rf src

# Build actual application
COPY . .
RUN cargo build --release --package terraphim_server

# Production image
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd --create-home --shell /bin/bash terraphim

WORKDIR /home/terraphim

# Copy binary from builder
COPY --from=builder /app/target/release/terraphim_server /usr/local/bin/

# Create directories
RUN mkdir -p .config/terraphim .local/share/terraphim logs

# Set permissions
RUN chown -R terraphim:terraphim /home/terraphim

USER terraphim

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

EXPOSE 8000

# Use tini for proper signal handling
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["terraphim_server"]
```

### Docker Compose Production

```yaml
version: '3.8'

services:
  terraphim:
    image: terraphim-server:latest
    container_name: terraphim-prod
    restart: unless-stopped
    ports:
      - "127.0.0.1:8000:8000"
    volumes:
      - ./config:/home/terraphim/.config/terraphim:ro
      - ./data:/home/terraphim/data
      - ./logs:/home/terraphim/logs
    environment:
      - RUST_LOG=info
      - LOG_LEVEL=info
      - TERRAPHIM_SERVER_HOSTNAME=0.0.0.0:8000
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '1.0'
        reservations:
          memory: 512M
          cpus: '0.5'

  nginx:
    image: nginx:alpine
    container_name: terraphim-nginx
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
      - ./logs/nginx:/var/log/nginx
    depends_on:
      - terraphim
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Optional: Ollama for local AI
  ollama:
    image: ollama/ollama:latest
    container_name: terraphim-ollama
    restart: unless-stopped
    ports:
      - "127.0.0.1:11434:11434"
    volumes:
      - ./ollama:/root/.ollama
    environment:
      - OLLAMA_HOST=0.0.0.0
    deploy:
      resources:
        limits:
          memory: 8G
          cpus: '2.0'
        reservations:
          memory: 2G
          cpus: '1.0'

networks:
  default:
    driver: bridge
```

## 🔄 Kubernetes Deployment

### Namespace and ConfigMap

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: terraphim
---
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: terraphim-config
  namespace: terraphim
data:
  config.json: |
    {
      "name": "Terraphim Production",
      "relevance_function": "TerraphimGraph",
      "theme": "spacelab",
      "haystacks": [
        {
          "name": "Knowledge Base",
          "service": "AtomicServer",
          "location": "https://atomic-data.dev",
          "extra_parameters": {}
        }
      ]
    }
```

### Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: terraphim-server
  namespace: terraphim
  labels:
    app: terraphim-server
spec:
  replicas: 2
  selector:
    matchLabels:
      app: terraphim-server
  template:
    metadata:
      labels:
        app: terraphim-server
    spec:
      containers:
      - name: terraphim-server
        image: ghcr.io/terraphim/terraphim-server:v0.2.3
        ports:
        - containerPort: 8000
        env:
        - name: RUST_LOG
          value: "info"
        - name: LOG_LEVEL
          value: "info"
        - name: TERRAPHIM_SERVER_HOSTNAME
          value: "0.0.0.0:8000"
        volumeMounts:
        - name: config
          mountPath: /home/terraphim/.config/terraphim
        - name: data
          mountPath: /home/terraphim/data
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: terraphim-config
      - name: data
        persistentVolumeClaim:
          claimName: terraphim-data
```

### Service and Ingress

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: terraphim-service
  namespace: terraphim
spec:
  selector:
    app: terraphim-server
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
  type: ClusterIP
---
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: terraphim-ingress
  namespace: terraphim
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - terraphim.example.com
    secretName: terraphim-tls
  rules:
  - host: terraphim.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: terraphim-service
            port:
              number: 80
```

### Persistent Volume

```yaml
# pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: terraphim-data
  namespace: terraphim
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
  storageClassName: fast-ssd
```

## 🔒 Security Configuration

### Security Headers (Nginx)

```nginx
server {
    listen 443 ssl http2;
    server_name terraphim.example.com;

    # SSL Configuration
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Security Headers
    add_header X-Frame-Options DENY always;
    add_header X-Content-Type-Options nosniff always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' ws: wss:;" always;

    location / {
        proxy_pass http://terraphim-server:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # WebSocket support
    location /ws {
        proxy_pass http://terraphim-server:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw enable
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw deny 8000/tcp  # Direct access to Terraphim (proxy only)

# iptables
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8000 -s 127.0.0.1 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8000 -j DROP
```

### Application Security

```yaml
# docker-compose.security.yml
version: '3.8'
services:
  terraphim:
    image: terraphim-server:latest
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
    user: "1000:1000"
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - SETGID
      - SETUID
    environment:
      - RUST_LOG=warn  # Reduce log noise in production
    # ... other configuration
```

## 📊 Monitoring and Logging

### Prometheus Monitoring

```yaml
# monitoring.yaml
apiVersion: v1
kind: ServiceMonitor
metadata:
  name: terraphim-monitor
  namespace: terraphim
spec:
  selector:
    matchLabels:
      app: terraphim-server
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

### Log Aggregation (ELK Stack)

```yaml
# filebeat.yml
filebeat.inputs:
- type: container
  paths:
    - '/var/lib/docker/containers/*/*.log'
  processors:
    - add_docker_metadata:
        host: "unix:///var/run/docker.sock"

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "terraphim-logs-%{+yyyy.MM.dd}"
```

### Health Check Script

```bash
#!/bin/bash
# health-check.sh

set -e

TERRAPHIM_URL="http://localhost:8000"
WEBHOOK_URL="https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"

check_health() {
    local response=$(curl -s -o /dev/null -w "%{http_code}" "$TERRAPHIM_URL/health")

    if [ "$response" = "200" ]; then
        echo "✅ Terraphim is healthy"
        return 0
    else
        echo "❌ Terraphim is unhealthy (HTTP $response)"

        # Send alert
        curl -X POST -H 'Content-type: application/json' \
            --data '{"text":"🚨 Terraphim AI is down! HTTP '"$response"'"}' \
            "$WEBHOOK_URL"

        return 1
    fi
}

check_performance() {
    local start_time=$(date +%s%N)
    curl -s "$TERRAPHIM_URL/api/documents/search" \
        -H "Content-Type: application/json" \
        -d '{"search_term":"test","limit":1}' > /dev/null
    local end_time=$(date +%s%N)

    local response_time=$(echo "scale=2; ($end_time - $start_time) / 1000000" | bc)

    if (( $(echo "$response_time > 5000" | bc -l) )); then
        echo "⚠️ Slow response time: ${response_time}ms"
        return 1
    else
        echo "✅ Response time: ${response_time}ms"
        return 0
    fi
}

# Main health check
if check_health && check_performance; then
    exit 0
else
    exit 1
fi
```

## 🔄 Backup and Recovery

### Backup Script

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/terraphim"
DATA_DIR="/home/terraphim/.local/share/terraphim"
CONFIG_DIR="/home/terraphim/.config/terraphim"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup data
echo "📦 Backing up Terraphim data..."
tar -czf "$BACKUP_DIR/data_$DATE.tar.gz" -C "$(dirname "$DATA_DIR")" "$(basename "$DATA_DIR")"

# Backup configuration
echo "📋 Backing up configuration..."
tar -czf "$BACKUP_DIR/config_$DATE.tar.gz" -C "$(dirname "$CONFIG_DIR")" "$(basename "$CONFIG_DIR")"

# Cleanup old backups (keep 7 days)
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +7 -delete

echo "✅ Backup completed: $BACKUP_DIR"
```

### Restore Script

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE="$1"
TERRAPHIM_USER="terraphim"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file.tar.gz>"
    exit 1
fi

echo "🔄 Restoring Terraphim from backup..."

# Stop service
sudo systemctl stop terraphim-server

# Restore data
echo "📦 Restoring data..."
sudo -u "$TERRAPHIM_USER" tar -xzf "$BACKUP_FILE" -C "/home/$TERRAPHIM/.local/share/"

# Restore configuration
echo "📋 Restoring configuration..."
sudo -u "$TERRAPHIM_USER" tar -xzf "$BACKUP_FILE" -C "/home/$TERRAPHIM/.config/"

# Fix permissions
sudo chown -R "$TERRAPHIM_USER:$TERRAPHIM_USER" "/home/$TERRAPHIM/.local/share/terraphim"
sudo chown -R "$TERRAPHIM_USER:$TERRAPHIM_USER" "/home/$TERRAPHIM/.config/terraphim"

# Start service
sudo systemctl start terraphim-server

echo "✅ Restore completed"
```

## 🚀 CI/CD Pipeline

### GitHub Actions Production Deployment

```yaml
# .github/workflows/deploy-production.yml
name: Deploy to Production

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production

    steps:
    - name: Checkout
      uses: actions/checkout@v4

    - name: Deploy to server
      uses: appleboy/ssh-action@v1.0.0
      with:
        host: ${{ secrets.HOST }}
        username: ${{ secrets.USERNAME }}
        key: ${{ secrets.SSH_KEY }}
        script: |
          cd /opt/terraphim
          docker-compose pull
          docker-compose up -d
          docker system prune -f

    - name: Health check
      run: |
        sleep 30
        curl -f https://terraphim.example.com/health

    - name: Notify deployment
      uses: 8398a7/action-slack@v3
      with:
        status: ${{ job.status }}
        text: "Terraphim AI deployed to production"
      env:
        SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}
```

## 📈 Performance Optimization

### Production Configuration

```json
{
  "name": "Terraphim Production",
  "relevance_function": "BM25F",
  "theme": "spacelab",
  "extra": {
    "cache_size": "2GB",
    "max_concurrent_searches": 10,
    "indexing_batch_size": 1000,
    "search_timeout_ms": 30000
  },
  "haystacks": [
    {
      "name": "Primary Knowledge Base",
      "service": "Ripgrep",
      "location": "/data/knowledge",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst",
        "max_file_size": "10MB",
        "exclude_patterns": ["*.tmp", "*.log"]
      }
    }
  ]
}
```

### Database Optimization

```bash
# PostgreSQL for metadata (if using)
CREATE INDEX idx_documents_created_at ON documents(created_at);
CREATE INDEX idx_documents_type ON documents(type);
CREATE INDEX idx_concepts_name ON concepts(name);

# Redis for caching
redis-cli CONFIG SET maxmemory 1gb
redis-cli CONFIG SET maxmemory-policy allkeys-lru
```

---

This deployment guide covers production-ready configurations for Terraphim AI. Always test in a staging environment before deploying to production.
