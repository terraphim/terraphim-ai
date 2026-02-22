//! WASM-based custom router implementation
//! 
//! This module implements Phase 2 of the 3-phase routing architecture,
//! allowing users to load custom routing logic as WASM modules.

use crate::{
    analyzer::RoutingHints,
    token_counter::ChatRequest,
    ProxyError, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// WASM router engine for executing custom routing logic
pub struct WasmRouter {
    engine: Engine,
    modules: Arc<RwLock<HashMap<String, CompiledModule>>>,
}

/// WASM routing function signature
/// 
/// The WASM function should have the signature:
/// ```wasm
/// (input_ptr: i32, input_len: i32) -> i32
/// ```
/// 
/// Where:
/// - input_ptr: Pointer to JSON-encoded RoutingInput in WASM memory
/// - input_len: Length of the JSON input
/// - Returns: Pointer to JSON-encoded RoutingOutput in WASM memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingInput {
    pub request: ChatRequest,
    pub hints: RoutingHints,
    pub available_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingOutput {
    pub provider: String,
    pub confidence: f64,
    pub reasoning: Option<String>,
}

impl WasmRouter {
    /// Create a new WASM router engine
    pub fn new() -> Result<Self> {
        // Configure Wasmtime engine for security and performance
        let mut config = Config::new();
        config.wasm_component_model(false);
        config.async_support(false);
        config.consume_fuel(true);
        config.fuel_consumption(true);
        
        // Security: Limit memory and execution
        config.max_wasm_stack(1024 * 1024); // 1MB stack
        config.consume_fuel(true); // Enable fuel metering
        
        let engine = Engine::new(&config)
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to create WASM engine: {}", e),
            })?;

        Ok(Self {
            engine,
            modules: Arc::new(RwLock::new(HashMap::new())),
            store_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Load a WASM module from file
    pub async fn load_module<P: AsRef<Path>>(&self, name: String, path: P) -> Result<()> {
        let path = path.as_ref();
        
        tracing::info!(name = %name, path = %path.display(), "Loading WASM routing module");

        // Read WASM file
        let wasm_bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to read WASM file: {}", e),
            })?;

        // Compile module
        let module = Module::from_binary(&self.engine, &wasm_bytes)
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to compile WASM module: {}", e),
            })?;

        // Store module
        let mut modules = self.modules.write().await;
        modules.insert(name.clone(), module);

        tracing::info!(name = %name, "WASM module loaded successfully");
        Ok(())
    }

    /// Load a WASM module from bytes
    pub async fn load_module_from_bytes(&self, name: String, wasm_bytes: Vec<u8>) -> Result<()> {
        tracing::info!(name = %name, size = wasm_bytes.len(), "Loading WASM module from bytes");

        // Compile module
        let module = Module::from_binary(&self.engine, &wasm_bytes)
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to compile WASM module: {}", e),
            })?;

        // Store module
        let mut modules = self.modules.write().await;
        modules.insert(name.clone(), module);

        tracing::info!(name = %name, "WASM module loaded successfully");
        Ok(())
    }

    /// Execute custom routing logic
    pub async fn route(
        &self,
        module_name: &str,
        request: &ChatRequest,
        hints: &RoutingHints,
        available_providers: &[String],
    ) -> Result<RoutingOutput> {
        let modules = self.modules.read().await;
        
        let module = modules.get(module_name).ok_or_else(|| ProxyError::ConfigurationError {
            message: format!("WASM module '{}' not found", module_name),
        })?;

        // Create WASI context for system access (limited)
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build()
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to create WASI context: {}", e),
            })?;

        let mut store = Store::new(&self.engine, wasi);
        
        // Configure fuel limits (5ms execution time)
        store.add_fuel(100_000) // Approximate fuel for 5ms
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to set fuel limit: {}", e),
            })?;

        // Create instance
        let instance = Instance::new(&mut store, module, &[])
            .map_err(|e| ProxyError::ProviderError {
                provider: "wasm_router".to_string(),
                message: format!("Failed to create WASM instance: {}", e),
            })?;

        // Get the main routing function
        let route_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "route")
            .map_err(|_| ProxyError::ConfigurationError {
                message: "WASM module must export a 'route' function with signature (i32, i32) -> i32".to_string(),
            })?;

        // Prepare input
        let input = RoutingInput {
            request: request.clone(),
            hints: hints.clone(),
            available_providers: available_providers.to_vec(),
        };

        let input_json = serde_json::to_string(&input)
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to serialize routing input: {}", e),
            })?;

        // Allocate memory in WASM and copy input
        let memory = instance
            .get_memory(&mut store, 0)
            .ok_or_else(|| ProxyError::ConfigurationError {
                message: "WASM module must export memory".to_string(),
            })?;

        let input_bytes = input_json.as_bytes();
        let input_ptr = self.allocate_memory(&mut store, memory, input_bytes.len())?;
        
        // Copy input to WASM memory
        let memory_data = memory.data_mut(&mut store);
        memory_data[input_ptr..input_ptr + input_bytes.len()].copy_from_slice(input_bytes);

        // Execute routing function
        let output_ptr = route_fn
            .call(&mut store, (input_ptr as i32, input_bytes.len() as i32))
            .map_err(|e| ProxyError::ProviderError {
                provider: "wasm_router".to_string(),
                message: format!("WASM routing function failed: {}", e),
            })?;

        // Read output from WASM memory
        let output_json = self.read_string(&mut store, memory, output_ptr as usize)?;

        // Parse output
        let output: RoutingOutput = serde_json::from_str(&output_json)
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Failed to parse routing output: {}", e),
            })?;

        // Validate output
        if !available_providers.contains(&output.provider) {
            return Err(ProxyError::ConfigurationError {
                message: format!("WASM router returned invalid provider: {}", output.provider),
            });
        }

        tracing::info!(
            module = %module_name,
            provider = %output.provider,
            confidence = %output.confidence,
            "WASM routing completed successfully"
        );

        Ok(output)
    }

    /// Allocate memory in WASM and return pointer
    fn allocate_memory(
        &self,
        store: &mut Store<WasiCtx>,
        memory: &Memory,
        size: usize,
    ) -> Result<usize> {
        // For now, use a simple allocation strategy
        // In a production implementation, you'd want proper memory management
        let memory_size = memory.size(store) * 65536; // Wasm pages are 64KB
        
        if size > memory_size {
            return Err(ProxyError::ConfigurationError {
                message: "Insufficient WASM memory".to_string(),
            });
        }

        // Allocate at the end of memory (simplified)
        Ok(memory_size - size)
    }

    /// Read null-terminated string from WASM memory
    fn read_string(
        &self,
        store: &mut Store<WasiCtx>,
        memory: &Memory,
        ptr: usize,
    ) -> Result<String> {
        let memory_data = memory.data(store);
        
        // Find null terminator
        let mut end = ptr;
        while end < memory_data.len() && memory_data[end] != 0 {
            end += 1;
        }

        if end >= memory_data.len() {
            return Err(ProxyError::ConfigurationError {
                message: "Invalid string pointer in WASM memory".to_string(),
            });
        }

        let string_bytes = &memory_data[ptr..end];
        String::from_utf8(string_bytes.to_vec())
            .map_err(|e| ProxyError::ConfigurationError {
                message: format!("Invalid UTF-8 in WASM output: {}", e),
            })
    }

    /// List loaded modules
    pub async fn list_modules(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }

    /// Remove a module
    pub async fn unload_module(&self, name: &str) -> Result<()> {
        let mut modules = self.modules.write().await;
        if modules.remove(name).is_some() {
            tracing::info!(name = %name, "WASM module unloaded");
            Ok(())
        } else {
            Err(ProxyError::ConfigurationError {
                message: format!("Module '{}' not found", name),
            })
        }
    }
}

impl Default for WasmRouter {
    fn default() -> Self {
        Self::new().expect("Failed to create WASM router")
    }
}

