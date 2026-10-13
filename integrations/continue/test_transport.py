"""Actual MCP SDK transport and released CLI integration; no mock server or search."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest

from mcp import ClientSession
from mcp.client.stdio import StdioServerParameters, stdio_client

HERE = Path(__file__).resolve().parent


class TransportTests(unittest.TestCase):
    def test_real_retrieval_and_refusals(self):
        binary = Path(os.environ['TERRAPHIM_GREP_BINARY']).resolve()
        graph = Path(os.environ['TERRAPHIM_EXPORTED_GRAPH']).resolve()
        receipt = {}
        async def exercise():
            with tempfile.TemporaryDirectory() as directory:
                corpus = Path(directory) / 'corpus'
                corpus.mkdir()
                (corpus / 'brief.md').write_text('Risk: supplier delays require a backup plan.\n')
                local_graph = Path(directory) / 'graph.json'
                local_graph.write_bytes(graph.read_bytes())
                baseline = json.loads(subprocess.check_output([str(binary), '--search-only', '--json', '--paths', str(corpus), '--', 'Concern'], cwd=corpus))
                self.assertEqual(baseline['chunks'], [])
                receipt['baseline'] = baseline
                params = StdioServerParameters(command=sys.executable, args=[str(HERE / 'server.py'),
                    '--binary', str(binary), '--binary-sha256', hashlib.sha256(binary.read_bytes()).hexdigest(),
                    '--graph', str(local_graph), '--corpus', str(corpus)])
                async with stdio_client(params) as streams:
                    async with ClientSession(*streams) as session:
                        await session.initialize()
                        tools = await session.list_tools()
                        self.assertEqual([t.name for t in tools.tools], ['terraphim_retrieve'])
                        result = await session.call_tool('terraphim_retrieve', {'query': 'Concern'})
                        self.assertFalse(result.isError)
                        actual = json.loads(result.content[0].text)
                        self.assertIn('--search-only', actual['argv'])
                        self.assertNotIn('--answer', actual['argv'])
                        self.assertIn('supplier delays', json.dumps(actual['output']))
                        self.assertIsNone(actual['output'].get('answer'))
                        receipt['retrieval'] = actual
                        invalid = await session.call_tool('terraphim_retrieve', {'query': '--answer', 'limit': True})
                        self.assertTrue(invalid.isError)
                        disabled = await session.call_tool('terraphim_answer', {'query': 'Concern'})
                        self.assertTrue(disabled.isError)
                        local_graph.write_text('{}')
                        drift = await session.call_tool('terraphim_retrieve', {'query': 'Concern'})
                        self.assertTrue(drift.isError)
                        self.assertIn('changed', drift.content[0].text)
        asyncio.run(exercise())
        if os.environ.get('TERRAPHIM_CONTINUE_RECEIPT'):
            Path(os.environ['TERRAPHIM_CONTINUE_RECEIPT']).write_text(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    unittest.main()
