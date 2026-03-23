# crates/terraphim_agent_supervisor

## Purpose
OTP-style supervision trees for agent lifecycle management with automatic restart strategies.

## Status: Production-Ready
- ~1,452 LOC (non-test)
- 16/16 tests passing (100%)

## Key Types

### AgentSupervisor
OTP-style supervision tree manager.
- `spawn_agent()` - Start new supervised agents
- `stop_agent()` - Graceful shutdown with timeout
- `handle_agent_exit()` - Automatic restart on failure

### RestartStrategy (enum)
- `OneForOne` - Restart only failed agent
- `OneForAll` - Restart all agents if one fails
- `RestForOne` - Restart failed agent and all started after it

### RestartPolicy
Combines strategy with intensity limits (max restarts within time window).

### RestartIntensity
Default: 5 restarts in 60 seconds. Prevents restart storms.

### SupervisedAgent (trait)
Agent lifecycle interface for supervised agents.

### AgentFactory (trait)
Agent creation abstraction for dynamic agent spawning.

### SupervisedAgentInfo
Tracking restart counts and timestamps per agent.

### SupervisorConfig
Configuration: restart policy, timeouts, max children, health check intervals.

## Integration Points
- Used by terraphim_multi_agent for agent pool management
- Health check background task with configurable intervals
- Hierarchical supervision support (supervisor trees)

## Relevance to TinyClaw Rebuild
PicoClaw has no supervision -- crashed channels stay crashed. This crate provides automatic fault recovery for channel adapters (if Telegram connection drops, supervisor restarts it).
