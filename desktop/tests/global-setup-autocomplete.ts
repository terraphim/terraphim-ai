/**
 * Global setup for Novel Editor Autocomplete Playwright tests
 * This file handles pre-test setup including service startup and health checks
 */

import { FullConfig } from '@playwright/test';
import { exec } from 'child_process';
import { promisify } from 'util';
import fetch from 'node-fetch';

const execAsync = promisify(exec);

const MCP_SERVER_PORT = process.env.MCP_SERVER_PORT ? parseInt(process.env.MCP_SERVER_PORT) : 8001;
const SETUP_TIMEOUT = 180000; // 3 minutes

/**
 * Check if a port is available
 */
async function isPortAvailable(port: number): Promise<boolean> {
	try {
		const { stdout } = await execAsync(`lsof -ti:${port}`);
		return stdout.trim() === '';
	} catch (error) {
		// lsof returns non-zero exit code if no process found (port available)
		return true;
	}
}

/**
 * Check if MCP server is responding
 */
async function checkMCPServerHealth(port: number): Promise<boolean> {
	try {
		const response = await fetch(`http://localhost:${port}/message?sessionId=setup-test`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				jsonrpc: '2.0',
				id: 1,
				method: 'tools/list',
				params: {},
			}),
			signal: AbortSignal.timeout(5000),
		});

		return response.ok;
	} catch (error) {
		return false;
	}
}

/**
 * Start MCP server for testing
 */
async function startMCPServer(): Promise<{ pid: number; port: number }> {
	console.log(`🚀 Starting MCP server on port ${MCP_SERVER_PORT}...`);

	// Check if server is already running
	if (await checkMCPServerHealth(MCP_SERVER_PORT)) {
		console.log(`✅ MCP server already running on port ${MCP_SERVER_PORT}`);
		return { pid: 0, port: MCP_SERVER_PORT };
	}

	// Check if port is available
	if (!(await isPortAvailable(MCP_SERVER_PORT))) {
		console.log(`⚠️  Port ${MCP_SERVER_PORT} is occupied by another process`);

		// Try to kill the process
		try {
			const { stdout } = await execAsync(`lsof -ti:${MCP_SERVER_PORT}`);
			const pid = parseInt(stdout.trim());
			if (pid) {
				console.log(`🔪 Killing process ${pid} on port ${MCP_SERVER_PORT}`);
				await execAsync(`kill -TERM ${pid}`);
				await new Promise((resolve) => setTimeout(resolve, 2000));
			}
		} catch (error) {
			console.warn(`Could not kill process on port ${MCP_SERVER_PORT}:`, error);
		}
	}

	// Start the MCP server
	const command = `cd ../crates/terraphim_mcp_server && RUST_LOG=info cargo run -- --sse --bind 127.0.0.1:${MCP_SERVER_PORT} --verbose`;

	const serverProcess = exec(command, (error, stdout, stderr) => {
		if (error) {
			console.error(`MCP server error:`, error);
		}
		if (stdout) {
			console.log(`MCP server stdout:`, stdout);
		}
		if (stderr) {
			console.error(`MCP server stderr:`, stderr);
		}
	});

	if (!serverProcess.pid) {
		throw new Error('Failed to start MCP server process');
	}

	console.log(`🔄 MCP server started with PID ${serverProcess.pid}, waiting for health check...`);

	// Wait for server to be ready
	let attempts = 0;
	const maxAttempts = 60; // 60 attempts with 3 second intervals = 3 minutes

	while (attempts < maxAttempts) {
		if (await checkMCPServerHealth(MCP_SERVER_PORT)) {
			console.log(`✅ MCP server is ready on port ${MCP_SERVER_PORT} (attempt ${attempts + 1})`);
			return { pid: serverProcess.pid, port: MCP_SERVER_PORT };
		}

		attempts++;
		console.log(`⏳ Waiting for MCP server... (attempt ${attempts}/${maxAttempts})`);
		await new Promise((resolve) => setTimeout(resolve, 3000));
	}

	// Server failed to start within timeout
	if (serverProcess.pid) {
		try {
			process.kill(serverProcess.pid, 'SIGTERM');
		} catch (error) {
			console.warn(`Could not kill MCP server process ${serverProcess.pid}:`, error);
		}
	}

	throw new Error(`MCP server failed to start within ${maxAttempts * 3} seconds`);
}

/**
 * Setup test directories and files
 */
async function setupTestDirectories(): Promise<void> {
	console.log('📁 Setting up test directories...');

	try {
		// Create test results directories
		await execAsync('mkdir -p test-results/screenshots');
		await execAsync('mkdir -p test-results/videos');
		await execAsync('mkdir -p test-results/traces');
		await execAsync('mkdir -p test-results/autocomplete-artifacts');

		// Create logs directory for MCP server
		await execAsync('mkdir -p ../logs');

		console.log('✅ Test directories created');
	} catch (error) {
		console.warn('⚠️  Could not create test directories:', error);
	}
}

