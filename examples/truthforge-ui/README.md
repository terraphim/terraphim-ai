# 🎭 TruthForge UI - Crisis Communication Vulnerability Analysis

**Standalone web application for the TruthForge Two-Pass Debate Arena**

## Overview

TruthForge UI is a professional, production-ready interface for analyzing crisis communication narratives through adversarial debate simulation. Built with vanilla JavaScript and designed for independent deployment, it connects to the Terraphim Server API to deliver real-time vulnerability analysis and strategic response generation.

## Features

### 🔍 Comprehensive Narrative Analysis
- **Omission Detection**: Identifies gaps in evidence, unstated assumptions, and absent stakeholder voices
- **Bias Analysis**: Detects loaded language, selective framing, and logical fallacies
- **SCCT Classification**: Maps narratives to Situational Crisis Communication Theory framework
- **Taxonomy Linking**: Connects content to strategic communication domains

### 💬 Two-Pass Adversarial Debate
- **Pass 1**: Balanced debate between supporting and opposing arguments with impartial evaluation
- **Pass 2**: Exploitation-focused debate that weaponizes identified vulnerabilities
- **Vulnerability Amplification**: Quantifies how weaknesses are amplified through adversarial analysis
- **Real-time Progress**: WebSocket streaming of analysis stages

### 🎯 Strategic Response Generation
- **Reframe Strategy**: Empathetic tone with missing context and stakeholder resonance
- **Counter-Argue Strategy**: Assertive evidence-based point-by-point rebuttal
- **Bridge Strategy**: Collaborative dialogic engagement with multiple perspectives
- **Ready-to-Use Drafts**: Social media, press statements, internal memos, Q&A briefs

### 📊 Professional Dashboard
- **Executive Summary**: High-level analysis with risk assessment
- **Interactive Tabs**: Omissions, debate transcripts, vulnerability metrics, response strategies
- **Risk Scoring**: Color-coded severity indicators (Severe/High/Moderate/Low)
- **Export Functionality**: Download complete analysis as JSON

## Architecture

### Technology Stack
- **Frontend**: Vanilla JavaScript (ES6+), no framework dependencies
- **Styling**: Modern CSS with CSS custom properties
- **API Integration**: REST + WebSocket for real-time updates
- **Deployment**: Static files served via Caddy reverse proxy

### Components

```
truthforge-ui/
├── index.html           # Main application interface
├── app.js              # TruthForgeClient + UI controller (600+ lines)
├── styles.css          # Professional design system (800+ lines)
├── websocket-client.js  # WebSocket client library (from agent-workflows)
└── README.md           # This file
```

### API Integration

**Backend Requirements**:
- Terraphim Server with TruthForge module
- REST API endpoint: `POST /api/v1/truthforge`
- WebSocket endpoint: `/ws` with `truthforge_progress` messages

**Environment Variables** (managed via 1Password CLI):
```bash
# Required for real LLM analysis (stored in 1Password)
OPENROUTER_API_KEY=op://Shared/OpenRouterClaudeCode/api-key
```

## Quick Start

### 1. Local Development

```bash
# Navigate to truthforge-ui directory
cd examples/truthforge-ui

# Serve with Python (simplest option)
python3 -m http.server 8081

# Or use Node.js http-server
npx http-server -p 8081

# Or use any static file server
php -S localhost:8081
```

Open browser to `http://localhost:8081`

### 2. Start Backend Server

```bash
# From terraphim-ai root (using 1Password CLI for secrets)
op run --env-file=.env -- cargo run -p terraphim_server --release
```

Create `.env` file:
```bash
echo "op://Shared/OpenRouterClaudeCode/api-key" > .env
```

Backend will start on `http://localhost:8090` by default.

### 3. Analyze a Narrative

1. **Enter Crisis Narrative**: Type or paste your communication text
2. **Set Context**: Choose urgency (Low/High), stakes (checkboxes), audience (Internal/PublicMedia)
3. **Click "Analyze Narrative"**: Submits to backend and shows real-time pipeline progress
4. **View Results**: Explore omissions, debate transcripts, vulnerability metrics, and response strategies

## Deployment

### Production Deployment Pattern

TruthForge UI is designed to be deployed independently from other Terraphim services using the established Caddy + rsync pattern.

**Recommended Topology**:
```
bigbox.terraphim.cloud (Caddy reverse proxy)
├── private.terraphim.cloud:8090 → TruthForge API Backend
└── alpha.truthforge.terraphim.cloud → Alpha UI (K-Partners pilot)
```

### Automated Deployment

