# crates/terraphim_multi_agent

## Purpose
Main agent implementation with command processing, LLM integration, token tracking, and multi-agent workflows.

## Status: Production-Ready
- ~8,445 LOC (non-test)
- 63/63 tests passing (100%)

## Key Types

### TerraphimAgent
Main agent wrapping Role configs.
- `new(role_config, persistence, agent_config)` - Create agent
- `initialize()` - Initialize agent state
- `process_command(CommandInput) -> CommandOutput` - Main processing entry point
- `get_enriched_context_for_query(query)` - Knowledge graph context enrichment
- `get_capabilities()` - List agent capabilities

Fields:
- `memory: Arc<RwLock<VersionedMemory>>` - Short/long-term memory
- `tasks: Arc<RwLock<VersionedTaskList>>` - Task tracking
- `lessons: Arc<RwLock<VersionedLessons>>` - Lessons learned
- `goals: AgentGoals` - Goal alignment
- `llm_client: Arc<GenAiLlmClient>` - LLM integration
- `vm_execution_client: Option<Arc<VmExecutionClient>>` - Firecracker VMs

### CommandInput / CommandOutput
Structured command processing with 9 command types:
Generate, Answer, Search, Analyze, Execute, Create, Edit, Review, Plan, System, Custom

### process_command() Flow
1. Status check (must be Ready)
2. Sets status to Busy
3. Creates CommandRecord with context snapshot
4. Routes to handler based on CommandType
5. Handler: extracts KG context, builds LLM messages, calls generate()
6. On success: updates context, records history
7. On error: records error, sets Error status
8. Sets status back to Ready

### GenAiLlmClient
rust-genai based LLM integration supporting Ollama and OpenRouter.

### AgentPool / AgentPoolManager
Agent pooling with reuse for multi-agent scenarios.

### AgentRegistry
Agent discovery and capability matching.

### Workflow Patterns
- RoleChaining, RoleRouting, RoleParallelization
- LeadWithSpecialists, RoleWithReview

## Integration Points
- Depends on terraphim_agent_evolution for memory/tasks/lessons
- Depends on terraphim_service for LLM providers
- Depends on terraphim_rolegraph for knowledge graph
- Optional terraphim_firecracker for VM execution

## Relevance to TinyClaw Rebuild
Maps to PicoClaw's AgentLoop but with much richer capabilities. The process_command() method needs adaptation to work as a chat-style message handler rather than command processor. Key gap: no tool-calling loop -- PicoClaw's iterative LLM->tool->LLM pattern needs to be built on top of this.
