# Terraphim AI Installation and Deployment Guide

This guide covers all available methods to install and deploy Terraphim AI, from quick setup to advanced configurations.

## 🚀 Quick Start

### Option 1: Universal Installer (Recommended)

The universal installer provides a single-command installation for all platforms with automatic platform detection and security verification.

```bash
# Install terraphim-agent (default)
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash

# Install both agent and CLI tools
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash --with-cli

# Install to custom directory
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash --install-dir /usr/local/bin
```

**Features:**
- ✅ Cross-platform support (Linux, macOS, Windows/WSL)
- ✅ Automatic platform detection
- ✅ Security verification with checksums
- ✅ Pre-built binaries when available
- ✅ Fallback to source compilation
- ✅ Multiple installation options

**Installation Options:**
```bash
--install-dir DIR       Custom installation directory (default: ~/.local/bin)
--with-cli              Also install terraphim-cli (automation-focused CLI)
--cli-only              Install only terraphim-cli
--version VERSION       Install specific version (default: latest)
--skip-verify           Skip checksum verification (not recommended)
--verbose               Enable verbose logging
--help, -h              Show help message
```

### Option 2: Docker (Container-based)

Docker provides an isolated environment with all dependencies handled automatically.

```bash
# One-command Docker installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/docker-run.sh | bash
```

**What this does:**
- Downloads the latest Terraphim AI Docker image
- Creates necessary directories for data and configuration
- Starts the server on `http://localhost:8000`
- Enables automatic restart

**After installation:**
- Web Interface: http://localhost:8000
- API Endpoint: http://localhost:8000/api
- Health Check: http://localhost:8000/health

### Option 2: System Package Managers

#### Debian/Ubuntu (18.04+)

```bash
# Download and install server
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.3/terraphim-server_0.2.3-1_amd64.deb
sudo dpkg -i terraphim-server_0.2.3-1_amd64.deb

# Download and install TUI (optional)
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.3/terraphim-agent_0.2.3-1_amd64.deb
sudo dpkg -i terraphim-agent_0.2.3-1_amd64.deb

# Start the server
sudo systemctl start terraphim-server
sudo systemctl enable terraphim-server
```

#### Arch Linux/Manjaro

```bash
# Install server
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.3/terraphim-server-0.2.3-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-server-0.2.3-1-x86_64.pkg.tar.zst

# Install TUI (optional)
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.3/terraphim-agent-0.2.3-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-agent-0.2.3-1-x86_64.pkg.tar.zst

# Start the server
sudo systemctl start terraphim-server
sudo systemctl enable terraphim-server
```

### Option 3: Source Installation

For users who prefer building from source or need custom configurations:

```bash
# Automated source installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash

# Or manual installation
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release
```

## 📋 System Requirements

### Minimum Requirements
- **Operating System**: Linux (Ubuntu 18.04+, CentOS 7+, Arch Linux)
- **Memory**: 4GB RAM minimum, 8GB recommended
- **Storage**: 1GB available space
- **Network**: Internet connection for first-time setup

### Recommended Requirements
- **Operating System**: Ubuntu 20.04+ or Arch Linux
- **Memory**: 8GB RAM or more
- **Storage**: 5GB available space
- **CPU**: Multi-core processor for better performance

## ⚙️ Configuration

### Default Configuration

Terraphim AI creates a default configuration at `~/.config/terraphim/config.json`:

```json
{
  "name": "Terraphim Engineer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "~/Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst"
      }
    }
  ]
}
```

### Adding Data Sources

Edit your configuration to add more haystacks:

```json
{
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "~/Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst"
      }
    },
    {
      "name": "Code Repository",
      "service": "Ripgrep",
      "location": "~/Projects",
      "extra_parameters": {
        "glob": "*.rs,*.js,*.ts,*.py"
      }
    },
    {
      "name": "Knowledge Base",
      "service": "AtomicServer",
      "location": "https://atomic-data.dev",
      "extra_parameters": {}
    }
  ]
}
```

### Available Haystack Types

