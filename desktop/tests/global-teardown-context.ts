/**
 * Global teardown for Context Management UI tests
 * 
 * This file handles cleanup after context management tests complete,
 * including service shutdown, artifact collection, and test reporting.
 */

import { FullConfig, FullResult } from '@playwright/test';
import { exec } from 'child_process';
import { promisify } from 'util';
import fs from 'fs/promises';
import path from 'path';

const execAsync = promisify(exec);

interface TestSummary {
  timestamp: string;
  duration: number;
  passed: number;
  failed: number;
  skipped: number;
  total: number;
  projects: string[];
  artifacts: string[];
}

/**
 * Shutdown backend services gracefully
 */
async function shutdownServices(): Promise<void> {
  console.log('🛑 Shutting down backend services...');

  const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
  const MCP_SERVER_PORT = process.env.MCP_SERVER_PORT || '8001';

  try {
    // Find and terminate backend server processes
    console.log('🔍 Finding backend server processes...');
    
    // Kill processes listening on our test ports
    const portsToKill = [BACKEND_PORT, MCP_SERVER_PORT];
    
    for (const port of portsToKill) {
      try {
        // Find processes using the port
        const { stdout } = await execAsync(`lsof -ti:${port} 2>/dev/null || true`);
        const pids = stdout.trim().split('\n').filter(pid => pid);
        
        if (pids.length > 0) {
          console.log(`🔪 Terminating processes on port ${port}: ${pids.join(', ')}`);
          
          // Try graceful shutdown first
          for (const pid of pids) {
            if (pid) {
              try {
                await execAsync(`kill -TERM ${pid}`);
                console.log(`✅ Sent TERM signal to process ${pid}`);
              } catch (error) {
                console.warn(`⚠️ Could not send TERM signal to ${pid}`);
              }
            }
          }
          
          // Wait a moment for graceful shutdown
          await new Promise(resolve => setTimeout(resolve, 3000));
          
          // Force kill if still running
          for (const pid of pids) {
            if (pid) {
              try {
                await execAsync(`kill -KILL ${pid} 2>/dev/null || true`);
                console.log(`🔪 Force killed process ${pid}`);
              } catch (error) {
                // Process might already be dead, which is fine
              }
            }
          }
        } else {
          console.log(`✅ No processes found on port ${port}`);
        }
      } catch (error) {
        console.warn(`⚠️ Error checking port ${port}:`, error);
      }
    }

    // Kill any remaining Terraphim processes
    try {
      await execAsync(`pkill -f "terraphim_server|terraphim_mcp_server" || true`);
      console.log('🧹 Cleaned up any remaining Terraphim processes');
    } catch (error) {
      console.warn('⚠️ Error cleaning up processes:', error);
    }

    console.log('✅ Service shutdown completed');

  } catch (error) {
    console.error('❌ Error during service shutdown:', error);
  }
}

/**
 * Collect and organize test artifacts
 */
async function collectTestArtifacts(): Promise<string[]> {
  console.log('📦 Collecting test artifacts...');

  const artifacts: string[] = [];
  const artifactDirs = [
    'test-results/context-artifacts',
    'test-results/screenshots', 
    'test-results/videos',
    'test-results/traces',
  ];

  try {
    // Create archive directory
    const archiveDir = `test-results/archive-${Date.now()}`;
    await fs.mkdir(archiveDir, { recursive: true });

    for (const dir of artifactDirs) {
      try {
        const files = await fs.readdir(dir);
        if (files.length > 0) {
          console.log(`📁 Found ${files.length} files in ${dir}`);
          
          // Copy files to archive
          for (const file of files) {
            const sourcePath = path.join(dir, file);
            const destPath = path.join(archiveDir, path.basename(dir) + '_' + file);
            
            try {
              await fs.copyFile(sourcePath, destPath);
              artifacts.push(destPath);
            } catch (copyError) {
              console.warn(`⚠️ Could not copy ${sourcePath}:`, copyError);
            }
          }
        }
      } catch (error) {
        console.warn(`⚠️ Could not read directory ${dir}:`, error);
      }
    }

    // Copy important logs
    const logFiles = [
      'test-results/environment.json',
      'test-results/test-data.json',
    ];

    for (const logFile of logFiles) {
      try {
        const destPath = path.join(archiveDir, path.basename(logFile));
        await fs.copyFile(logFile, destPath);
        artifacts.push(destPath);
        console.log(`📄 Copied ${logFile}`);
      } catch (error) {
        console.warn(`⚠️ Could not copy ${logFile}:`, error);
      }
    }

    console.log(`✅ Collected ${artifacts.length} test artifacts`);
    return artifacts;

  } catch (error) {
    console.error('❌ Error collecting test artifacts:', error);
    return [];
  }
}

/**
 * Generate test summary report
 */
