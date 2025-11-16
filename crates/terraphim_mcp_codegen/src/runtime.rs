//! MCP Runtime Bridge - Enables code execution environment to call MCP tools
//!
//! This module provides the runtime infrastructure that allows code generated
//! by the TypeScript/Python generators to actually call MCP tools.

use std::sync::Arc;

use crate::{CodegenError, Result};

/// Configuration for the MCP runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// MCP server URL for HTTP transport
    pub mcp_server_url: Option<String>,
    /// Whether to use stdio transport
    pub use_stdio: bool,
    /// Timeout for MCP calls in milliseconds
    pub timeout_ms: u64,
    /// Maximum concurrent calls
    pub max_concurrent: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mcp_server_url: None,
            use_stdio: true,
            timeout_ms: 30000,
            max_concurrent: 10,
        }
    }
}

/// MCP Runtime that bridges code execution to MCP servers
pub struct McpRuntime {
    config: RuntimeConfig,
}

impl McpRuntime {
    /// Create a new MCP runtime
    pub fn new(config: RuntimeConfig) -> Self {
        Self { config }
    }

    /// Generate JavaScript runtime code that injects the mcpCall function
    pub fn generate_javascript_runtime(&self) -> String {
        let server_url = self
            .config
            .mcp_server_url
            .as_deref()
            .unwrap_or("http://localhost:3001");

        format!(
            r#"
// MCP Runtime Bridge for JavaScript/TypeScript
// This provides the mcpCall function that generated code uses

const MCP_SERVER_URL = "{}";
const MCP_TIMEOUT_MS = {};

async function mcpCall(toolName, params) {{
  const response = await fetch(`${{MCP_SERVER_URL}}/mcp/tools/call`, {{
    method: 'POST',
    headers: {{
      'Content-Type': 'application/json',
    }},
    body: JSON.stringify({{
      jsonrpc: '2.0',
      id: Date.now(),
      method: 'tools/call',
      params: {{
        name: toolName,
        arguments: params
      }}
    }}),
    signal: AbortSignal.timeout(MCP_TIMEOUT_MS)
  }});

  if (!response.ok) {{
    throw new Error(`MCP call failed: ${{response.statusText}}`);
  }}

  const result = await response.json();

  if (result.error) {{
    throw new Error(`MCP tool error: ${{result.error.message}}`);
  }}

  return result.result;
}}

// Make mcpCall available globally
globalThis.mcpCall = mcpCall;
"#,
            server_url, self.config.timeout_ms
        )
    }

    /// Generate Python runtime code that injects the mcp_call function
    pub fn generate_python_runtime(&self) -> String {
        let server_url = self
            .config
            .mcp_server_url
            .as_deref()
            .unwrap_or("http://localhost:3001");

        format!(
            r#"
# MCP Runtime Bridge for Python
# This provides the mcp_call function that generated code uses

import aiohttp
import json
from typing import Any, Dict

MCP_SERVER_URL = "{}"
MCP_TIMEOUT_MS = {}

async def mcp_call(tool_name: str, params: Dict[str, Any]) -> Dict[str, Any]:
    """
    Call an MCP tool through the MCP server.

    Args:
        tool_name: Name of the tool to call
        params: Parameters to pass to the tool

    Returns:
        The result from the MCP server

    Raises:
        Exception: If the MCP call fails
    """
    async with aiohttp.ClientSession() as session:
        payload = {{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {{
                "name": tool_name,
                "arguments": params
            }}
        }}

        timeout = aiohttp.ClientTimeout(total=MCP_TIMEOUT_MS / 1000)

        async with session.post(
            f"{{MCP_SERVER_URL}}/mcp/tools/call",
            json=payload,
            timeout=timeout
        ) as response:
            if not response.ok:
                raise Exception(f"MCP call failed: {{response.status}}")

            result = await response.json()

            if "error" in result:
                raise Exception(f"MCP tool error: {{result['error']['message']}}")

            return result.get("result", {{}})

# Inject into module namespace
import sys
current_module = sys.modules[__name__]
current_module.mcp_call = mcp_call
"#,
            server_url, self.config.timeout_ms
        )
    }

