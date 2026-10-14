#!/usr/bin/env python3
"""Continue MCP transport for the released Terraphim CLI; no search implementation."""
import argparse
import asyncio
import hashlib
import json
import os
import signal
from pathlib import Path
import tempfile

from mcp.server import Server, NotificationOptions
from mcp.server.models import InitializationOptions
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent

MAX_OUTPUT = 8 * 1024 * 1024


def digest(path):
    """Hash an admitted file without loading the executable into memory."""
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument('--binary', required=True, type=Path)
    result.add_argument('--binary-sha256', required=True)
    result.add_argument('--graph', required=True, type=Path)
    result.add_argument('--corpus', required=True, type=Path)
    result.add_argument('--allow-synthesis', action='store_true')
    return result


async def serve(args):
    binary, graph, corpus = (p.resolve(strict=True) for p in (args.binary, args.graph, args.corpus))
    if not binary.is_file() or not graph.is_file() or not corpus.is_dir():
        raise ValueError('Expected binary, exported graph file and corpus directory')
    binary_hash, graph_hash = digest(binary), digest(graph)
    if binary_hash != args.binary_sha256:
        raise ValueError('Binary digest does not match the admitted release')
    server = Server('terraphim-continue')
    concurrent_calls = asyncio.Semaphore(2)
    schema = {'type': 'object', 'additionalProperties': False, 'required': ['query'], 'properties': {
        'query': {'type': 'string', 'minLength': 1, 'maxLength': 2000},
        'limit': {'type': 'integer', 'minimum': 1, 'maximum': 50}}}

    @server.list_tools()
    async def tools():
        result = [Tool(name='terraphim_retrieve', description='Retrieve actual local corpus excerpts using the editor-exported synonym graph. Synthesis is disabled.', inputSchema=schema)]
        if args.allow_synthesis:
            result.append(Tool(name='terraphim_answer', description='Explicit opt-in: retrieve and ask the configured provider to synthesize an answer. Sends retrieved context to that provider; generated output requires source checking.', inputSchema=schema))
        return result

    async def execute(name, arguments):
        arguments = arguments or {}
        query, limit = arguments.get('query'), arguments.get('limit', 8)
        if name not in ('terraphim_retrieve', 'terraphim_answer') or (name == 'terraphim_answer' and not args.allow_synthesis):
            raise ValueError('Tool unavailable; synthesis requires explicit startup opt-in')
        if set(arguments) - {'query', 'limit'} or not isinstance(query, str) or not query.strip() or len(query) > 2000 or type(limit) is not int or not 1 <= limit <= 50:
            raise ValueError('Invalid query or result limit')
        if digest(binary) != binary_hash or digest(graph) != graph_hash:
            raise ValueError('Admitted binary or graph changed; restart after reviewing new inputs')
        argv = [str(binary), '--answer' if name == 'terraphim_answer' else '--search-only', '--json', '--thesaurus', str(graph), '--paths', str(corpus), '-n', str(limit), '--', query]
        # Temporary output is private, bounded, and deleted; provider stderr never becomes tool text.
        with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
            process = await asyncio.create_subprocess_exec(*argv, stdout=output, stderr=errors, stdin=asyncio.subprocess.DEVNULL, cwd=corpus, start_new_session=True)
            try:
                async with asyncio.timeout(90):
                    while process.returncode is None:
                        if output.tell() > MAX_OUTPUT or errors.tell() > MAX_OUTPUT:
                            raise ValueError('CLI output exceeded the local transport limit')
                        await asyncio.sleep(.05)
                    await process.wait()
                if process.returncode:
                    raise ValueError('Terraphim CLI failed; no provider diagnostics exposed')
                if output.tell() > MAX_OUTPUT or errors.tell() > MAX_OUTPUT:
                    raise ValueError('CLI output exceeded the local transport limit')
                output.seek(0)
                actual = json.load(output)
            finally:
                if process.returncode is None:
                    os.killpg(process.pid, signal.SIGKILL)
                await process.wait()
        if digest(binary) != binary_hash or digest(graph) != graph_hash:
            raise ValueError('Inputs changed during retrieval; output refused')
        return [TextContent(type='text', text=json.dumps({'mode': 'generated-answer' if name == 'terraphim_answer' else 'retrieval-only', 'binarySha256': binary_hash, 'graphSha256': graph_hash, 'argv': argv, 'output': actual}))]

    @server.call_tool()
    async def call(name, arguments):
        async with asyncio.timeout(90):
            async with concurrent_calls:
                return await execute(name, arguments)

    async with stdio_server() as (read, write):
        await server.run(read, write, InitializationOptions(server_name='terraphim-continue', server_version='0.1.1', capabilities=server.get_capabilities(notification_options=NotificationOptions(), experimental_capabilities={})))


if __name__ == '__main__':
    asyncio.run(serve(parser().parse_args()))
