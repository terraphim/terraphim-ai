# Terraphim AI Examples & Integration Guide

Comprehensive collection of examples, tutorials, and integration patterns for Terraphim AI across all platforms and use cases.

## 🎯 Quick Navigation

### 🔬 For Beginners
- [Getting Started](#getting-started) - 5-minute quick start guide
- [Basic Search Examples](#basic-search) - Simple search patterns
- [First AI Chat](#basic-ai-chat) - Conversational AI basics

### 👨‍💻 For Developers  
- [Code Integration](#code-integration) - IDE and build system integration
- [API Examples](#api-examples) - REST API and SDK usage
- [Web Development](#web-development) - Frontend integration examples

### 🔬 For Advanced Users
- [Multi-Source Search](#advanced-search) - Complex search strategies
- [Custom Scoring](#custom-scoring) - Domain-specific optimization
- [Automation Workflows](#automation) - Advanced automation patterns

### 🏢 For Enterprise
- [Team Integration](#team-integration) - Jira, Confluence, SharePoint
- [Security Setup](#security-setup) - Enterprise security configuration
- [Performance Tuning](#performance) - Large-scale optimization

---

## 🔬 Getting Started

### [Quick Start Tutorial](agent-workflows/1-prompt-chaining/)
**Learn**: Step-by-step coding environment with 6-stage development pipeline
**Time**: 15 minutes
**Skills**: Basic CLI usage, AI interaction, workflow automation

### [First Search](agent-workflows/2-routing/)
**Learn**: Lovable-style prototyping with intelligent model selection
**Time**: 10 minutes  
**Skills**: Model selection, cost optimization, prompt routing

### [Basic AI Chat](agent-workflows/4-orchestrator-workers/)
**Learn**: Multi-perspective analysis with 6 concurrent AI viewpoints
**Time**: 20 minutes
**Skills**: Parallel processing, synthesis, AI orchestration

---

## 👨‍💻 Code Integration

### [Claude Code Integration](claude-code-hooks/)
**Purpose**: Direct IDE integration with Claude Code
**Features**: 
- Real-time document search in IDE
- Context-aware code suggestions
- Automatic documentation lookup
- Knowledge graph exploration in editor

**Setup**:
```bash
# Install Claude Code extension
npm install -g @terraphim/claude-code-hook

# Configure Terraphim integration
terraphim-agent config set ide.claude.enabled true
```

### [Cursor Integration](codebase-evaluation/)
**Purpose**: Advanced codebase evaluation and analysis
**Features**:
- Comprehensive codebase analysis
- Security vulnerability scanning
- Performance optimization suggestions
- Technical debt assessment

**Usage**:
```bash
# Run codebase evaluation
terraphim-agent evaluate --path ~/my-project --depth 3 --security-scan

# Generate improvement report
terraphim-agent evaluate --path ~/my-project --report detailed
```

### [Package Manager Integration](claude-skills/terraphim-package-manager/)
**Purpose**: Package management and development workflow automation
**Features**:
- Multi-registry package management
- Dependency analysis and updates
- Build tool integration
- Development environment setup

**Setup**:
```bash
# Initialize in project directory
terraphim-pm init

# Add dependencies
terraphim-pm add react typescript

# Automated workflow
terraphim-pm build --test && terraphim-pm publish
```

---

## 🌐 Web Development

### [Autocomplete Engine](../terraphim_ai_nodejs/)
**Purpose**: Real-time search autocomplete for web applications
**Features**:
- Sub-millisecond search responses
- Fuzzy matching and typo tolerance
- Multi-language support (JavaScript, TypeScript, Python)
- Knowledge graph integration

**JavaScript Example**:
```javascript
import { TerraphimAutocomplete } from '@terraphim/autocomplete';

const engine = new TerraphimAutocomplete({
  dataPath: './data',
  scorer: 'tfidf'
});

const results = await engine.search('your query');
console.log(results);
```

### [React Integration](../terraphim_ai_nodejs/)
**Purpose**: React components for AI-powered search
**Features**:
- Search input component with autocomplete
- Results display with highlighting
- Knowledge graph visualization
- Real-time search suggestions

**React Example**:
```jsx
import { TerraphimSearch, TerraphimKnowledgeGraph } from '@terraphim/autocomplete/react';

function App() {
  return (
    <div>
      <TerraphimSearch 
        onSearch={handleSearch}
        placeholder="Search your knowledge..."
      />
      <TerraphimKnowledgeGraph query="your-query" />
    </div>
  );
}
```

---

## 🔍 Advanced Search

### [Multi-Source Integration](agent-workflows/3-parallelization/)
**Purpose**: Search across multiple data sources simultaneously
**Features**:
- Parallel search execution
- Result aggregation and ranking
- Source-specific optimization
- Conflict resolution between sources

**Configuration**:
```bash
# Setup multiple sources
terraphim-agent add-source local --path ~/Documents
terraphim-agent add-source github --owner "username" --repo "repository"
terraphim-agent add-source confluence --url "https://company.atlassian.net"

# Search across all sources
terraphim-agent search --source local,github,confluence "query"
```

### [Custom Scoring](../terraphim_service/src/score/)
**Purpose**: Domain-specific search relevance optimization
**Available Scorers**:
- **BM25**: Best for technical documents and code
- **TFIDF**: Excellent for general knowledge discovery
- **Jaccard**: Perfect for exact matches and deduplication

**Custom Implementation**:
```rust
// Custom scorer example
use terraphim_service::score::{Scorer, Document};

pub struct CustomScorer {
    domain_weights: HashMap<String, f64>,
}

impl Scorer for CustomScorer {
    fn score(&self, query: &str, doc: &Document) -> f64 {
        // Custom scoring logic
        let base_score = self.base_scorer.score(query, doc);
        let domain_boost = self.domain_weights
            .get(doc.domain)
            .unwrap_or(1.0);
        base_score * domain_boost
    }
}
```

---

## 🤖 Automation Workflows

### [Advanced Orchestration](agent-workflows/5-evaluator-optimizer/)
**Purpose**: Complex AI workflow management and optimization
**Features**:
- Multi-stage processing pipelines
- Quality evaluation and optimization
- Cost management and monitoring
- Error handling and recovery

**Workflow Example**:
```yaml
# advanced-workflow.yaml
name: "Research Analysis Pipeline"
stages:
  - name: "document-collection"
    type: "search"
    sources: ["local", "github"]
    
  - name: "ai-analysis"
    type: "chat"
    model: "claude-3-sonnet"
    prompt: "Analyze these documents for key themes"
    
  - name: "quality-evaluation"
    type: "score"
    scorer: "custom-domain"
    
  - name: "report-generation"
    type: "template"
    template: "research-analysis"
```

### [Batch Processing](../scripts/)
**Purpose**: Large-scale document processing and analysis
**Features**:
- Parallel document indexing
- Bulk search operations
- Automated report generation
- Performance monitoring

**Usage**:
```bash
# Batch process documents
terraphim-agent batch-process --input ./documents --output ./results --format json

# Parallel searches from file
terraphim-agent batch-search --input ./queries.txt --parallel 4
```

---

## 🏢 Enterprise Integration

### [Jira Integration](../haystack_atlassian/)
**Purpose**: Search across Jira tickets and project documentation
**Features**:
- Real-time Jira ticket search
- Project and component context
- Issue tracking and status updates
- Integration with Confluence

**Setup**:
```bash
# Configure Jira connection
terraphim-agent add-source jira \
  --url "https://company.atlassian.net" \
  --username "your-email@company.com" \
  --project "PROJECTKEY"

# Search Jira tickets
terraphim-agent search --source jira "bug in authentication module"
```

### [SharePoint Integration](../haystack_discourse/)
**Purpose**: Enterprise document repository integration
**Features**:
- SharePoint document indexing
- Permission-aware search
- Version history tracking
- Integration with Microsoft 365

**Configuration**:
```toml
[sources.sharepoint]
url = "https://company.sharepoint.com"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
scopes = ["Sites.Read.All", "User.Read.All"]

[search.sharepoint]
include_document_libraries = true
include_sites = true
max_results = 50
```

---

## 🔧 Security Setup

### [Enterprise Security](../terraphim_service/src/security/)
**Purpose**: Security configuration for enterprise environments
**Features**:
- Advanced authentication methods
- Data encryption at rest
- Access control and audit logging
- Integration with enterprise security systems

**Authentication Setup**:
```bash
# OAuth2 integration
terraphim-agent config set auth.provider "oauth2"
terraphim-agent config set auth.client_id "your-client-id"
terraphim-agent config set auth.client_secret "your-client-secret"

# SAML integration
terraphim-agent config set auth.provider "saml"
terraphim-agent config set auth.idp_url "https://company.idp.com"
```

### [Access Control](../terraphim_settings/src/access_control/)
**Purpose**: Fine-grained access control for data and features
**Features**:
- Role-based permissions
- Team-based access control
- Feature-level restrictions
- Audit logging and compliance reporting

**Configuration**:
```toml
[access_control]
enabled = true
default_role = "reader"

[roles.admin]
permissions = ["read", "write", "delete", "admin"]

[roles.analyst]
permissions = ["read", "write"]
restrictions = ["sensitive_data"]

[roles.reader]
permissions = ["read"]
restrictions = ["api_keys", "configuration"]
```

---

## 📊 Performance Tuning

### [Large-Scale Optimization](../desktop/tests/performance/)
**Purpose**: Performance optimization for enterprise deployments
**Metrics**:
- Search latency and throughput
- Memory usage optimization
- Concurrent user capacity
- Cache hit rates and optimization

**Optimization Strategies**:
```bash
# Memory optimization
terraphim-agent config set performance.cache_size "4GB"
terraphim-agent config set performance.max_concurrent_queries "8"

# Search optimization
terraphim-agent config set search.indexing_threads "4"
terraphim-agent config set search.batch_size "100"

# Network optimization
terraphim-agent config set network.connection_pool_size "16"
terraphim-agent config set network.timeout_seconds "45"
```

### [Monitoring Setup](../terraphim_agent/monitoring/)
**Purpose**: Production monitoring and observability
**Features**:
- Performance metrics collection
- Error rate monitoring
- Usage analytics
- Health check endpoints

**Configuration**:
```bash
# Enable monitoring
terraphim-agent config set monitoring.enabled true
terraphim-agent config set monitoring.metrics_endpoint "https://monitoring.company.com"

# Health checks
terraphim-agent health-check --detailed

# Performance metrics
terraphim-agent metrics --export prometheus --format json
```

---

## 🔗 API Reference

### [REST API Examples](../terraphim_server/)
**Purpose**: HTTP API integration examples
**Endpoints**:
- `/api/v1/search` - Semantic search
- `/api/v1/chat` - AI conversation
- `/api/v1/knowledge-graph` - Graph operations
- `/api/v1/documents` - Document management

**JavaScript Example**:
```javascript
// Advanced API usage
const response = await fetch('http://localhost:8090/api/v1/search', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    query: 'your search query',
    sources: ['local', 'github'],
    scorer: 'tfidf',
    max_results: 20
  })
});

const results = await response.json();
console.log('Search results:', results);
```

### [WebSocket Integration](../terraphim_server/)
**Purpose**: Real-time search and chat updates
**Features**:
- Live search results streaming
- Real-time AI chat responses
- Progress notifications
- Connection management

**WebSocket Example**:
```javascript
const ws = new WebSocket('ws://localhost:8090/ws');

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  switch(data.type) {
    case 'search_results':
      console.log('Live search results:', data.results);
      break;
    case 'chat_response':
      console.log('AI response:', data.response);
      break;
  }
}
```

---

## 🛠️ Debugging & Troubleshooting

### [Common Issues](../troubleshooting/)
**Purpose**: Debug and resolve common integration issues

**Connection Issues**:
```bash
# Test API connectivity
curl -I http://localhost:8090/health

# Check WebSocket connection
wscat -c ws://localhost:8090/ws

# Debug authentication
terraphim-agent auth test --verbose
```

**Performance Issues**:
```bash
# Profile search performance
terraphim-agent search --profile "test query" --debug

# Monitor resource usage
terraphim-agent monitor --resources --duration 60

# Cache analysis
terraphim-agent cache stats --verbose
```

---

## 🚀 Advanced Tutorials

### [Custom Workflow Creation](../workflows/)
**Purpose**: Create custom automated workflows
**Tutorial Path**:
1. [Basic Workflow](agent-workflows/1-prompt-chaining/) - Learn fundamentals
2. [Multi-Source Search](agent-workflows/3-parallelization/) - Add complexity
3. [AI Integration](agent-workflows/2-routing/) - Add intelligence
4. [Advanced Orchestration](agent-workflows/5-evaluator-optimizer/) - Production workflows

### [Domain-Specific Solutions](../domains/)
**Purpose**: Industry-specific configurations and examples
**Available Domains**:
- **Software Development**: Code analysis, documentation search
- **Research**: Literature review, concept discovery
- **Legal**: Case law search, precedent analysis
- **Healthcare**: Medical literature, clinical decision support
- **Finance**: Regulation analysis, market research

### [Performance Optimization](../performance/)
**Purpose**: Optimize Terraphim for specific use cases
**Topics**:
- Large dataset handling
- High-query throughput
- Low-latency requirements
- Memory-constrained environments

---

## 📚 Learning Resources

### [Video Tutorials](https://www.youtube.com/@terraphim)
- **Getting Started**: Complete installation and setup walkthrough
- **Advanced Features**: Deep dive into AI capabilities
- **Integration Examples**: Real-world integration scenarios
- **Performance Tuning**: Optimization techniques and best practices

### [Community Examples](https://github.com/terraphim/terraphim-ai/discussions/)
- **User Submissions**: Community-contributed examples and workflows
- **Use Cases**: Real-world deployment scenarios
- **Troubleshooting**: Community solutions and workarounds
- **Feature Requests**: Discussion and voting on new features

---

## 🎯 Choose Your Path

### 🔬 For Beginners
1. Start with [Quick Start Tutorial](agent-workflows/1-prompt-chaining/)
2. Try [Basic Search Examples](#basic-search)
3. Explore [First AI Chat](#basic-ai-chat)

### 👨‍💻 For Developers
1. Set up [IDE Integration](claude-code-hooks/)
2. Explore [API Examples](#api-examples)
3. Build [Web Integration](#web-development)

### 🔬 For Advanced Users
1. Master [Advanced Search](#advanced-search)
2. Create [Automation Workflows](#automation)
3. Optimize [Performance](#performance)

### 🏢 For Enterprise
1. Deploy [Team Integration](#team-integration)
2. Configure [Security Setup](#security-setup)
3. Implement [Monitoring](#monitoring)

---

## 🔗 Getting Help

### Documentation
- [Full Documentation](https://docs.terraphim.ai) - Complete guides and API reference
- [API Reference](../terraphim_server/) - Detailed API documentation
- [Architecture Guide](../docs/developer-guide/architecture.md) - System design and components

### Community Support
- **Discord**: [Join our Community](https://discord.gg/VPJXB6BGuY) for real-time help
- **GitHub Discussions**: [Start a Discussion](https://github.com/terraphim/terraphim-ai/discussions) for detailed questions
- **Issues**: [Report an Issue](https://github.com/terraphim/terraphim-ai/issues) for bug reports and feature requests

### Professional Support
- **Enterprise Support**: Contact support@terraphim.ai for business-critical issues
- **Documentation Issues**: Report documentation problems at docs@terraphim.ai
- **Security Issues**: Report security vulnerabilities at security@terraphim.ai

---

**🚀 Start building with Terraphim AI today!**

Whether you're a beginner looking to explore AI-powered search, a developer integrating advanced capabilities, or an enterprise deploying at scale, these examples provide the foundation for your success.

---

*Last Updated: December 20, 2025*
*Version: Terraphim AI v1.3.0*
*Part of: Terraphim AI Documentation Suite*