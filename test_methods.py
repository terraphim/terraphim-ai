#!/usr/bin/env python3

import asyncio
import json
import subprocess

async def test_server_methods():
    """Test what methods the server supports"""
    binary_path = "/Users/alex/projects/terraphim/terraphim-ai/target/release/terraphim_mcp_server"

    proc = subprocess.Popen(
        [binary_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    try:
        # Step 1: Initialize
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

        proc.stdin.write(json.dumps(init_request) + "\n")
        proc.stdin.flush()
        response = proc.stdout.readline()
        print(f"Initialize: {response.strip()}")

        # Step 2: Initialized notification
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        proc.stdin.write(json.dumps(initialized_notification) + "\n")
        proc.stdin.flush()

        # Test different method calls to see what's supported
        test_methods = [
            ("tools/list", {}),
            ("tools/call", {"name": "search", "arguments": {"query": "test"}}),
            ("call_tool", {"name": "search", "arguments": {"query": "test"}}),
            ("resources/list", {}),
            ("list_resources", {}),
            ("list_tools", {}),
        ]

        for i, (method, params) in enumerate(test_methods):
            request = {
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": method,
                "params": params
            }

            print(f"\nTesting method: {method}")
            proc.stdin.write(json.dumps(request) + "\n")
            proc.stdin.flush()

            response = proc.stdout.readline()
            print(f"Response: {response.strip()}")

    except Exception as e:
        print(f"Error: {e}")
    finally:
        proc.terminate()
        proc.wait()

        stderr_output = proc.stderr.read()
        if stderr_output:
            print(f"Stderr: {stderr_output}")

if __name__ == "__main__":
    asyncio.run(test_server_methods())
