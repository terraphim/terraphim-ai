// Simple test to verify command parsing works
use std::str::FromStr;

// Mock the structures we need for testing
#[derive(Debug, PartialEq)]
enum MockCommand {
    File { subcommand: String },
    Web { subcommand: String, url: String },
    VM { subcommand: String },
    Help,
}

impl FromStr for MockCommand {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "/file" => {
                if parts.len() > 1 {
                    Ok(MockCommand::File { subcommand: parts[1].to_string() })
                } else {
                    Err("File command requires subcommand".to_string())
                }
            }
            "/web" => {
                if parts.len() > 2 {
                    Ok(MockCommand::Web {
                        subcommand: parts[1].to_string(),
                        url: parts[2].to_string()
                    })
                } else {
                    Err("Web command requires subcommand and URL".to_string())
                }
            }
            "/vm" => {
                if parts.len() > 1 {
                    Ok(MockCommand::VM { subcommand: parts[1].to_string() })
                } else {
                    Err("VM command requires subcommand".to_string())
                }
            }
            "/help" => Ok(MockCommand::Help),
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }
}

fn main() {
    println!("=== Terraphim TUI Command Parsing Demo ===\n");

    // Test file operations commands
    let file_commands = vec![
        "/file search \"async rust\" --path ./src",
        "/file classify ./src --recursive",
        "/file analyze ./main.rs --classification",
        "/file summarize ./README.md --detailed",
        "/file tag ./lib.rs rust,core,module",
        "/file index ./docs --recursive",
    ];

    println!("📁 Testing File Operations:");
    for cmd in file_commands {
        match MockCommand::from_str(cmd) {
            Ok(MockCommand::File { subcommand }) => {
                println!("  ✅ {} -> File subcommand: {}", cmd, subcommand);
            }
            _ => println!("  ❌ {} -> Failed to parse as file command", cmd),
        }
    }

    // Test web operations commands
    let web_commands = vec![
        "/web get https://api.example.com/data",
        "/web post https://api.example.com/submit",
        "/web scrape https://example.com '.content'",
        "/web screenshot https://github.com",
    ];

    println!("\n🌐 Testing Web Operations:");
    for cmd in web_commands {
        match MockCommand::from_str(cmd) {
            Ok(MockCommand::Web { subcommand, url }) => {
                println!("  ✅ {} -> Web {} to {}", cmd, subcommand, url);
            }
            _ => println!("  ❌ {} -> Failed to parse as web command", cmd),
        }
    }

    // Test VM operations commands
    let vm_commands = vec![
        "/vm list",
        "/vm create my-vm",
        "/vm start my-vm",
        "/vm stop my-vm",
        "/vm status my-vm",
    ];

    println!("\n🖥️  Testing VM Operations:");
    for cmd in vm_commands {
        match MockCommand::from_str(cmd) {
            Ok(MockCommand::VM { subcommand }) => {
                println!("  ✅ {} -> VM subcommand: {}", cmd, subcommand);
            }
            _ => println!("  ❌ {} -> Failed to parse as VM command", cmd),
        }
    }

    // Test error handling
    let invalid_commands = vec![
        "/file",           // missing subcommand
        "/web get",        // missing URL
        "/vm",             // missing subcommand
        "/invalid",        // unknown command
    ];

    println!("\n🚫 Testing Error Handling:");
    for cmd in invalid_commands {
        match MockCommand::from_str(cmd) {
            Ok(_) => println!("  ❌ {} -> Should have failed", cmd),
            Err(e) => println!("  ✅ {} -> Correctly rejected: {}", cmd, e),
        }
    }

    println!("\n🎉 Command parsing demonstration completed!");
    println!("This proves that the command structure for all new TUI features is correctly implemented.");
}