async function generateTestReport(result: FullResult, artifacts: string[]): Promise<void> {
  console.log('📊 Generating test summary report...');

  try {
    const summary: TestSummary = {
      timestamp: new Date().toISOString(),
      duration: result.duration,
      passed: 0,
      failed: 0,
      skipped: 0,
      total: 0,
      projects: [],
      artifacts: artifacts.map(a => path.basename(a)),
    };

    // Calculate test statistics from result
    if (result.suites) {
      for (const suite of result.suites) {
        for (const spec of suite.specs) {
          summary.total++;
          
          let specPassed = false;
          let specFailed = false;
          let specSkipped = false;

          for (const test of spec.tests) {
            for (const result of test.results) {
              switch (result.status) {
                case 'passed':
                  specPassed = true;
                  break;
                case 'failed':
                  specFailed = true;
                  break;
                case 'skipped':
                  specSkipped = true;
                  break;
              }
            }
          }

          if (specFailed) {
            summary.failed++;
          } else if (specSkipped) {
            summary.skipped++;
          } else if (specPassed) {
            summary.passed++;
          }
        }
      }
    }

    // Write summary report
    const reportPath = 'test-results/context-summary.json';
    await fs.writeFile(reportPath, JSON.stringify(summary, null, 2));

    // Write human-readable report
    const humanReport = `
# Context Management UI Test Summary

**Test Run Completed**: ${summary.timestamp}
**Duration**: ${Math.round(summary.duration / 1000)}s

## Results
- ✅ **Passed**: ${summary.passed}
- ❌ **Failed**: ${summary.failed}
- ⏭️ **Skipped**: ${summary.skipped}
- 📊 **Total**: ${summary.total}

## Artifacts
${summary.artifacts.map(a => `- ${a}`).join('\n')}

## Status
${summary.failed === 0 ? '🎉 **ALL TESTS PASSED!**' : `⚠️ **${summary.failed} test(s) failed**`}
`;

    await fs.writeFile('test-results/context-summary.md', humanReport);

    console.log(`✅ Test summary saved to ${reportPath}`);
    console.log(`📊 Results: ${summary.passed} passed, ${summary.failed} failed, ${summary.skipped} skipped`);

    // Log final status
    if (summary.failed === 0) {
      console.log('🎉 All Context Management UI tests completed successfully!');
    } else {
      console.log(`⚠️ ${summary.failed} test(s) failed. Check test-results for details.`);
    }

  } catch (error) {
    console.error('❌ Error generating test report:', error);
  }
}

/**
 * Clean up temporary files (only in CI environments)
 */
async function cleanupTemporaryFiles(): Promise<void> {
  const CI = process.env.CI === 'true';
  
  if (!CI) {
    console.log('🏠 Keeping temporary files for local development');
    return;
  }

  console.log('🧹 Cleaning up temporary files (CI mode)...');

  const tempFiles = [
    'test-results/test-data.json',
    'test-results/environment.json',
  ];

  const tempDirs = [
    'test-results/context-artifacts',
  ];

  // Clean up temporary files
  for (const file of tempFiles) {
    try {
      await fs.unlink(file);
      console.log(`🗑️ Removed ${file}`);
    } catch (error) {
      // File might not exist, which is fine
    }
  }

  // Clean up empty temporary directories
  for (const dir of tempDirs) {
    try {
      const files = await fs.readdir(dir);
      if (files.length === 0) {
        await fs.rmdir(dir);
        console.log(`🗑️ Removed empty directory ${dir}`);
      }
    } catch (error) {
      // Directory might not exist or not be empty, which is fine
    }
  }

  console.log('✅ Temporary file cleanup completed');
}

/**
 * Validate test environment health
 */
async function validateEnvironmentHealth(): Promise<void> {
  console.log('🏥 Validating environment health...');

  try {
    // Check if any services are still running that shouldn't be
    const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
    const MCP_SERVER_PORT = process.env.MCP_SERVER_PORT || '8001';

    for (const port of [BACKEND_PORT, MCP_SERVER_PORT]) {
      try {
        const { stdout } = await execAsync(`lsof -ti:${port} 2>/dev/null || echo ""`);
        const pids = stdout.trim().split('\n').filter(pid => pid);
        
        if (pids.length > 0) {
          console.warn(`⚠️ Port ${port} still has processes running: ${pids.join(', ')}`);
        } else {
          console.log(`✅ Port ${port} is clean`);
        }
      } catch (error) {
        console.log(`✅ Port ${port} appears clean`);
      }
    }

    // Check disk space usage
    try {
      const { stdout } = await execAsync('df -h test-results/ 2>/dev/null || echo "N/A"');
      console.log('💾 Disk usage:', stdout.split('\n')[1] || 'Unknown');
    } catch (error) {
      console.log('💾 Could not check disk usage');
    }

    console.log('✅ Environment health validation completed');

  } catch (error) {
    console.error('❌ Environment health validation failed:', error);
  }
}

/**
 * Main global teardown function
 */
async function globalTeardown(config: FullConfig, result: FullResult): Promise<void> {
  console.log('🏁 Starting Context Management UI Test Global Teardown');

  try {
    // Shutdown services gracefully
    await shutdownServices();

    // Collect test artifacts
    const artifacts = await collectTestArtifacts();

    // Generate test report
    await generateTestReport(result, artifacts);

    // Validate environment health
    await validateEnvironmentHealth();

    // Clean up temporary files if in CI
    await cleanupTemporaryFiles();

    console.log('✅ Global teardown completed successfully');

  } catch (error) {
    console.error('❌ Global teardown encountered errors:', error);
  } finally {
    console.log('👋 Context Management UI Test Suite finished');
  }
}

export default globalTeardown;