#!/usr/bin/env python3

import asyncio
import logging
import sys
import traceback
from mcp import StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.client.session import ClientSession

# Set up logging
logging.basicConfig(level=logging.DEBUG,
                    format='%(asctime)s - %(levelname)s - %(message)s',
                    stream=sys.stdout)
logger = logging.getLogger(__name__)

async def test_mcp_connection():
    """Simple test to verify MCP server connectivity"""
    try:
        binary_path = "/Users/alex/projects/terraphim/terraphim-ai/target/release/terraphim_mcp_server"
        
        logger.info("Creating server parameters...")
        server_params = StdioServerParameters(command=binary_path, args=[], env={})
        
        logger.info("Attempting to connect to MCP server...")
        async with stdio_client(server_params) as (read, write):
            logger.info("Connected! Creating client...")
            client = ClientSession(read, write)
            
            logger.info("Initializing client...")
            result = await client.initialize()
            logger.info(f"Initialize result: {result}")
            
            logger.info("Testing list_tools...")
            tools = await client.list_tools()
            logger.info(f"Tools: {tools}")
            
            logger.info("Test completed successfully!")
            return True
            
    except Exception as e:
        logger.error(f"Test failed with error: {e}")
        logger.error(f"Traceback: {traceback.format_exc()}")
        return False

if __name__ == "__main__":
    success = asyncio.run(test_mcp_connection())
    sys.exit(0 if success else 1) 