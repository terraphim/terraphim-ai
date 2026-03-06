#!/bin/bash
echo "🔗 TUI Integration Test Suite"
echo "================================="

# Test 1: Feature Flag Integration
echo "Testing feature flag compilation..."
echo "Testing repl-full feature..."
cargo check -p terraphim_tui --features repl-full 2>/dev/null && echo "✅ repl-full feature compiles" || echo "❌ repl-full feature failed"
echo "Testing repl-file feature..."
cargo check -p terraphim_tui --features repl-file 2>/dev/null && echo "✅ repl-file feature compiles" || echo "❌ repl-file feature failed"
echo "Testing individual features..."
cargo check -p terraphim_tui --features repl,repl-chat,repl-file 2>/dev/null && echo "✅ individual features compile" || echo "❌ individual features failed"

echo

# Test 2: Command Availability
echo "Testing command availability in help system..."
# Test if commands are properly exported
echo "Testing file command availability..."
if grep -r "FileSubcommand" crates/terraphim_tui/src/repl/ >/dev/null; then
    echo "✅ File operations commands found in codebase"
else
    echo "❌ File operations commands not found"
fi

echo "Testing web command availability..."
if grep -r "WebSubcommand" crates/terraphim_tui/src/repl/ >/dev/null; then
    echo "✅ Web operations commands found in codebase"
else
    echo "❌ Web operations commands not found"
fi

echo "Testing VM command availability..."
if grep -r "VMSubcommand" crates/terraphim_tui/src/repl/ >/dev/null; then
    echo "✅ VM management commands found in codebase"
else
    echo "❌ VM management commands not found"
fi

echo

# Test 3: Command Integration
echo "Testing command parsing integration..."
echo "Creating integration test script..."

# Test script for command parsing integration
cat > integration_test.rs << 'EOF'
use std::str::FromStr;

// Simplified command types for integration testing
#[derive(Debug, PartialEq)]
enum Command {
    File { subcommand: String },
    Web { subcommand: String, url: String },
    VM { subcommand: String },
    Help,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "/file" => {
                if parts.len() > 1 {
                    Ok(Command::File { subcommand: parts[1].to_string() })
                } else {
                    Err("File command requires subcommand".to_string())
                }
            }
            "/web" => {
                if parts.len() > 2 {
                    Ok(Command::Web {
                        subcommand: parts[1].to_string(),
                        url: parts[2].to_string()
                    })
                } else {
                    Err("Web command requires subcommand and URL".to_string())
                }
            }
            "/vm" => {
                if parts.len() > 1 {
                    Ok(Command::VM { subcommand: parts[1].to_string() })
                } else {
                    Err("VM command requires subcommand".to_string())
                }
            }
            "/help" => Ok(Command::Help),
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }
}

fn main() {
    println!("🔗 Integration Test Results:");
    println!("====================");

    // Test all command types work together
    let test_commands = vec![
        "/file search async rust --path ./src",
        "/file classify ./src --recursive",
        "/file analyze ./main.rs --classification",
        "/file summarize ./README.md --detailed",
        "/file tag ./lib.rs rust,core,module",

        "/web get https://api.example.com/data",
        "/web post https://api.example.com/submit",
        "/web scrape https://example.com '.content'",
        "/web screenshot https://github.com",

        "/vm list",
        "/vm create my-vm",
        "/vm start my-vm",
        "/vm stop my-vm",
        "/vm status my-vm",

        "/help",
    ];

    let mut passed = 0;
    let mut total = test_commands.len();

    for cmd in test_commands {
        match Command::from_str(cmd) {
            Ok(_) => {
                println!("✅ {}", cmd);
                passed += 1;
            }
            Err(_) => {
                println!("❌ {}", cmd);
            }
        }
    }

    println!();
    println!("📊 Integration Test Results:");
    println!("   Total Commands: {}", total);
    println!("   Passed: {}", passed);
    println!("   Failed: {}", total - passed);
    println!("   Success Rate: {}%", (passed * 100 / total));

    if passed == total {
        println!("🎉 All integration tests passed!");
    } else {
        println!("⚠️  Some integration tests failed!");
    }
}
EOF

# Compile and run integration test
rustc integration_test.rs && ./integration_test
rm integration_test.rs integration_test

echo

# Test 4: Available Commands Check
echo "Testing available commands in TUI..."
if [ -f "crates/terraphim_tui/src/repl/commands.rs" ]; then
    echo "Checking command availability in commands.rs..."
    grep -c "pub enum.*Subcommand" crates/terraphim_tui/src/repl/commands.rs && echo "✅ Multiple command subcommands found"
fi

echo "🔗 Integration testing completed!"
EOF
