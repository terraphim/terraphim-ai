# Code Execution with MCP - Gap Analysis for Terraphim AI

**Version:** 1.0
**Date:** 2025-11-15
**Status:** Planning

## Executive Summary

This document analyzes Terraphim AI's current capabilities against the requirements for implementing Anthropic's Code Execution with MCP approach. Overall assessment: **60% capability exists**, requiring targeted development in specific areas.

## Current Capabilities ✅

### 1. Secure Code Execution Environment
**Status:** ✅ **COMPLETE**

Terraphim AI has a fully functional VM execution system:

- **Firecracker VMs:** Sub-2 second boot times
- **VM Pooling:** Efficient resource management
- **Multiple Languages:** Python, JavaScript, Bash, Rust
- **Security:** Sandboxed execution with resource limits
- **Monitoring:** Execution metrics and error tracking

**Location:** `crates/terraphim_multi_agent/src/vm_execution/`

**Evidence:**
```rust
// crates/terraphim_multi_agent/src/vm_execution/models.rs
pub struct VmExecutionConfig {
    pub enabled: bool,
    pub api_base_url: String,
    pub vm_pool_size: u32,
    pub default_vm_type: String,
    pub execution_timeout_ms: u64,
    pub allowed_languages: Vec<String>,
    // ... more configuration
}
```

**Capabilities:**
- Execute code in isolated VMs
- Timeout enforcement
- Resource limits (CPU, memory, disk)
- Command history tracking
- Snapshot and rollback support
- Code validation before execution

### 2. MCP Server Implementation
**Status:** ✅ **COMPLETE**

Comprehensive MCP server with 17 tools:

- **Location:** `crates/terraphim_mcp_server/`
- **Tools Available:**
  - search
  - autocomplete_terms
  - autocomplete_with_snippets
  - fuzzy_autocomplete_search
  - find_matches
  - replace_matches
  - extract_paragraphs_from_automata
  - load_thesaurus
  - build_autocomplete_index
  - serialize/deserialize_autocomplete_index
  - is_all_terms_connected_by_path
  - json_decode
  - update_config_tool

**Evidence:**
```rust
// crates/terraphim_mcp_server/src/lib.rs
impl ServerHandler for McpService {
    async fn list_tools(...) -> Result<ListToolsResult, ErrorData> {
        // 17 tools exposed via MCP protocol
    }
}
```

### 3. Agent System
**Status:** ✅ **COMPLETE**

Multi-agent system with lifecycle management:

- **Supervisor:** `crates/terraphim_agent_supervisor/`
- **Multi-agent:** `crates/terraphim_multi_agent/`
- **Registry:** `crates/terraphim_agent_registry/`
- **Messaging:** `crates/terraphim_agent_messaging/`

**Capabilities:**
- Agent lifecycle (init, start, stop, terminate)
- Health checks and monitoring
- Agent supervision and restart policies
- Inter-agent communication
- Task decomposition
- Goal alignment

### 4. State Persistence
**Status:** ✅ **COMPLETE**

Comprehensive state management:

- **Location:** `crates/terraphim_persistence/`
- **Backends:** Memory, DashMap, SQLite, Redb
- **Features:**
  - Document storage
  - Configuration persistence
  - State snapshots
  - Versioned memory (agent evolution)

### 5. Code Extraction
**Status:** ✅ **COMPLETE**

Extract code blocks from LLM responses:

- **Location:** `crates/terraphim_multi_agent/src/vm_execution/code_extractor.rs`
- **Features:**
  - Markdown code block parsing
  - Language detection
  - Execution intent detection
  - Confidence scoring

## Missing Capabilities ❌

### 1. MCP Servers as Code APIs
**Status:** ❌ **NOT IMPLEMENTED**

**Current State:**
- MCP tools exposed only via MCP protocol
- Direct tool calling through request/response
- No programmatic import interface

**Required:**
```typescript
// Need to support this:
import { terraphim } from 'mcp-servers';

const results = await terraphim.search({
  query: "rust async patterns",
  limit: 10
});

const filtered = results.filter(doc => doc.rank > 0.8);
```

