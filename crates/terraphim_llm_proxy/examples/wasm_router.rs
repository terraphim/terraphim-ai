//! Example WASM custom router implementation
//!
//! This example shows how to create a custom routing logic in Rust
//! that can be compiled to WASM and loaded by the proxy.
//!
//! To compile to WASM:
//! ```bash
//! rustc --target wasm32-wasi examples/wasm_router.rs -o examples/wasm_router.wasm
//! ```

use serde::{Deserialize, Serialize};

// Import the same types used by the proxy
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct RoutingInput {
    // Simplified for example - in real implementation, match proxy types exactly
    request_text: String,
    hints: RoutingHints,
    available_providers: Vec<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct RoutingHints {
    is_background: bool,
    is_thinking: bool,
    is_long_context: bool,
    has_images: bool,
    token_count: usize,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct RoutingOutput {
    provider: String,
    confidence: f64,
    reasoning: Option<String>,
}

// Example custom routing logic
#[no_mangle]
pub extern "C" fn route(_input_ptr: i32, _input_len: i32) -> i32 {
    // In a real implementation, this would:
    // 1. Read input JSON from WASM memory at input_ptr
    // 2. Parse it into RoutingInput
    // 3. Apply custom routing logic
    // 4. Write RoutingOutput to WASM memory
    // 5. Return pointer to output

    // For this example, we'll just return a simple decision
    // In practice, you'd need proper memory management

    // Example logic:
    // - Use openrouter for general queries
    // - Use anthropic for complex reasoning
    // - Use ollama for background tasks

    0 // Placeholder return pointer
}

// Example routing logic function (not exported to WASM)
#[allow(dead_code)]
fn route_logic(input: &RoutingInput) -> RoutingOutput {
    let provider = if input.hints.is_thinking {
        "anthropic".to_string()
    } else if input.hints.is_background {
        "ollama".to_string()
    } else {
        "openrouter".to_string()
    };

    RoutingOutput {
        provider,
        confidence: 0.8,
        reasoning: Some("Custom WASM routing logic".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_logic() {
        let input = RoutingInput {
            request_text: "What is the meaning of life?".to_string(),
            hints: RoutingHints {
                is_background: false,
                is_thinking: true,
                is_long_context: false,
                has_images: false,
                token_count: 50,
            },
            available_providers: vec!["openrouter".to_string(), "anthropic".to_string()],
        };

        let output = route_logic(&input);
        assert_eq!(output.provider, "anthropic");
        assert!(output.confidence > 0.5);
    }
}

fn main() {
    println!("This is a WASM routing example. Compile with: rustc --target wasm32-wasi examples/wasm_router.rs -o examples/wasm_router.wasm");
}
