#!/usr/bin/env python3

import argparse
import asyncio
import logging
import os
import sys
from typing import Dict, List, Optional, Any

from mcp import ClientSession, StdioServerParameters, types
from mcp.client.stdio import stdio_client

class TerraphimMcpTester:
    """Test runner for Terraphim MCP Server using the official MCP SDK"""

    def __init__(self, binary_path: str, debug: bool = False):
        self.binary_path = os.path.abspath(binary_path)
        self.debug = debug
        self.logger = self._setup_logging()
        
        # Setup fixtures directory for haystack
        self.haystack_dir = os.path.join(
            os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(self.binary_path)))),
            "terraphim_server/fixtures/haystack"
        )
        if not os.path.exists(self.haystack_dir):
            self.logger.warning(f"Haystack directory not found at {self.haystack_dir}")
            os.makedirs(self.haystack_dir, exist_ok=True)
            # Create a test file
            with open(os.path.join(self.haystack_dir, "test.md"), "w") as f:
                f.write("# Test\nThis is a test document.")

    def _setup_logging(self) -> logging.Logger:
        """Set up logging configuration."""
        log_level = logging.DEBUG if self.debug else logging.INFO
        logging.basicConfig(
            level=log_level,
            format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
            handlers=[logging.StreamHandler(sys.stdout)]
        )
        return logging.getLogger("MCP-E2E-Test")

    async def test_update_config(self, session: ClientSession) -> bool:
        """Test updating configuration."""
        self.logger.info("Testing update_config...")
        
        config = {
            "id": "Server",
            "global_shortcut": "Ctrl+X",
            "roles": {
                "Default": {
                    "name": "Default",
                    "shortname": "default",
                    "relevance_function": "title-scorer",
                    "theme": "spacelab",
                    "haystacks": [{
                        "path": self.haystack_dir,
                        "id": "default-haystack",
                        "name": "Default Haystack",
                        "service": "Ripgrep"
                    }]
                }
            },
            "default_role": "Default",
            "selected_role": "Default"
        }
        
        try:
            result = await session.call_tool(
                "update_config",
                arguments={"config_str": str(config)}
            )
            self.logger.info("Configuration updated successfully")
            return True
        except Exception as e:
            self.logger.error(f"Failed to update config: {e}")
            return False

    async def test_search(self, session: ClientSession) -> bool:
        """Test search functionality."""
        self.logger.info("Testing search...")
        
        try:
            result = await session.call_tool(
                "search",
                arguments={"query": "test"}
            )
            self.logger.info(f"Search completed successfully: {result}")
            return True
        except Exception as e:
            self.logger.error(f"Search failed: {e}")
            return False

    async def test_list_tools(self, session: ClientSession) -> bool:
        """Test listing available tools."""
        self.logger.info("Testing list_tools...")
        
        try:
            tools = await session.list_tools()
            self.logger.info(f"Found tools: {tools}")
            return True
        except Exception as e:
            self.logger.error(f"Failed to list tools: {e}")
            return False

    async def test_list_resources(self, session: ClientSession) -> bool:
        """Test listing available resources."""
        self.logger.info("Testing list_resources...")
        
        try:
            resources = await session.list_resources()
            self.logger.info(f"Found resources: {resources}")
            return True
        except Exception as e:
            self.logger.error(f"Failed to list resources: {e}")
            return False

    async def test_read_resource(self, session: ClientSession) -> bool:
        """Test reading a specific resource."""
        self.logger.info("Testing read_resource...")
        
        try:
            # Try to read terraphim.md resource
            resource_uri = "terraphim://document/terraphim.md"
            content = await session.read_resource(resource_uri)
            self.logger.info(f"Successfully read resource: {resource_uri}")
            return True
        except Exception as e:
            self.logger.error(f"Failed to read resource: {e}")
            return False

    async def test_search_terms(self, session: ClientSession) -> bool:
        """Test searching for specific terms."""
        self.logger.info("Testing search with specific terms...")
        
        terms = ["terraphim", "system"]
        all_passed = True
        
        for term in terms:
            try:
                result = await session.call_tool(
                    "search",
                    arguments={"query": term}
                )
                self.logger.info(f"Search for '{term}' completed: {result}")
            except Exception as e:
                self.logger.error(f"Search for '{term}' failed: {e}")
                all_passed = False
                
        return all_passed

    async def run_tests(self):
        """Run all test cases."""
        self.logger.info("Starting Terraphim MCP Server tests")
        
        server_params = StdioServerParameters(
            command=self.binary_path,
            args=[],
            env=None
        )
        
        try:
            async with stdio_client(server_params) as (read, write):
                async with ClientSession(read, write) as session:
                    # Initialize the connection
                    await session.initialize()
                    self.logger.info("Connected to MCP server")
                    
                    # Run tests in sequence
                    tests = [
                        self.test_update_config,
                        self.test_list_tools,
                        self.test_list_resources,
                        self.test_read_resource,
                        self.test_search_terms,
                        self.test_search
                    ]
                    
                    all_passed = True
                    for test in tests:
                        try:
                            result = await test(session)
                            if result:
                                self.logger.info(f"✅ {test.__name__} passed")
                            else:
                                self.logger.error(f"❌ {test.__name__} failed")
                                all_passed = False
                        except Exception as e:
                            self.logger.error(f"❌ {test.__name__} failed with error: {e}")
                            all_passed = False
                    
                    return all_passed
        except Exception as e:
            self.logger.error(f"Test suite failed: {e}")
            return False

def main():
    parser = argparse.ArgumentParser(description="Terraphim MCP Server E2E Tests")
    parser.add_argument("--binary", required=True, help="Path to terraphim_mcp_server binary")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    args = parser.parse_args()
    
    tester = TerraphimMcpTester(args.binary, args.debug)
    success = asyncio.run(tester.run_tests())
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main() 