**Gap:**
- No TypeScript/Python module wrappers for MCP tools
- No import mechanism in code execution environment
- No module discovery API

**Location to Implement:**
- New crate: `crates/terraphim_mcp_codegen/`
- Modify: `crates/terraphim_multi_agent/src/vm_execution/`

### 2. Progressive Tool Discovery
**Status:** ❌ **NOT IMPLEMENTED**

**Current State:**
- All tools listed via `list_tools()`
- No search or filtering
- No dynamic documentation

**Required:**
```typescript
// Need to support:
import { searchTools, getToolDocs } from 'mcp-runtime';

const tools = await searchTools({
  category: 'knowledge-graph',
  capabilities: ['search', 'autocomplete']
});

const docs = await getToolDocs('terraphim.search');
```

**Gap:**
- No tool categorization system
- No tool search functionality
- No dynamic documentation generation
- No capability-based filtering

**Location to Implement:**
- New module: `crates/terraphim_mcp_server/src/discovery.rs`
- Update: `crates/terraphim_mcp_server/src/lib.rs`

### 3. In-Environment Data Processing
**Status:** ⚠️ **PARTIAL**

**Current State:**
- VM execution runs code
- But MCP tools not accessible within VM
- Results still pass through context

**Required:**
- MCP tools callable from within VM environment
- Data processing happens in VM before returning
- Only final results exit to agent

**Gap:**
- No MCP runtime in VM environment
- No bridge between VM execution and MCP tools
- No in-VM data transformation utilities

**Location to Implement:**
- New module: `crates/terraphim_multi_agent/src/vm_execution/mcp_runtime.rs`
- Update: `crates/terraphim_multi_agent/src/vm_execution/client.rs`

### 4. Skill Library System
**Status:** ❌ **NOT IMPLEMENTED**

**Current State:**
- No skill storage mechanism
- No SKILL.MD pattern
- No reusable function library

**Required:**
```markdown
# SKILL.MD: Knowledge Graph Analysis

## Function
async function analyzeConnectivity(text: string): Promise<Analysis> {
  // Reusable skill implementation
}

## Usage History
- Success rate: 95%
- Average execution time: 1.2s
```

**Gap:**
- No skill storage directory structure
- No skill discovery/search
- No usage tracking
- No skill versioning

**Location to Implement:**
- New crate: `crates/terraphim_skills/`
- Directory: `skills/` in workspace root

### 5. Agent Code Generation Optimization
**Status:** ⚠️ **PARTIAL**

**Current State:**
- Agents use LLM for responses
- Code extraction exists
- But not optimized for code-first approach

**Required:**
- Agents preferentially generate code over tool calls
- Code generation prompts optimized
- Import-based tool usage
- Error handling in code

**Gap:**
- No code-first prompt templates
- No examples of MCP tool imports in prompts
- No code quality feedback loop

**Location to Implement:**
- Update: `crates/terraphim_multi_agent/src/agent.rs`
- New: `crates/terraphim_multi_agent/src/prompts/code_execution.rs`

### 6. Token Usage Optimization
**Status:** ⚠️ **PARTIAL**

**Current State:**
- Token tracking exists
- Cost tracking exists
- But not optimized for code execution pattern

**Required:**
- Track token savings from code execution
- Compare traditional vs code approach
- Metrics dashboard

**Gap:**
- No comparison metrics
- No optimization recommendations
- No A/B testing framework

**Location to Implement:**
- Update: `crates/terraphim_multi_agent/src/agent.rs`
- New: `crates/terraphim_multi_agent/src/metrics/code_execution.rs`

### 7. Workspace Management
**Status:** ⚠️ **PARTIAL**

**Current State:**
- VM execution has temporary storage
- But no structured workspace

**Required:**
```
workspace/
  ├── data/           # Temporary data files
  ├── results/        # Execution results
  ├── checkpoints/    # Saved state snapshots
  └── skills/         # Reusable skill library
```

