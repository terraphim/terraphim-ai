/**
 * Test Runner Configuration for Context Management System
 *
 * Orchestrates the execution of all test suites including unit tests,
 * integration tests, E2E tests, and performance tests. Provides
 * comprehensive test coverage validation and reporting.
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import fs from 'fs/promises';

const execAsync = promisify(exec);

interface TestSuite {
  name: string;
  description: string;
  command: string;
  workingDirectory: string;
  timeout: number;
  required: boolean;
  tags?: string[];
}

interface TestResult {
  suite: string;
  status: 'passed' | 'failed' | 'skipped';
  duration: number;
  output: string;
  error?: string;
  coverage?: {
    statements: number;
    branches: number;
    functions: number;
    lines: number;
  };
}

interface TestRunConfig {
  parallel: boolean;
  continueOnFailure: boolean;
  generateReport: boolean;
  includeCoverage: boolean;
  tags: string[];
  outputDir: string;
}

// Test suite definitions
const TEST_SUITES: TestSuite[] = [
  // Backend unit tests
  {
    name: 'backend-unit-context',
    description: 'Backend unit tests for ContextManager service',
    command: 'cargo test context_tests --lib -p terraphim_service',
    workingDirectory: path.resolve(__dirname, '../../..'),
    timeout: 120000,
    required: true,
    tags: ['backend', 'unit', 'context']
  },

  // API integration tests
  {
    name: 'api-integration-context',
    description: 'API integration tests for context endpoints',
    command: 'cargo test api_context_tests --test "*"',
    workingDirectory: path.resolve(__dirname, '../../../terraphim_server'),
    timeout: 180000,
    required: true,
    tags: ['api', 'integration', 'context']
  },

  // Tauri command tests
  {
    name: 'tauri-command-context',
    description: 'Tauri command tests for desktop integration',
    command: 'cargo test tauri_context_tests --test "*"',
    workingDirectory: path.resolve(__dirname, '../src-tauri'),
    timeout: 120000,
    required: true,
    tags: ['tauri', 'desktop', 'commands']
  },

  // Frontend unit tests
  {
    name: 'frontend-unit',
    description: 'Frontend unit tests for Svelte components',
    command: 'yarn test:unit',
    workingDirectory: path.resolve(__dirname, '..'),
    timeout: 90000,
    required: false,
    tags: ['frontend', 'unit', 'svelte']
  },

  // E2E context management tests
  {
    name: 'e2e-context-management',
    description: 'End-to-end tests for context management UI',
    command: 'npx playwright test --config playwright-context.config.ts context-management.spec.ts',
    workingDirectory: path.resolve(__dirname, '..'),
    timeout: 300000,
    required: true,
    tags: ['e2e', 'context', 'ui']
  },

  // E2E workflow integration tests
  {
    name: 'e2e-workflow-integration',
    description: 'End-to-end workflow integration tests',
    command: 'npx playwright test --config playwright-context.config.ts workflow-integration.spec.ts',
    workingDirectory: path.resolve(__dirname, '..'),
    timeout: 360000,
    required: true,
    tags: ['e2e', 'workflow', 'integration']
  },

  // Performance tests
  {
    name: 'performance-stress',
    description: 'Performance and stress tests',
    command: 'npx playwright test --config playwright-context.config.ts performance-stress.spec.ts --grep @performance',
    workingDirectory: path.resolve(__dirname, '..'),
    timeout: 600000,
    required: false,
    tags: ['performance', 'stress', 'benchmark']
  },

  // Accessibility tests
  {
    name: 'accessibility',
    description: 'Accessibility compliance tests',
    command: 'npx playwright test --config playwright-context.config.ts --grep @accessibility',
    workingDirectory: path.resolve(__dirname, '..'),
    timeout: 180000,
    required: false,
    tags: ['accessibility', 'a11y', 'compliance']
  },

  // Visual regression tests
  {
    name: 'visual-regression',
    description: 'Visual regression tests',
    command: 'npx playwright test --config playwright-context.config.ts --grep @visual',
    workingDirectory: path.resolve(__dirname, '..'),
    timeout: 240000,
    required: false,
    tags: ['visual', 'regression', 'screenshots']
  }
];

// Default configuration
const DEFAULT_CONFIG: TestRunConfig = {
  parallel: false,
  continueOnFailure: true,
  generateReport: true,
  includeCoverage: false,
  tags: [],
  outputDir: 'test-results/comprehensive'
};

/**
 * Filter test suites based on configuration
 */
