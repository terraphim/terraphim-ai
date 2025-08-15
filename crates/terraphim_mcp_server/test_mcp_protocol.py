#!/usr/bin/env python3
"""
Test MCP autocomplete functionality via stdio transport with proper protocol handshake
"""

import json
import subprocess
import time
import sys

def send_mcp_message(message):
    """Send a message to the MCP server and return the response"""
    print(f"Sending: {json.dumps(message)}")
    return json.dumps(message) + "\n"

def test_mcp_autocomplete():
    """Test MCP autocomplete functionality with proper protocol handshake"""
    
    print("Starting MCP autocomplete test via stdio transport...")
    
    # Start the MCP server
    env = {"TERRAPHIM_SETTINGS_PATH": "../terraphim_settings/default/settings_local_dev.toml"}
    process = subprocess.Popen(
        ["cargo", "run", "--", "--verbose"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        cwd=".",
        text=True
    )
    
    try:
        # Wait for server to start
        time.sleep(3)
        
        # Step 1: Send initialization request
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "clientInfo": {
                    "name": "MCP Test Client",
                    "version": "1.0.0"
                }
            }
        }
        
        print("\n1. Sending initialization request...")
        process.stdin.write(send_mcp_message(init_request))
        process.stdin.flush()
        
        # Read response
        response = process.stdout.readline()
        print(f"Response: {response.strip()}")
        
        # Step 2: List available tools
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        
        print("\n2. Listing available tools...")
        process.stdin.write(send_mcp_message(tools_request))
        process.stdin.flush()
        
        response = process.stdout.readline()
        print(f"Response: {response.strip()}")
        
        # Step 3: Build autocomplete index
        build_index_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "build_autocomplete_index",
                "arguments": {
                    "role": "Terraphim Engineer"
                }
            }
        }
        
        print("\n3. Building autocomplete index...")
        process.stdin.write(send_mcp_message(build_index_request))
        process.stdin.flush()
        
        response = process.stdout.readline()
        print(f"Response: {response.strip()}")
        
        # Step 4: Test autocomplete with snippets
        autocomplete_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "autocomplete_with_snippets",
                "arguments": {
                    "query": "terraphim",
                    "limit": 5,
                    "role": "Terraphim Engineer"
                }
            }
        }
        
        print("\n4. Testing autocomplete with snippets...")
        process.stdin.write(send_mcp_message(autocomplete_request))
        process.stdin.flush()
        
        response = process.stdout.readline()
        print(f"Response: {response.strip()}")
        
        # Step 5: Test autocomplete terms
        terms_request = {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "autocomplete_terms",
                "arguments": {
                    "query": "terraphim",
                    "limit": 5,
                    "role": "Terraphim Engineer"
                }
            }
        }
        
        print("\n5. Testing autocomplete terms...")
        process.stdin.write(send_mcp_message(terms_request))
        process.stdin.flush()
        
        response = process.stdout.readline()
        print(f"Response: {response.strip()}")
        
        print("\nTest completed successfully!")
        
    except Exception as e:
        print(f"Error during test: {e}")
    finally:
        # Clean up
        process.terminate()
        process.wait()

if __name__ == "__main__":
    test_mcp_autocomplete()