**Gap:**
- No workspace directory structure
- No file management utilities
- No cleanup policies

**Location to Implement:**
- New module: `crates/terraphim_multi_agent/src/workspace.rs`

## Capability Matrix

| Requirement | Status | Priority | Effort | Notes |
|------------|--------|----------|--------|-------|
| Secure Code Execution | ✅ Complete | - | - | Firecracker VMs ready |
| MCP Server | ✅ Complete | - | - | 17 tools available |
| Agent System | ✅ Complete | - | - | Full lifecycle management |
| State Persistence | ✅ Complete | - | - | Multiple backends |
| Code Extraction | ✅ Complete | - | - | Parse markdown blocks |
| **MCP Code APIs** | ❌ Missing | **Critical** | **High** | Core requirement |
| **Progressive Discovery** | ❌ Missing | **High** | **Medium** | Scalability essential |
| **In-Environment Processing** | ⚠️ Partial | **Critical** | **High** | Token reduction key |
| **Skill Library** | ❌ Missing | **Medium** | **Medium** | Reusability benefit |
| **Code-First Prompts** | ⚠️ Partial | **High** | **Low** | Quick win |
| **Token Optimization** | ⚠️ Partial | **Medium** | **Low** | Metrics important |
| **Workspace Management** | ⚠️ Partial | **Low** | **Low** | Nice to have |

## Summary Statistics

- **Complete:** 5/12 (42%)
- **Partial:** 4/12 (33%)
- **Missing:** 3/12 (25%)
- **Overall Readiness:** ~60%

## Critical Path Items

To achieve minimum viable implementation:

1. **MCP Code APIs** (Critical, High Effort)
   - Convert MCP tools to importable modules
   - Create runtime environment in VMs
   - Enable code-based tool usage

2. **In-Environment Processing** (Critical, High Effort)
   - Bridge MCP tools to VM execution
   - Process data within VM
   - Return only final results

3. **Code-First Prompts** (High, Low Effort)
   - Update agent prompts
   - Add code examples
   - Optimize for imports

## Recommended Implementation Order

### Phase 1: Foundation (4 weeks)
**Goal:** Basic code execution with MCP tools

1. Create MCP code API layer
   - TypeScript/Python wrappers
   - Import mechanism
   - Runtime in VM

2. Update code-first prompts
   - Add import examples
   - Optimize for code generation
   - Test with existing agents

3. Implement in-environment processing
   - MCP bridge to VM
   - Data transformation utilities
   - Result minimization

**Success Criteria:**
- Agents can import and use MCP tools in code
- Basic workflow achieves >80% token reduction
- Code execution completes in <3 seconds

### Phase 2: Discovery & Scale (4 weeks)
**Goal:** Support many tools efficiently

1. Progressive tool discovery
   - Tool search API
   - Categorization system
   - Dynamic documentation

2. Workspace management
   - Structured directories
   - File utilities
   - Cleanup policies

3. Token optimization metrics
   - Comparison tracking
   - Dashboard creation
   - Optimization recommendations

**Success Criteria:**
- Tool discovery <100ms
- Support 100+ tools
- Token reduction metrics visible

### Phase 3: Skills & Optimization (4 weeks)
**Goal:** Production-ready features

1. Skill library system
   - Storage structure
   - SKILL.MD format
   - Discovery and search
   - Usage tracking

2. Performance optimization
   - Caching and memoization
   - Resource pooling
   - Load testing

3. Production hardening
   - Monitoring dashboards
   - Error handling
   - Documentation

**Success Criteria:**
- Skills reusable across agents
- 98%+ token reduction achieved
- Production deployment ready

## Next Steps

1. **Review and approve** this gap analysis
2. **Prioritize** critical path items
3. **Create detailed tasks** for Phase 1
4. **Assign resources** and timeline
5. **Begin implementation** of MCP Code APIs

## Appendices

### A. Code API Example

Current MCP tool call:
```json
{
  "tool": "search",
  "arguments": {
    "query": "rust async patterns",
    "limit": 10
  }
}
```