function filterTestSuites(suites: TestSuite[], config: TestRunConfig): TestSuite[] {
  if (config.tags.length === 0) {
    return suites;
  }

  return suites.filter(suite => {
    if (!suite.tags) return false;
    return config.tags.some(tag => suite.tags?.includes(tag));
  });
}

/**
 * Run a single test suite
 */
async function runTestSuite(suite: TestSuite): Promise<TestResult> {
  console.log(`🚀 Running ${suite.name}: ${suite.description}`);

  const startTime = Date.now();

  try {
    const { stdout, stderr } = await execAsync(suite.command, {
      cwd: suite.workingDirectory,
      timeout: suite.timeout,
      env: {
        ...process.env,
        RUST_LOG: 'warn',
        NODE_ENV: 'test'
      }
    });

    const endTime = Date.now();
    const duration = endTime - startTime;

    console.log(`✅ ${suite.name} passed (${duration}ms)`);

    return {
      suite: suite.name,
      status: 'passed',
      duration,
      output: stdout
    };

  } catch (error: any) {
    const endTime = Date.now();
    const duration = endTime - startTime;

    console.log(`❌ ${suite.name} failed (${duration}ms)`);
    console.log(`Error: ${error.message}`);

    return {
      suite: suite.name,
      status: 'failed',
      duration,
      output: error.stdout || '',
      error: error.message
    };
  }
}

/**
 * Run multiple test suites in parallel
 */
async function runTestSuitesParallel(suites: TestSuite[]): Promise<TestResult[]> {
  console.log(`🔄 Running ${suites.length} test suites in parallel...`);

  const promises = suites.map(suite => runTestSuite(suite));
  return Promise.all(promises);
}

/**
 * Run test suites sequentially
 */
async function runTestSuitesSequential(suites: TestSuite[], config: TestRunConfig): Promise<TestResult[]> {
  console.log(`🔄 Running ${suites.length} test suites sequentially...`);

  const results: TestResult[] = [];

  for (const suite of suites) {
    const result = await runTestSuite(suite);
    results.push(result);

    // Stop on first failure if not continuing on failure
    if (!config.continueOnFailure && result.status === 'failed' && suite.required) {
      console.log(`🛑 Stopping test execution due to required test failure: ${suite.name}`);
      break;
    }
  }

  return results;
}

/**
 * Generate comprehensive test report
 */
