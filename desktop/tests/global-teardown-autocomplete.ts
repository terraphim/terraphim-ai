/**
 * Global teardown for Novel Editor Autocomplete Playwright tests
 * This file handles post-test cleanup including service shutdown and artifact collection
 */

import type { FullConfig } from '@playwright/test';
import { exec } from 'child_process';
import * as fs from 'fs/promises';
import * as path from 'path';
import { promisify } from 'util';

const execAsync = promisify(exec);

/**
 * Stop MCP server process
 */
async function stopMCPServer(): Promise<void> {
	const pidStr = process.env.PLAYWRIGHT_MCP_SERVER_PID;
	const port = process.env.PLAYWRIGHT_MCP_SERVER_PORT || '8001';

	if (!pidStr || pidStr === '0') {
		console.log('📋 No MCP server PID found, checking port...');

		// Try to find and kill process by port
		try {
			const { stdout } = await execAsync(`lsof -ti:${port}`);
			const pid = parseInt(stdout.trim());

			if (pid) {
				console.log(`🔍 Found process ${pid} on port ${port}`);
				await execAsync(`kill -TERM ${pid}`);
				console.log(`🛑 Terminated process ${pid} on port ${port}`);

				// Wait for graceful shutdown
				await new Promise((resolve) => setTimeout(resolve, 3000));

				// Force kill if still running
				try {
					await execAsync(`kill -0 ${pid}`);
					console.log(`🔪 Force killing process ${pid}`);
					await execAsync(`kill -KILL ${pid}`);
				} catch (error) {
					// Process already terminated
				}
			}
		} catch (error) {
			console.log(`✅ No process found on port ${port}`);
		}

		return;
	}

	const pid = parseInt(pidStr);

	console.log(`🛑 Stopping MCP server (PID: ${pid})...`);

	try {
		// Check if process is still running
		await execAsync(`kill -0 ${pid}`);

		// Graceful termination
		console.log(`📨 Sending SIGTERM to process ${pid}`);
		process.kill(pid, 'SIGTERM');

		// Wait for graceful shutdown
		let attempts = 0;
		const maxAttempts = 10;

		while (attempts < maxAttempts) {
			try {
				await execAsync(`kill -0 ${pid}`);
				console.log(
					`⏳ Waiting for process ${pid} to terminate... (${attempts + 1}/${maxAttempts})`
				);
				await new Promise((resolve) => setTimeout(resolve, 1000));
				attempts++;
			} catch (error) {
				console.log(`✅ Process ${pid} terminated gracefully`);
				return;
			}
		}

		// Force kill if still running
		console.log(`🔪 Force killing process ${pid}`);
		process.kill(pid, 'SIGKILL');

		await new Promise((resolve) => setTimeout(resolve, 2000));
		console.log(`💀 Process ${pid} force terminated`);
	} catch (error) {
		if (error.message.includes('No such process')) {
			console.log(`✅ Process ${pid} was already terminated`);
		} else {
			console.warn(`⚠️  Could not stop MCP server process ${pid}:`, error.message);

			// Try to kill by port as fallback
			try {
				const { stdout } = await execAsync(`lsof -ti:${port}`);
				const portPid = parseInt(stdout.trim());
				if (portPid && portPid !== pid) {
					await execAsync(`kill -KILL ${portPid}`);
					console.log(`🔪 Force killed different process ${portPid} on port ${port}`);
				}
			} catch (fallbackError) {
				console.log(`✅ Port ${port} is now free`);
			}
		}
	}
}

/**
 * Collect test artifacts and logs
 */
async function collectTestArtifacts(): Promise<void> {
	console.log('📦 Collecting test artifacts...');

	try {
		const artifactsDir = 'test-results/autocomplete-artifacts';
		const timestamp = new Date().toISOString().replace(/[:.]/g, '-');

		// Create final artifacts directory
		const finalArtifactsDir = `${artifactsDir}/run-${timestamp}`;
		await fs.mkdir(finalArtifactsDir, { recursive: true });

		// Copy important files
		const filesToCopy = [
			{ src: 'test-results/autocomplete-results.json', dest: 'test-results.json' },
			{ src: 'test-results/autocomplete-junit.xml', dest: 'junit-results.xml' },
			{ src: '../logs', dest: 'server-logs', isDir: true },
		];

		for (const file of filesToCopy) {
			try {
				const srcPath = file.src;
				const destPath = path.join(finalArtifactsDir, file.dest);

				if (file.isDir) {
					// Copy directory
					await execAsync(`cp -r "${srcPath}" "${destPath}"`);
					console.log(`📁 Copied directory ${srcPath} -> ${destPath}`);
				} else {
					// Copy file
					await fs.copyFile(srcPath, destPath);
					console.log(`📄 Copied file ${srcPath} -> ${destPath}`);
				}
			} catch (error) {
				console.log(`⏭️  Skipped ${file.src} (not found or inaccessible)`);
			}
		}

		// Create test summary
		const summary = {
			timestamp: new Date().toISOString(),
			environment: process.env.CI ? 'CI' : 'Local',
			mcpServerPort: process.env.PLAYWRIGHT_MCP_SERVER_PORT,
			testDuration: process.env.PLAYWRIGHT_TEST_DURATION,
			nodeVersion: process.version,
			platform: process.platform,
			arch: process.arch,
		};

		await fs.writeFile(
			path.join(finalArtifactsDir, 'summary.json'),
			JSON.stringify(summary, null, 2)
		);

		console.log(`✅ Test artifacts collected in ${finalArtifactsDir}`);
	} catch (error) {
		console.warn('⚠️  Could not collect all test artifacts:', error.message);
	}
}

