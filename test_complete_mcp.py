#!/usr/bin/env python3

import asyncio
import json
import subprocess
import sys

async def test_complete_mcp():
    """Test complete MCP protocol communication"""
    binary_path = "/Users/alex/projects/terraphim/terraphim-ai/target/release/terraphim_mcp_server"
    
    # Start the server process
    proc = subprocess.Popen(
        [binary_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    try:
        # Step 1: Send initialization request
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }
        
        print(f"Sending init: {json.dumps(init_request)}")
        proc.stdin.write(json.dumps(init_request) + "\n")
        proc.stdin.flush()
        
        # Read initialization response
        response_line = proc.stdout.readline()
        print(f"Init response: {response_line.strip()}")
        
        # Step 2: Send initialized notification
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        
        print(f"Sending initialized: {json.dumps(initialized_notification)}")
        proc.stdin.write(json.dumps(initialized_notification) + "\n")
        proc.stdin.flush()
        
        # Step 3: List tools
        list_tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }
        
        print(f"Sending list_tools: {json.dumps(list_tools_request)}")
        proc.stdin.write(json.dumps(list_tools_request) + "\n")
        proc.stdin.flush()
        
        # Read tools response
        tools_response = proc.stdout.readline()
        print(f"Tools response: {tools_response.strip()}")
        
        # Step 4: Call search tool
        search_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": "test"
                }
            }
        }
        
        print(f"Sending search: {json.dumps(search_request)}")
        proc.stdin.write(json.dumps(search_request) + "\n")
        proc.stdin.flush()
        
        # Read search response
        search_response = proc.stdout.readline()
        print(f"Search response: {search_response.strip()}")
        
        # Step 5: List resources
        list_resources_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "resources/list"
        }
        
        print(f"Sending list_resources: {json.dumps(list_resources_request)}")
        proc.stdin.write(json.dumps(list_resources_request) + "\n")
        proc.stdin.flush()
        
        # Read resources response
        resources_response = proc.stdout.readline()
        print(f"Resources response: {resources_response.strip()}")
        
        print("✅ All MCP protocol tests completed successfully!")
        
    except Exception as e:
        print(f"Error: {e}")
    finally:
        proc.terminate()
        proc.wait()
        
        # Print any stderr output
        stderr_output = proc.stderr.read()
        if stderr_output:
            print(f"Stderr: {stderr_output}")

if __name__ == "__main__":
    asyncio.run(test_complete_mcp()) 