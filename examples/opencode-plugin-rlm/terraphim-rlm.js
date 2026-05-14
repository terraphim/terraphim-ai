/**
 * @file terraphim-rlm.js
 * OpenCode plugin: RLM (Recursive Language Model) orchestration for secure code execution.
 *
 * This plugin exposes RLM capabilities to OpenCode agents:
 * - Execute Python code in isolated environments
 * - Run bash commands in VMs
 * - Recursive query loops with LLM feedback
 * - Session management with snapshots
 *
 * Install:
 *   cp terraphim-rlm.js ~/.config/opencode/plugin/
 *   # or ~/.config/opencode/plugins/ depending on your OpenCode version
 *
 * Requirements:
 *   - terraphim_mcp_server running with RLM tools, OR
 *   - terraphim_rlm crate built and available
 */

const { spawn } = require("child_process");
const path = require("path");

const TERRAPHIM_AGENT = process.env.TERRAPHIM_AGENT ||
  path.join(process.env.HOME || "/", ".cargo", "bin", "terraphim-agent");

const TERRAPHIM_MCP = process.env.TERRAPHIM_MCP ||
  "terraphim_mcp_server";

const RLM_SESSION_TIMEOUT_MS = 300000;

const TerraphimRlmPlugin = async ({ project, client, $, directory, worktree }) => {
  let mcpProcess = null;
  let currentSessionId = null;

  const ensureMcpServer = async () => {
    if (mcpProcess) return true;

    return new Promise((resolve) => {
      mcpProcess = spawn(TERRAPHIM_MCP, ["--profile", "desktop"], {
        stdio: ["pipe", "pipe", "inherit"],
        env: { ...process.env },
      });

      mcpProcess.on("error", () => {
        mcpProcess = null;
        resolve(false);
      });

      mcpProcess.on("close", () => {
        mcpProcess = null;
      });

      setTimeout(() => resolve(true), 1000);
    });
  };

  const callMcpTool = async (tool, args) => {
    const available = await ensureMcpServer();
    if (!available) {
      throw new Error("RLM MCP server not available. Install terraphim_mcp_server.");
    }

    return new Promise((resolve, reject) => {
      const request = JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "tools/call",
        params: {
          name: tool,
          arguments: args,
        },
      });

      let responseData = "";

      const proc = spawn(TERRAPHIM_MCP, [], {
        stdio: ["pipe", "pipe", "pipe"],
      });

      proc.stdout.on("data", (data) => {
        responseData += data.toString();
        try {
          const response = JSON.parse(responseData);
          if (response.result) {
            resolve(response.result);
            proc.kill();
          }
        } catch {}
      });

      proc.stderr.on("data", (data) => {
        console.error("MCP stderr:", data.toString());
      });

      proc.on("error", reject);
      proc.on("close", () => resolve(null));

      proc.stdin.write(request + "\n");
      proc.stdin.end();
    });
  };

  return {
    name: "terraphim-rlm",
    version: "1.0.0",

    "tool.execute.before": async (input, output) => {
      if (input.tool !== "Bash" || !input.args?.command) return;

      const command = input.args.command;
      const rlmPattern = /^\s*(rlm|rlm_code|rlm_bash|rlm_query)\b/i;

      if (!rlmPattern.test(command)) return;

      const parts = command.trim().split(/\s+/);
      const rlmCmd = parts[0].toLowerCase();

      if (rlmCmd === "rlm" || rlmCmd === "rlm_query") {
        const prompt = parts.slice(1).join(" ") || "Hello";
        try {
          const result = await callMcpTool("rlm_query", { prompt });
          if (result && result.content) {
            output.result = result.content[0]?.text || result.content;
          }
        } catch (e) {
          output.error = e.message;
        }
      } else if (rlmCmd === "rlm_code") {
        const code = parts.slice(1).join(" ");
        try {
          const result = await callMcpTool("rlm_code", { code });
          if (result) {
            output.result = result.stdout || JSON.stringify(result);
          }
        } catch (e) {
          output.error = e.message;
        }
      } else if (rlmCmd === "rlm_bash") {
        const bashCmd = parts.slice(1).join(" ");
        try {
          const result = await callMcpTool("rlm_bash", { command: bashCmd });
          if (result) {
            output.result = result.stdout || JSON.stringify(result);
          }
        } catch (e) {
          output.error = e.message;
        }
      }
    },

    "session.start": async (session) => {
      try {
        await ensureMcpServer();
        const result = await callMcpTool("rlm_status", {});
        if (result && result.session_id) {
          currentSessionId = result.session_id;
        }
      } catch {}
    },

    "session.end": async (session) => {
      currentSessionId = null;
    },

    cleanup: async () => {
      if (mcpProcess) {
        mcpProcess.kill();
        mcpProcess = null;
      }
    },
  };
};

module.exports = TerraphimRlmPlugin;
module.exports.default = TerraphimRlmPlugin;