Use the deployment script that follows the bigbox deployment pattern:

```bash
# From terraphim-ai root
./scripts/deploy-truthforge-ui.sh
```

This script performs 5 phases:
1. **Copy files**: Rsync to bigbox server
2. **Caddy integration**: Add domain configuration with HTTPS
3. **Update endpoints**: Replace localhost URLs with production URLs
4. **Start backend**: Launch TruthForge API with 1Password secrets
5. **Verify deployment**: Health checks for UI and API

### Manual Deployment Steps

#### 1. Copy Files to Bigbox

```bash
# Rsync files to server
rsync -avz --delete \
  examples/truthforge-ui/ \
  alex@bigbox.terraphim.cloud:/home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui/
```

#### 2. Configure Caddy

Add to `/etc/caddy/Caddyfile`:

```caddy
alpha.truthforge.terraphim.cloud {
    import tls_config
    authorize with mypolicy
    root * /home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui
    file_server
    
    # Proxy API requests to backend
    handle /api/* {
        reverse_proxy 127.0.0.1:8090
    }
    
    # WebSocket support
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
```

Validate and reload:
```bash
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

#### 3. Update API Endpoints

```bash
cd /home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui

# Replace localhost URLs with production URLs
find . -type f \( -name "*.js" -o -name "*.html" \) -exec sed -i \
  -e 's|http://localhost:8090|https://alpha.truthforge.terraphim.cloud|g' \
  -e 's|ws://localhost:8090|wss://alpha.truthforge.terraphim.cloud|g' {} \;

chmod -R 755 .
```

#### 4. Start Backend with 1Password CLI

Create systemd service `/etc/systemd/system/truthforge-backend.service`:

```ini
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
```

Create `.env` file with 1Password reference:
```bash
echo "op://Shared/OpenRouterClaudeCode/api-key" > .env
```

Start service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable truthforge-backend
sudo systemctl start truthforge-backend
```

#### 5. Verify Deployment

```bash
# Check backend status
sudo systemctl status truthforge-backend

# Test UI access
curl -s https://alpha.truthforge.terraphim.cloud | grep "TruthForge"

# Test API health
curl -s https://alpha.truthforge.terraphim.cloud/api/health
```

## Usage Examples

### Example 1: Financial Communication

**Narrative**:
```
We achieved a 40% cost reduction this quarter through process optimization. 
This will improve our operational efficiency and deliver value to shareholders.
```

**Context**:
- Urgency: Low
- Stakes: Financial, Reputational
- Audience: Internal

**Expected Results**:
- **Omissions**: Missing evidence for 40% claim, no employee impact discussed, optimization details absent
- **Risk Score**: ~6.5 (moderate-high)
- **Strategies**: 3 distinct response approaches with ready-to-use drafts

### Example 2: Crisis Response

**Narrative**:
```
Company announces layoffs affecting 500 employees. This strategic restructuring 
positions us for long-term growth and competitiveness.
```

**Context**:
- Urgency: High
- Stakes: Reputational, Legal, Operational, SocialLicense
- Audience: PublicMedia

**Expected Results**:
- **Omissions**: Support resources not mentioned, timeline missing, rationale unclear
- **Risk Score**: ~9.2 (severe)
- **SCCT Classification**: Preventable cluster
- **Vulnerability Delta**: High (>40% amplification in Pass 2)

## API Reference

### Submit Narrative

```javascript
POST /api/v1/truthforge
Content-Type: application/json

{
  "text": "Crisis narrative text...",
  "urgency": "Low" | "High",
  "stakes": ["Reputational", "Financial", "Legal", "Operational", "SocialLicense"],
  "audience": "Internal" | "PublicMedia"
}

Response:
{
  "status": "success",
  "session_id": "uuid",
  "analysis_url": "/api/v1/truthforge/{session_id}"
}
```

### Get Analysis Result

```javascript
GET /api/v1/truthforge/{session_id}

Response (when complete):
{
  "status": "success",
  "result": {
    "session_id": "uuid",
    "omission_catalog": { ... },
    "pass_one_debate": { ... },
    "pass_two_debate": { ... },
    "cumulative_analysis": { ... },
    "response_strategies": [ ... ],
    "executive_summary": "...",
    "processing_time_ms": 45000
  },
  "error": null
}
```

### WebSocket Progress

