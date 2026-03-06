import argparse
import asyncio
import logging
import sys
import mcp
from mcp import StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.client.session import ClientSession

class E2ETestRunner:
    def __init__(self, binary_path: str, debug_mode: bool = False, expect_data: bool = False):
        self.binary_path = binary_path
        self.debug_mode = debug_mode
        self.expect_data = expect_data
        self.logger = self._setup_logger()

    def _setup_logger(self):
        log_level = logging.DEBUG if self.debug_mode else logging.INFO
        logging.basicConfig(level=log_level,
                            format='%(asctime)s - %(levelname)s - %(message)s',
                            stream=sys.stdout)
        return logging.getLogger(__name__)

    async def run_tests(self):
        try:
            # Start server automatically via mcp stdio_client
            server_params = StdioServerParameters(command=self.binary_path, args=[], env={})
            self.logger.info("Starting Terraphim MCP Server via mcp stdio_client...")
            async with stdio_client(server_params) as (read, write):
                client = ClientSession(read, write)
                self.logger.info("Client connected to server. Running protocol tests...")

                # Initialize
                await client.initialize()
                self.logger.info("✅ initialize succeeded")

                # list tools
                tools_result = await client.list_tools()
                tools = tools_result.tools
                self.logger.info(f"✅ list_tools returned {len(tools)} tools")
                tool_names = [t.name for t in tools]
                expected = ["search", "update_config_tool"]
                for name in expected:
                    if name not in tool_names:
                        self.logger.error(f"Expected tool {name} missing")
                        return False

                # call search tool
                search_result = await client.call_tool(mcp.CallToolRequest(name="search", arguments={"query": "test"}))
                self.logger.info("✅ search tool executed")

                # list resources
                res_list = await client.list_resources()
                self.logger.info(f"✅ list_resources returned {len(res_list.resources)} resources")

                # read first resource if exists
                if res_list.resources:
                    first_uri = res_list.resources[0].uri
                    await client.read_resource(mcp.ReadResourceRequest(uri=first_uri))
                    self.logger.info("✅ read_resource succeeded")

                self.logger.info("All MCP protocol tests passed.")
                return True
        except Exception as e:
            self.logger.error(f"Error during protocol tests: {e}")
            return False

def main():
    parser = argparse.ArgumentParser(description="E2E Test Runner for Terraphim MCP Server")
    parser.add_argument("--binary", required=True, help="Path to the terraphim_mcp_server binary")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    parser.add_argument("--expect-data", action="store_true", help="Strict mode, expect data to be available")
    args = parser.parse_args()

    runner = E2ETestRunner(args.binary, args.debug, args.expect_data)

    # Use asyncio.run() instead of manually managing the event loop
    try:
        if asyncio.run(runner.run_tests()):
            logging.info("✅ All tests passed!")
            sys.exit(0)
        else:
            logging.error("❌ Tests failed.")
            sys.exit(1)
    except KeyboardInterrupt:
        logging.info("Tests interrupted by user")
        sys.exit(1)
    except Exception as e:
        logging.error(f"Unexpected error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