/**
 * Validate test environment
 */
async function validateTestEnvironment(): Promise<void> {
	console.log('🔍 Validating test environment...');

	const checks = [
		{ name: 'Node.js', command: 'node --version' },
		{ name: 'Cargo', command: 'cargo --version' },
		{ name: 'Git', command: 'git --version' },
		{ name: 'curl', command: 'curl --version' },
	];

	for (const check of checks) {
		try {
			const { stdout } = await execAsync(check.command);
			console.log(`✅ ${check.name}: ${stdout.split('\n')[0]}`);
		} catch (error) {
			console.error(`❌ ${check.name}: Not available or failed`);
			throw new Error(`Required dependency ${check.name} is not available`);
		}
	}

	// Check if we're in the right directory
	try {
		await execAsync('ls ../crates/terraphim_mcp_server/Cargo.toml');
		console.log('✅ Terraphim project structure validated');
	} catch (error) {
		throw new Error('Not in the correct Terraphim project directory');
	}
}

/**
 * Main global setup function
 */
async function globalSetup(config: FullConfig): Promise<void> {
	console.log('🎯 Starting Global Setup for Novel Editor Autocomplete Tests');
	console.log(`📊 Environment: ${process.env.CI ? 'CI' : 'Local'}`);
	console.log(`🔧 Config: ${config.configFile || 'default'}`);
	console.log(`⏱️  Setup timeout: ${SETUP_TIMEOUT}ms`);

	const setupStartTime = Date.now();

	try {
		// Step 1: Validate environment
		await validateTestEnvironment();

		// Step 2: Setup directories
		await setupTestDirectories();

		// Step 3: Start MCP server (if not explicitly disabled)
		if (process.env.SKIP_MCP_SERVER !== 'true') {
			const mcpServer = await startMCPServer();

			// Store server info for teardown
			process.env.PLAYWRIGHT_MCP_SERVER_PID = mcpServer.pid.toString();
			process.env.PLAYWRIGHT_MCP_SERVER_PORT = mcpServer.port.toString();

			// Verify server is working with a test request
			console.log('🧪 Testing MCP server functionality...');
			const testResponse = await fetch(
				`http://localhost:${mcpServer.port}/message?sessionId=setup-test`,
				{
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						jsonrpc: '2.0',
						id: 1,
						method: 'tools/call',
						params: {
							name: 'autocomplete_terms',
							arguments: {
								query: 'test',
								limit: 3,
							},
						},
					}),
					signal: AbortSignal.timeout(10000),
				}
			);

			if (testResponse.ok) {
				const result = await testResponse.json();
				console.log('✅ MCP server functionality verified');
				console.log(`📝 Test response: ${JSON.stringify(result).substring(0, 200)}...`);
			} else {
				console.warn(`⚠️  MCP server test request failed: ${testResponse.status}`);
			}
		} else {
			console.log('⏭️  Skipping MCP server startup (SKIP_MCP_SERVER=true)');
		}

		// Step 4: Set global environment variables for tests
		process.env.PLAYWRIGHT_TEST_MODE = 'true';
		process.env.PLAYWRIGHT_AUTOCOMPLETE_TESTS = 'true';
		process.env.PLAYWRIGHT_MCP_SERVER_URL = `http://localhost:${MCP_SERVER_PORT}`;

		const setupDuration = Date.now() - setupStartTime;
		console.log(`🎉 Global setup completed successfully in ${setupDuration}ms`);
		console.log('🚀 Ready to run Novel Editor Autocomplete tests!');
	} catch (error) {
		const setupDuration = Date.now() - setupStartTime;
		console.error(`💥 Global setup failed after ${setupDuration}ms:`, error);

		// Cleanup on failure
		if (process.env.PLAYWRIGHT_MCP_SERVER_PID) {
			try {
				const pid = parseInt(process.env.PLAYWRIGHT_MCP_SERVER_PID);
				process.kill(pid, 'SIGTERM');
				console.log(`🧹 Cleaned up MCP server process ${pid}`);
			} catch (cleanupError) {
				console.warn('Could not cleanup MCP server process:', cleanupError);
			}
		}

		throw error;
	}
}

// Handle timeout
const timeoutId = setTimeout(() => {
	console.error(`⏰ Global setup timed out after ${SETUP_TIMEOUT}ms`);
	process.exit(1);
}, SETUP_TIMEOUT);

// Clear timeout when setup completes
process.on('beforeExit', () => {
	clearTimeout(timeoutId);
});

export default globalSetup;