```javascript
// Subscribe to progress updates
ws://localhost:8090/ws

Message Format:
{
  "message_type": "truthforge_progress",
  "session_id": "uuid",
  "data": {
    "stage": "started" | "completed" | "failed",
    "details": {
      // Stage-specific data
      "narrative_length": 156,
      "omissions_count": 12,
      "strategies_count": 3,
      "total_risk_score": 8.45,
      "processing_time_ms": 45000
    }
  },
  "timestamp": "2025-10-08T12:34:56.789Z"
}
```

## Configuration

### Client Configuration

Edit `app.js` to change default server URL:

```javascript
class TruthForgeUI {
  constructor() {
    // Change baseUrl to point to your backend
    this.client = new TruthForgeClient('http://your-backend-url:8090');
    // ...
  }
}
```

### Styling Customization

Edit CSS custom properties in `styles.css`:

```css
:root {
  --primary: #3b82f6;      /* Primary brand color */
  --risk-severe: #dc2626;  /* Severe risk indicator */
  --risk-high: #f59e0b;    /* High risk indicator */
  --risk-moderate: #fbbf24; /* Moderate risk indicator */
  --risk-low: #10b981;     /* Low risk indicator */
}
```

## Performance

### Expected Response Times

- **Mock Mode** (no OPENROUTER_API_KEY): 100-500ms
- **Real LLM Analysis** (with API key):
  - Short narrative (<200 chars): 30-60s
  - Medium narrative (200-500 chars): 60-90s
  - Long narrative (>500 chars): 90-120s

### Optimization Tips

1. **Use WebSocket**: Real-time progress updates reduce perceived latency
2. **Enable Caching**: Cache analysis results by session_id
3. **CDN Deployment**: Serve static assets from CDN for global distribution
4. **Lazy Loading**: Load heavy components only when needed
5. **Compression**: Enable gzip/brotli compression in nginx

## Security

### CORS Configuration

```nginx
# nginx CORS headers for API requests
add_header 'Access-Control-Allow-Origin' 'https://alpha.truthforge.terraphim.cloud';
add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS';
add_header 'Access-Control-Allow-Headers' 'Content-Type';
```

### CSP Headers

```nginx
# Content Security Policy
add_header Content-Security-Policy "default-src 'self'; \
  script-src 'self' 'unsafe-inline'; \
  style-src 'self' 'unsafe-inline'; \
  connect-src 'self' ws://localhost:8090 wss://private.terraphim.cloud;";
```

### Input Validation

- Character limit enforced (10,000 chars)
- XSS protection via text sanitization
- No eval() or dynamic code execution
- Safe markdown rendering (escapes HTML)

## Troubleshooting

### Connection Issues

**Problem**: "Connection Error" in header

**Solutions**:
1. Verify backend server is running: `curl http://localhost:8090/health`
2. Check CORS settings in backend
3. Inspect browser console for WebSocket errors
4. Confirm port 8090 is not blocked by firewall

### Analysis Timeout

**Problem**: "Analysis timeout" after 120 seconds

**Solutions**:
1. Verify OPENROUTER_API_KEY is set in backend
2. Check backend logs for LLM errors
3. Reduce narrative length for faster processing
4. Increase timeout in `app.js` (pollForResults maxWaitSeconds parameter)

### Empty Results

**Problem**: Results section shows "Loading..." indefinitely

**Solutions**:
1. Check network tab for failed API requests
2. Verify backend API endpoint is `/api/v1/truthforge`
3. Confirm WebSocket connection established
4. Check browser console for JavaScript errors

## Browser Support

- **Chrome/Edge**: 90+
- **Firefox**: 88+
- **Safari**: 14+
- **Mobile**: iOS 14+, Android Chrome 90+

## Future Enhancements

- [ ] **Real-time Collaboration**: Multiple users analyzing same narrative
- [ ] **History Management**: Save and revisit past analyses
- [ ] **Comparison Tool**: Side-by-side analysis of multiple narratives
- [ ] **Custom Branding**: White-label configuration for enterprise clients
- [ ] **Advanced Filters**: Filter omissions by category, sort by risk
- [ ] **PDF Export**: Formatted PDF reports with executive summary
- [ ] **API Key Management**: Self-service API key generation
- [ ] **Cost Tracking**: Per-analysis cost display and budgets

## License

Proprietary - Zestic AI  
Not for public distribution or use

## Support

- **GitHub Issues**: https://github.com/terraphim/terraphim-ai/issues
- **Documentation**: See `crates/terraphim_truthforge/examples/api_usage.md`
- **Backend README**: See `crates/terraphim_truthforge/README.md`

---

**Built with ❤️ by the Terraphim AI Team**  
**Powered by Claude 3.5 Sonnet & Haiku via OpenRouter**