async function generateTestReport(results: TestResult[], config: TestRunConfig): Promise<void> {
  console.log('📊 Generating comprehensive test report...');

  // Create output directory
  await fs.mkdir(config.outputDir, { recursive: true });

  // Calculate summary statistics
  const totalTests = results.length;
  const passedTests = results.filter(r => r.status === 'passed').length;
  const failedTests = results.filter(r => r.status === 'failed').length;
  const skippedTests = results.filter(r => r.status === 'skipped').length;

  const totalDuration = results.reduce((sum, r) => sum + r.duration, 0);
  const averageDuration = totalDuration / totalTests;

  // Generate JSON report
  const jsonReport = {
    timestamp: new Date().toISOString(),
    config,
    summary: {
      total: totalTests,
      passed: passedTests,
      failed: failedTests,
      skipped: skippedTests,
      successRate: (passedTests / totalTests) * 100,
      totalDuration,
      averageDuration
    },
    results
  };

  await fs.writeFile(
    path.join(config.outputDir, 'comprehensive-test-report.json'),
    JSON.stringify(jsonReport, null, 2)
  );

  // Generate HTML report
  const htmlReport = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Context Management Test Results</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; }
        .header { border-bottom: 2px solid #eee; margin-bottom: 20px; padding-bottom: 20px; }
        .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin-bottom: 30px; }
        .metric { background: #f8f9fa; padding: 15px; border-radius: 6px; text-align: center; }
        .metric-value { font-size: 2em; font-weight: bold; margin-bottom: 5px; }
        .passed { color: #28a745; }
        .failed { color: #dc3545; }
        .skipped { color: #ffc107; }
        .suite-result { margin-bottom: 20px; border: 1px solid #ddd; border-radius: 6px; overflow: hidden; }
        .suite-header { padding: 15px; background: #f8f9fa; border-bottom: 1px solid #ddd; }
        .suite-content { padding: 15px; }
        .status-badge { padding: 4px 8px; border-radius: 4px; color: white; font-size: 0.8em; }
        .status-passed { background-color: #28a745; }
        .status-failed { background-color: #dc3545; }
        .status-skipped { background-color: #ffc107; }
        .error-output { background: #f8d7da; color: #721c24; padding: 10px; border-radius: 4px; margin-top: 10px; }
        .output { background: #d4edda; color: #155724; padding: 10px; border-radius: 4px; margin-top: 10px; max-height: 200px; overflow-y: auto; }
        pre { margin: 0; white-space: pre-wrap; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Context Management System - Comprehensive Test Results</h1>
            <p><strong>Generated:</strong> ${new Date().toLocaleString()}</p>
            <p><strong>Configuration:</strong> ${JSON.stringify(config, null, 2)}</p>
        </div>

        <div class="summary">
            <div class="metric">
                <div class="metric-value">${totalTests}</div>
                <div>Total Tests</div>
            </div>
            <div class="metric">
                <div class="metric-value passed">${passedTests}</div>
                <div>Passed</div>
            </div>
            <div class="metric">
                <div class="metric-value failed">${failedTests}</div>
                <div>Failed</div>
            </div>
            <div class="metric">
                <div class="metric-value skipped">${skippedTests}</div>
                <div>Skipped</div>
            </div>
            <div class="metric">
                <div class="metric-value">${((passedTests / totalTests) * 100).toFixed(1)}%</div>
                <div>Success Rate</div>
            </div>
            <div class="metric">
                <div class="metric-value">${(totalDuration / 1000).toFixed(1)}s</div>
                <div>Total Duration</div>
            </div>
        </div>

        <div class="results">
            <h2>Detailed Results</h2>
            ${results.map(result => `
                <div class="suite-result">
                    <div class="suite-header">
                        <h3>${result.suite}
                            <span class="status-badge status-${result.status}">${result.status.toUpperCase()}</span>
                        </h3>
                        <p><strong>Duration:</strong> ${(result.duration / 1000).toFixed(2)}s</p>
                    </div>
                    <div class="suite-content">
                        ${result.error ? `
                            <div class="error-output">
                                <strong>Error:</strong>
                                <pre>${result.error}</pre>
                            </div>
                        ` : ''}
                        ${result.output ? `
                            <div class="output">
                                <strong>Output:</strong>
                                <pre>${result.output.substring(0, 2000)}${result.output.length > 2000 ? '...' : ''}</pre>
                            </div>
                        ` : ''}
                    </div>
                </div>
            `).join('')}
        </div>
    </div>
</body>
</html>`;

  await fs.writeFile(
    path.join(config.outputDir, 'comprehensive-test-report.html'),
    htmlReport
  );

  // Generate summary report
  const summaryReport = `
# Context Management System - Test Summary

**Generated:** ${new Date().toLocaleString()}

## Summary
- **Total Tests:** ${totalTests}
- **Passed:** ${passedTests} ✅
- **Failed:** ${failedTests} ❌
- **Skipped:** ${skippedTests} ⏭️
- **Success Rate:** ${((passedTests / totalTests) * 100).toFixed(1)}%
- **Total Duration:** ${(totalDuration / 1000).toFixed(1)}s

## Test Results by Suite

${results.map(result => `
### ${result.suite}
- **Status:** ${result.status === 'passed' ? '✅ PASSED' : result.status === 'failed' ? '❌ FAILED' : '⏭️ SKIPPED'}
- **Duration:** ${(result.duration / 1000).toFixed(2)}s
${result.error ? `- **Error:** ${result.error}` : ''}
`).join('')}

## Overall Status
${failedTests === 0 ? '🎉 **ALL TESTS PASSED!**' : `⚠️ **${failedTests} TEST(S) FAILED**`}

---
*Generated by Context Management Test Runner*
`;

  await fs.writeFile(
    path.join(config.outputDir, 'test-summary.md'),
    summaryReport
  );

  console.log(`✅ Test reports generated in ${config.outputDir}/`);
}

/**
 * Main test runner function
 */
async function runComprehensiveTests(userConfig: Partial<TestRunConfig> = {}): Promise<void> {
  const config: TestRunConfig = { ...DEFAULT_CONFIG, ...userConfig };

  console.log('🚀 Starting Comprehensive Context Management Test Suite');
  console.log(`📋 Configuration:`, config);

  // Filter test suites based on configuration
  const filteredSuites = filterTestSuites(TEST_SUITES, config);

  console.log(`📝 Running ${filteredSuites.length} test suites:`);
  filteredSuites.forEach(suite => {
    console.log(`  - ${suite.name}: ${suite.description} ${suite.required ? '(required)' : '(optional)'}`);
  });

  // Run tests
  const startTime = Date.now();
  let results: TestResult[];

  if (config.parallel && filteredSuites.length > 1) {
    results = await runTestSuitesParallel(filteredSuites);
  } else {
    results = await runTestSuitesSequential(filteredSuites, config);
  }

  const endTime = Date.now();
  const totalDuration = endTime - startTime;

  // Generate reports
  if (config.generateReport) {
    await generateTestReport(results, config);
  }

  // Print summary
  const passedCount = results.filter(r => r.status === 'passed').length;
  const failedCount = results.filter(r => r.status === 'failed').length;
  const skippedCount = results.filter(r => r.status === 'skipped').length;

  console.log('\n📊 Test Execution Summary:');
  console.log(`⏱️  Total Duration: ${(totalDuration / 1000).toFixed(1)}s`);
  console.log(`✅ Passed: ${passedCount}/${results.length}`);
  console.log(`❌ Failed: ${failedCount}/${results.length}`);
  console.log(`⏭️  Skipped: ${skippedCount}/${results.length}`);
  console.log(`📈 Success Rate: ${((passedCount / results.length) * 100).toFixed(1)}%`);

  // Check for required test failures
  const requiredFailures = results.filter(r =>
    r.status === 'failed' &&
    filteredSuites.find(s => s.name === r.suite)?.required
  );

  if (requiredFailures.length > 0) {
    console.log(`\n❌ CRITICAL: ${requiredFailures.length} required test(s) failed:`);
    requiredFailures.forEach(failure => {
      console.log(`  - ${failure.suite}`);
    });
    process.exit(1);
  } else if (failedCount > 0) {
    console.log(`\n⚠️  ${failedCount} optional test(s) failed, but all required tests passed.`);
  } else {
    console.log('\n🎉 ALL TESTS PASSED!');
  }
}

// CLI interface
if (require.main === module) {
  const args = process.argv.slice(2);

  const config: Partial<TestRunConfig> = {};

  // Parse command line arguments
  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--parallel':
        config.parallel = true;
        break;
      case '--sequential':
        config.parallel = false;
        break;
      case '--stop-on-failure':
        config.continueOnFailure = false;
        break;
      case '--continue-on-failure':
        config.continueOnFailure = true;
        break;
      case '--no-report':
        config.generateReport = false;
        break;
      case '--coverage':
        config.includeCoverage = true;
        break;
      case '--tags':
        if (i + 1 < args.length) {
          config.tags = args[i + 1].split(',');
          i++;
        }
        break;
      case '--output-dir':
        if (i + 1 < args.length) {
          config.outputDir = args[i + 1];
          i++;
        }
        break;
      case '--help':
        console.log(`
Usage: node test-runner-config.js [options]

Options:
  --parallel              Run tests in parallel (default: false)
  --sequential            Run tests sequentially (default: true)
  --stop-on-failure       Stop on first failure (default: false)
  --continue-on-failure   Continue on failures (default: true)
  --no-report             Skip report generation (default: false)
  --coverage              Include coverage reporting (default: false)
  --tags <tags>           Run only tests with specified tags (comma-separated)
  --output-dir <dir>      Output directory for reports (default: test-results/comprehensive)
  --help                  Show this help message

Available tags: backend, frontend, api, tauri, unit, integration, e2e, performance, accessibility, visual

Examples:
  node test-runner-config.js --parallel --tags backend,api
  node test-runner-config.js --sequential --stop-on-failure --coverage
  node test-runner-config.js --tags e2e,performance --output-dir custom-results
`);
        process.exit(0);
    }
  }

  // Run tests
  runComprehensiveTests(config).catch(error => {
    console.error('❌ Test runner failed:', error);
    process.exit(1);
  });
}

export { runComprehensiveTests, TEST_SUITES, TestRunConfig, TestResult };