| Service | Description | Use Case |
|---------|-------------|----------|
| `Ripgrep` | Local file search | Personal documents, code repositories |
| `AtomicServer` | Atomic Data protocol | Knowledge bases, structured data |
| `QueryRs` | Reddit + Rust docs | Community knowledge, documentation |
| `ClickUp` | Task management | Project data, task tracking |
| `Logseq` | Personal knowledge management | Notes, personal knowledge graphs |
| `MCP` | Model Context Protocol | AI tool integration |

### Environment Variables

```bash
# Override configuration location
export TERRAPHIM_SETTINGS_PATH="/custom/path/config.json"

# Set data directory
export TERRAPHIM_DATA_PATH="/custom/data/directory"

# Set logging level
export LOG_LEVEL="debug"  # debug, info, warn, error

# Server configuration
export TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8000"
export TERRAPHIM_SERVER_API_ENDPOINT="http://localhost:8000/api"
```

## 🐳 Docker Deployment

### Basic Docker Setup

```bash
# Pull and run the image
docker run -d \
  --name terraphim-server \
  -p 8000:8000 \
  -v ~/.config/terraphim:/home/terraphim/.config/terraphim \
  -v ~/.local/share/terraphim:/home/terraphim/data \
  --restart unless-stopped \
  ghcr.io/terraphim/terraphim-server:v0.2.3
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'
services:
  terraphim:
    image: ghcr.io/terraphim/terraphim-server:v0.2.3
    container_name: terraphim-server
    ports:
      - "8000:8000"
    volumes:
      - ./config:/home/terraphim/.config/terraphim
      - ./data:/home/terraphim/data
    environment:
      - LOG_LEVEL=info
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

Start with:
```bash
docker-compose up -d
```

### Custom Docker Build

```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

# Build application
WORKDIR /app
COPY . .
RUN cargo build --release --package terraphim_server

# Create non-root user
RUN useradd --create-home --shell /bin/bash terraphim
RUN chown -R terraphim:terraphim /home/terraphim
USER terraphim

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run application
CMD ["terraphim_server"]
```

## 🖥️ Usage

### Web Interface

Access the web interface at `http://localhost:8000`:
- **Search**: Semantic search across your data sources
- **Configuration**: Manage roles and data sources
- **Knowledge Graph**: Visualize concept relationships
- **Chat**: AI-powered assistance

### Terminal Interface (TUI)

The TUI provides a command-line interface with advanced features:

```bash
# Show help
terraphim-agent --help

# Search with TUI
terraphim-agent search "rust programming" --limit 20

# Multi-term search
terraphim-agent search "rust" --terms "async,await" --operator and

# List available roles
terraphim-agent roles list

# Switch role
terraphim-agent search "web" --role "System Operator"

# Interactive mode
terraphim-agent interactive

# REPL mode
terraphim-agent repl
```

### API Usage

#### Health Check
```bash
curl http://localhost:8000/health
```

#### Search API
```bash
curl -X POST http://localhost:8000/api/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "search_term": "rust",
    "limit": 10,
    "role": "Terraphim Engineer"
  }'
```

#### Summarization API
```bash
curl -X POST http://localhost:8000/api/documents/summarize \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Your document content here...",
    "role": "Terraphim Engineer"
  }'
```

#### Chat API
```bash
curl -X POST http://localhost:8000/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Help me understand Rust async programming",
    "role": "Terraphim Engineer"
  }'
```

## 🔧 Advanced Configuration

### Multiple Roles

Create different configurations for different use cases:

```json
{
  "name": "Software Developer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Code Base",
      "service": "Ripgrep",
      "location": "~/Projects",
      "extra_parameters": {
        "glob": "*.rs,*.js,*.ts,*.py,*.go"
      }
    },
    {
      "name": "Documentation",
      "service": "AtomicServer",
      "location": "https://docs.rs",
      "extra_parameters": {}
    }
  ]
}
```

### LLM Integration

Configure AI providers for enhanced features:

```json
{
  "name": "AI Engineer",
  "extra": {
    "llm_provider": "ollama",
    "ollama_base_url": "http://127.0.0.1:11434",
    "ollama_model": "llama3.2:3b"
  }
}
```

Or with OpenRouter:
```json
{
  "name": "AI Engineer",
  "extra": {
    "llm_provider": "openrouter",
    "openrouter_api_key": "your-api-key-here",
    "openrouter_model": "anthropic/claude-3.5-sonnet"
  }
}
```