    /// Write JavaScript runtime to a file
    pub fn write_javascript_runtime(&self, path: &std::path::Path) -> Result<()> {
        let runtime_code = self.generate_javascript_runtime();
        std::fs::write(path, runtime_code)?;
        Ok(())
    }

    /// Write Python runtime to a file
    pub fn write_python_runtime(&self, path: &std::path::Path) -> Result<()> {
        let runtime_code = self.generate_python_runtime();
        std::fs::write(path, runtime_code)?;
        Ok(())
    }

    /// Setup runtime in a VM environment
    pub async fn setup_vm_environment(&self, workspace_path: &std::path::Path) -> Result<()> {
        // Create workspace directories
        std::fs::create_dir_all(workspace_path.join("mcp-runtime"))?;

        // Write JavaScript runtime
        self.write_javascript_runtime(&workspace_path.join("mcp-runtime/runtime.js"))?;

        // Write Python runtime
        self.write_python_runtime(&workspace_path.join("mcp-runtime/runtime.py"))?;

        // Write package.json for Node.js
        let package_json = serde_json::json!({
            "name": "mcp-runtime",
            "version": "1.0.0",
            "type": "module",
            "main": "runtime.js"
        });
        std::fs::write(
            workspace_path.join("mcp-runtime/package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        Ok(())
    }
}

/// Builder for creating complete code execution packages
pub struct CodeExecutionPackage {
    /// Generated wrapper code (TypeScript or Python)
    pub wrapper_code: String,
    /// Runtime bridge code
    pub runtime_code: String,
    /// Configuration
    pub config: RuntimeConfig,
}

impl CodeExecutionPackage {
    /// Create a new code execution package for TypeScript
    pub fn typescript(wrapper_code: String, config: RuntimeConfig) -> Self {
        let runtime = McpRuntime::new(config.clone());
        Self {
            wrapper_code,
            runtime_code: runtime.generate_javascript_runtime(),
            config,
        }
    }

    /// Create a new code execution package for Python
    pub fn python(wrapper_code: String, config: RuntimeConfig) -> Self {
        let runtime = McpRuntime::new(config.clone());
        Self {
            wrapper_code,
            runtime_code: runtime.generate_python_runtime(),
            config,
        }
    }

    /// Write the complete package to a directory
    pub fn write_to_directory(&self, dir: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;

        // Determine file extensions based on content
        let (wrapper_name, runtime_name) = if self.wrapper_code.contains("export async function") {
            ("terraphim.ts", "runtime.js")
        } else {
            ("terraphim.py", "runtime.py")
        };

        std::fs::write(dir.join(wrapper_name), &self.wrapper_code)?;
        std::fs::write(dir.join(runtime_name), &self.runtime_code)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_javascript_runtime() {
        let config = RuntimeConfig {
            mcp_server_url: Some("http://localhost:3001".to_string()),
            timeout_ms: 30000,
            ..Default::default()
        };

        let runtime = McpRuntime::new(config);
        let code = runtime.generate_javascript_runtime();

        assert!(code.contains("http://localhost:3001"));
        assert!(code.contains("async function mcpCall"));
        assert!(code.contains("globalThis.mcpCall"));
    }

    #[test]
    fn test_generate_python_runtime() {
        let config = RuntimeConfig {
            mcp_server_url: Some("http://localhost:3001".to_string()),
            timeout_ms: 30000,
            ..Default::default()
        };

        let runtime = McpRuntime::new(config);
        let code = runtime.generate_python_runtime();

        assert!(code.contains("http://localhost:3001"));
        assert!(code.contains("async def mcp_call"));
        assert!(code.contains("aiohttp"));
    }

    #[test]
    fn test_code_execution_package() {
        let wrapper = "export async function search() {}".to_string();
        let config = RuntimeConfig::default();

        let package = CodeExecutionPackage::typescript(wrapper, config);

        assert!(package.wrapper_code.contains("export async"));
        assert!(package.runtime_code.contains("mcpCall"));
    }
}