Desired code-based usage:
```typescript
import { terraphim } from 'mcp-servers';

async function analyzePatterns() {
  const docs = await terraphim.search({
    query: "rust async patterns",
    limit: 100
  });

  const highQuality = docs.filter(d => d.rank > 0.8);
  const byTopic = groupBy(highQuality, 'topic');

  return {
    total: highQuality.length,
    topics: Object.keys(byTopic),
    top_doc: highQuality[0]
  };
}
```

### B. Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│                  Agent Layer                     │
│  - Generates code instead of tool calls         │
│  - Optimized prompts for imports                │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│            Code Execution Layer                  │
│  ┌────────────────────────────────────────┐    │
│  │  Firecracker VM (existing)             │    │
│  │  ┌──────────────────────────────────┐  │    │
│  │  │  MCP Runtime (NEW)                │  │    │
│  │  │  - Import MCP tools as modules    │  │    │
│  │  │  - Process data in-environment    │  │    │
│  │  │  - Return minimal results         │  │    │
│  │  └──────────────────────────────────┘  │    │
│  └────────────────────────────────────────┘    │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│         MCP Code API Layer (NEW)                 │
│  - TypeScript/Python module wrappers            │
│  - Tool discovery API                           │
│  - Documentation generation                     │
└─────────────────┬───────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│      MCP Server (existing)                       │
│  - 17 knowledge graph tools                     │
│  - Autocomplete, search, analysis               │
└─────────────────────────────────────────────────┘
```

### C. Token Reduction Calculation

**Baseline Workflow (Traditional):**
```
1. Load all tool definitions: 17 tools × 800 tokens = 13,600 tokens
2. Call search: query (200) + results (8,000) = 8,200 tokens
3. Call autocomplete: query (200) + results (5,000) = 5,200 tokens
4. Call find_matches: query (300) + results (10,000) = 10,300 tokens
5. Agent processing and response: 2,000 tokens
Total: 39,300 tokens
```

**Code Execution Workflow:**
```
1. Agent generates code: 1,200 tokens
2. Code executes in VM:
   - Calls search (internal, no tokens)
   - Calls autocomplete (internal, no tokens)
   - Calls find_matches (internal, no tokens)
   - Processes data (internal, no tokens)
3. Final result returned: 800 tokens
Total: 2,000 tokens
```

**Reduction: 95% (39,300 → 2,000)**

### D. Implementation Checklist

**Phase 1: Foundation**
- [ ] Create `crates/terraphim_mcp_codegen/` crate
- [ ] Generate TypeScript wrappers for all 17 MCP tools
- [ ] Generate Python wrappers for all 17 MCP tools
- [ ] Implement MCP runtime in VM environment
- [ ] Add import mechanism to code execution
- [ ] Create code-first prompt templates
- [ ] Update agent code generation logic
- [ ] Implement MCP bridge in VM execution client
- [ ] Add data transformation utilities
- [ ] Test end-to-end workflow
- [ ] Measure token reduction
- [ ] Document new patterns

**Phase 2: Discovery & Scale**
- [ ] Implement tool search API
- [ ] Create tool categorization system
- [ ] Add capability-based filtering
- [ ] Generate dynamic documentation
- [ ] Create workspace directory structure
- [ ] Implement file management utilities
- [ ] Add cleanup policies
- [ ] Create token comparison metrics
- [ ] Build metrics dashboard
- [ ] Add optimization recommendations
- [ ] Load test with 100+ tools

**Phase 3: Skills & Optimization**
- [ ] Design skill storage structure
- [ ] Implement SKILL.MD format parser
- [ ] Create skill discovery/search
- [ ] Add usage tracking
- [ ] Implement skill versioning
- [ ] Add caching layer
- [ ] Implement memoization
- [ ] Optimize resource pooling
- [ ] Load test with 1000+ concurrent agents
- [ ] Create monitoring dashboards
- [ ] Write production documentation
- [ ] Security audit and hardening
