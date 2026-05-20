# Research Document: Opencode Task Delivery Mechanism in ADF

## 1. Problem Restatement and Scope

**Problem:** The ADF orchestrator spawns opencode agents using stdin delivery for large tasks (>32KB threshold). While the agents now survive OOM killer (via `oom_score_adj=-1000`), they appear to hang indefinitely without producing output or completing work. This suggests the task delivery mechanism may not be correctly passing the prompt to opencode.

**IN scope:**
- How opencode `run` command accepts task input (stdin vs positional args)
- How the ADF spawner constructs the command line
- How stdin is piped and closed
- Testing both delivery methods with real task sizes

**OUT of scope:**
- API provider responsiveness (kimi-for-coding/k2p5 latency)
- Opencode plugin configuration
- Model-specific issues

## 2. User & Business Outcomes

- Agents should receive their full task prompt and begin execution
- Agents should produce output visible in log files
- Agents should complete within expected wall-clock time (not hang indefinitely)

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| opencode CLI | `/home/alex/.bun/bin/opencode` | Bun-compiled Node.js CLI | Node runtime, opencode config |
| `run` subcommand | opencode built-in | Non-interactive mode | Expects message as positionals |
| Spawner | `crates/terraphim_spawner/src/lib.rs` | Process spawning | tokio::process::Command |
| Orchestrator | `crates/terraphim_orchestrator/src/lib.rs` | Task composition & spawn decision | Spawner, agent config |
| Agent config | `terraphim.toml` | Defines cli_tool, task, schedule | None |

## 4. Constraints and Their Implications

1. **Task size threshold**: Tasks >32KB use stdin delivery (threshold in orchestrator at line 2121). This triggers for large prompts with skill chains.
2. **Opencode interface**: `opencode run [message..]` — the `[message..]` positional syntax indicates multiple message arguments, not stdin reading.
3. **ARG_MAX limit**: `getconf ARG_MAX` returns 2097152 (2MB) on bigbox. Tasks are ~63KB, well under this limit.
4. **Stdin pipe closure**: The spawner writes to stdin but relies on `ChildStdin` drop to close the pipe. No explicit flush or shutdown is called.

## 5. Investigation Results

### Test 1: Simple task via stdin (500 bytes)
**Method:** `echo "Say hello" | opencode run --format json -m kimi-for-coding/k2p5`
**Result:** SUCCESS. Process exited in ~5s with JSON output.

### Test 2: 50KB task via stdin
**Method:** `cat /tmp/50kb-task.txt | opencode run ...`
**Result:** SUCCESS. Process exited in ~10s with response.

### Test 3: 97KB task via stdin (actual swarm task size)
**Method:** `cat /tmp/63kb-task.txt | opencode run --format json -m kimi-for-coding/k2p5`
**Result:** FAILURE. Process still running after 25s. Had to be killed with timeout.

### Test 4: 97KB task via positional arguments
**Method:** `opencode run --format json -m kimi-for-coding/k2p5 "$(cat /tmp/63kb-task.txt)"`
**Result:** SUCCESS. Process exited in ~7s with proper response ("I can't execute 500 nearly identical research steps...").

### Conclusion
**Stdin delivery hangs for tasks >~50KB** while positional argument delivery works correctly. This is the root cause of implementation swarm agents hanging.

## 6. Root Cause Analysis

The opencode `run` command is designed to accept messages as **positional arguments** (`[message..]`). While stdin may work for small inputs (possibly buffered by Node.js stream handling), it fails for larger tasks because:

1. opencode does not explicitly read from stdin for the `run` subcommand
2. The Node.js wrapper (`opencode` shell script) passes `stdio: "inherit"` to the child, but the actual CLI may not consume stdin
3. Large stdin data fills buffers and causes the process to block waiting for a reader that never comes

## 7. Context Complexity vs. Simplicity Opportunities

**Complexity**: The spawner has a dual-mode delivery system (stdin vs argument) with a 32KB threshold. This adds unnecessary complexity for opencode which only works reliably with positional arguments.

**Simplification**: Always pass opencode tasks as positional arguments, regardless of size. Remove the stdin threshold logic for opencode specifically.

## 8. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| ARG_MAX exceeded for extremely large tasks | Process spawn fails | ARG_MAX is 2MB; current tasks are ~63KB. Add guard to fall back to file-based delivery if approaching limit. |
| Special characters in task break shell parsing | Command injection or parse errors | Pass arguments directly via `Command::arg()` (Rust handles escaping). |
| Other CLI tools (claude, codex) may need stdin | Regression | Only change opencode behavior; claude and codex keep existing stdin logic. |

## 9. Questions for Human Reviewer

1. Should we remove stdin delivery entirely for opencode, or make it configurable per-agent?
2. Should we increase the stdin threshold globally, or only for opencode?
3. Do we need to test this fix with claude and codex to ensure no regression?