### Storage Backends

#### Local Storage (Default)
- **Memory**: In-memory storage for testing
- **DashMap**: High-performance concurrent storage
- **SQLite**: Local database storage
- **ReDB**: Embedded key-value database

#### Cloud Storage (Optional)
```bash
# AWS S3 configuration
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export TERRAPHIM_PROFILE_S3_REGION="us-east-1"
export TERRAPHIM_PROFILE_S3_ENDPOINT="https://s3.amazonaws.com/"
```

## 🔍 Troubleshooting

### Common Issues

#### Server Won't Start
```bash
# Check logs
journalctl -u terraphim-server -f

# Or with Docker
docker logs terraphim-server

# Check configuration
terraphim_server --config ~/.config/terraphim/config.json --check
```

#### Permission Errors
```bash
# Fix permissions
sudo chown -R $USER:$USER ~/.config/terraphim
sudo chown -R $USER:$USER ~/.local/share/terraphim
```

#### Port Already in Use
```bash
# Check what's using port 8000
sudo netstat -tlnp | grep :8000

# Kill the process
sudo kill -9 <PID>

# Or use a different port
export TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8080"
```

#### Search Returns No Results
1. Verify your haystack configuration
2. Check if files exist in specified locations
3. Validate glob patterns match your file types
4. Ensure file permissions allow reading

#### Memory Issues
```bash
# Monitor memory usage
htop

# Increase swap space if needed
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Debug Mode

Enable verbose logging:
```bash
# Enable debug logging
export RUST_LOG=debug
export LOG_LEVEL=debug

# Enable backtrace for errors
export RUST_BACKTRACE=1

# Start server with debug
terraphim_server --config ~/.config/terraphim/config.json
```

### Performance Tuning

#### Optimize Search Performance
- Use SSD storage for data directories
- Limit haystack size to responsive amounts
- Choose appropriate relevance function:
  - `TitleScorer`: Fastest, basic text matching
  - `BM25/BM25F`: Good balance of speed and relevance
  - `TerraphimGraph`: Slowest but most accurate

#### Memory Optimization
```bash
# Limit memory usage
export TERRAPHIM_CACHE_SIZE="1GB"
export TERRAPHIM_MAX_CONCURRENT_SEARCHES="4"
```

## 🚀 Production Deployment

### Systemd Service (Linux)

Create `/etc/systemd/system/terraphim-server.service`:

```ini
[Unit]
Description=Terraphim AI Server
After=network.target

[Service]
Type=simple
User=terraphim
Group=terraphim
WorkingDirectory=/home/terraphim
ExecStart=/usr/local/bin/terraphim_server --config /home/terraphim/.config/terraphim/config.json
Restart=always
RestartSec=5

# Environment variables
Environment=RUST_LOG=info
Environment=LOG_LEVEL=info

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/terraphim/.local/share/terraphim

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable terraphim-server
sudo systemctl start terraphim-server
sudo systemctl status terraphim-server
```

### Nginx Reverse Proxy

Create `/etc/nginx/sites-available/terraphim`:

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://localhost:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket support for real-time features
    location /ws {
        proxy_pass http://localhost:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

Enable site:
```bash
sudo ln -s /etc/nginx/sites-available/terraphim /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### SSL/TLS with Let's Encrypt

```bash
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d your-domain.com

# Auto-renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

## 📚 Additional Resources

- [User Guide](https://github.com/terraphim/terraphim-ai/wiki)
- [API Documentation](https://docs.terraphim.ai)
- [Development Setup](development-setup.md)
- [Configuration Reference](configuration.md)
- [Troubleshooting Guide](troubleshooting.md)

## 🤝 Getting Help

- **GitHub Issues**: [Report bugs](https://github.com/terraphim/terraphim-ai/issues)
- **Discussions**: [Community forum](https://github.com/terraphim/terraphim-ai/discussions)
- **Discord**: [Real-time chat](https://discord.gg/VPJXB6BGuY)
- **Discourse**: [Community discussions](https://terraphim.discourse.group)

---

**Terraphim AI v0.2.3** - Privacy-first AI assistant with semantic search capabilities.