/**
 * Generate test report summary
 */
async function generateTestReportSummary(): Promise<void> {
	console.log('📊 Generating test report summary...');

	try {
		const resultsPath = 'test-results/autocomplete-results.json';

		try {
			const resultsData = await fs.readFile(resultsPath, 'utf8');
			const results = JSON.parse(resultsData);

			const summary = {
				totalTests:
					results.suites?.reduce((total, suite) => total + (suite.specs?.length || 0), 0) || 0,
				passedTests: 0,
				failedTests: 0,
				skippedTests: 0,
				duration: results.duration || 0,
				timestamp: new Date().toISOString(),
			};

			// Count test results
			if (results.suites) {
				for (const suite of results.suites) {
					if (suite.specs) {
						for (const spec of suite.specs) {
							if (spec.tests) {
								for (const test of spec.tests) {
									if (test.results) {
										for (const result of test.results) {
											switch (result.status) {
												case 'passed':
													summary.passedTests++;
													break;
												case 'failed':
													summary.failedTests++;
													break;
												case 'skipped':
													summary.skippedTests++;
													break;
											}
										}
									}
								}
							}
						}
					}
				}
			}

			console.log('📈 Test Summary:');
			console.log(`   Total Tests: ${summary.totalTests}`);
			console.log(`   ✅ Passed: ${summary.passedTests}`);
			console.log(`   ❌ Failed: ${summary.failedTests}`);
			console.log(`   ⏭️  Skipped: ${summary.skippedTests}`);
			console.log(`   ⏱️  Duration: ${Math.round(summary.duration / 1000)}s`);

			// Write summary to file
			await fs.writeFile(
				'test-results/autocomplete-summary.json',
				JSON.stringify(summary, null, 2)
			);
		} catch (error) {
			console.log('⏭️  No test results found to summarize');
		}
	} catch (error) {
		console.warn('⚠️  Could not generate test report summary:', error.message);
	}
}

/**
 * Cleanup temporary files and processes
 */
async function cleanupTempFiles(): Promise<void> {
	console.log('🧹 Cleaning up temporary files...');

	const tempPaths = ['/tmp/playwright*', '/tmp/chromium*', 'test-results/.tmp', '.playwright'];

	for (const tmpPath of tempPaths) {
		try {
			await execAsync(`rm -rf ${tmpPath}`);
		} catch (error) {
			// Ignore cleanup errors
		}
	}

	// Kill any remaining processes
	const processesToKill = ['terraphim_mcp_server', 'terraphim_server', 'chromium', 'firefox'];

	for (const processName of processesToKill) {
		try {
			await execAsync(`pkill -f ${processName}`);
		} catch (error) {
			// Ignore if processes not found
		}
	}

	console.log('✅ Cleanup completed');
}

/**
 * Main global teardown function
 */
async function globalTeardown(config: FullConfig): Promise<void> {
	console.log('🏁 Starting Global Teardown for Novel Editor Autocomplete Tests');

	const teardownStartTime = Date.now();

	try {
		// Step 1: Stop MCP server
		if (process.env.SKIP_MCP_SERVER !== 'true') {
			await stopMCPServer();
		}

		// Step 2: Collect test artifacts
		await collectTestArtifacts();

		// Step 3: Generate test report summary
		await generateTestReportSummary();

		// Step 4: Cleanup temporary files (in CI only)
		if (process.env.CI) {
			await cleanupTempFiles();
		}

		const teardownDuration = Date.now() - teardownStartTime;
		console.log(`✨ Global teardown completed successfully in ${teardownDuration}ms`);
	} catch (error) {
		const teardownDuration = Date.now() - teardownStartTime;
		console.error(
			`💥 Global teardown encountered errors after ${teardownDuration}ms:`,
			error.message
		);

		// Still try to cleanup critical resources
		try {
			await stopMCPServer();
		} catch (cleanupError) {
			console.error('Critical cleanup failed:', cleanupError.message);
		}

		// Don't throw error to avoid masking test results
		console.warn('⚠️  Continuing despite teardown errors...');
	}

	// Clear environment variables
	delete process.env.PLAYWRIGHT_MCP_SERVER_PID;
	delete process.env.PLAYWRIGHT_MCP_SERVER_PORT;
	delete process.env.PLAYWRIGHT_TEST_MODE;
	delete process.env.PLAYWRIGHT_AUTOCOMPLETE_TESTS;
	delete process.env.PLAYWRIGHT_MCP_SERVER_URL;

	console.log('👋 Farewell from Novel Editor Autocomplete tests!');
}

export default globalTeardown;
