#!/usr/bin/env python3

import asyncio
import json
import subprocess

async def test_raw_jsonrpc():
    """Test raw JSON-RPC communication with MCP server"""
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
        # Send a simple initialization request
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

        print(f"Sending: {json.dumps(init_request)}")
        proc.stdin.write(json.dumps(init_request) + "\n")
        proc.stdin.flush()

        # Wait for response
        response_line = proc.stdout.readline()
        print(f"Received: {response_line.strip()}")

        if response_line:
            try:
                response = json.loads(response_line)
                print(f"Parsed response: {response}")
            except json.JSONDecodeError as e:
                print(f"Failed to parse JSON: {e}")

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
    asyncio.run(test_raw_jsonrpc())
