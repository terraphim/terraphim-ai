#!/usr/bin/env node

/**
 * Terraphim AI - Browser Automation Test Suite
 *
 * This script performs end-to-end browser automation testing of all workflow examples
 * to ensure the real multi-agent API integration is working properly.
 *
 * Requirements:
 * - Node.js 18+
 * - Playwright (installed automatically if missing)
 * - Running Terraphim backend server
 *
 * Usage:
 *   node browser-automation-tests.js
 *
 * Environment Variables:
 *   BACKEND_URL - Backend server URL (default: http://localhost:8000)
 *   HEADLESS - Run in headless mode (default: true)
 *   TIMEOUT - Test timeout in ms (default: 60000)
 *   SCREENSHOT_ON_FAILURE - Take screenshots on failure (default: true)
 */

const { chromium } = require('playwright');
const fs = require('fs').promises;
const path = require('path');

class BrowserAutomationTestSuite {
    constructor(options = {}) {
        this.options = {
            backendUrl: options.backendUrl || process.env.BACKEND_URL || 'http://localhost:8000',
            headless: options.headless !== undefined ? options.headless : (process.env.HEADLESS !== 'false'),
            timeout: options.timeout || parseInt(process.env.TIMEOUT) || 60000,
            screenshotOnFailure: options.screenshotOnFailure !== undefined ? options.screenshotOnFailure : (process.env.SCREENSHOT_ON_FAILURE !== 'false'),
            slowMo: options.slowMo || 0,
            devtools: options.devtools || false
        };

        this.browser = null;
        this.context = null;
        this.results = {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            tests: []
        };

        this.workflows = [
            {
                name: 'Prompt Chaining',
                path: '1-prompt-chaining/index.html',
                testId: 'prompt-chain',
                description: 'Multi-step software development workflow'
            },
            {
                name: 'Routing',
                path: '2-routing/index.html',
                testId: 'routing',
                description: 'Smart agent selection based on complexity'
            },
            {
                name: 'Parallelization',
                path: '3-parallelization/index.html',
                testId: 'parallel',
                description: 'Multi-perspective analysis with aggregation'
            },
            {
                name: 'Orchestrator-Workers',
                path: '4-orchestrator-workers/index.html',
                testId: 'orchestration',
                description: 'Hierarchical task decomposition'
            },
            {
                name: 'Evaluator-Optimizer',
                path: '5-evaluator-optimizer/index.html',
                testId: 'optimization',
                description: 'Iterative content improvement'
            }
        ];
    }

    async initialize() {
        console.log('🚀 Initializing Browser Automation Test Suite');
        console.log(`Backend URL: ${this.options.backendUrl}`);
        console.log(`Headless: ${this.options.headless}`);
        console.log(`Timeout: ${this.options.timeout}ms`);

        try {
            // Launch browser
            this.browser = await chromium.launch({
                headless: this.options.headless,
                slowMo: this.options.slowMo,
                devtools: this.options.devtools,
                args: ['--no-sandbox', '--disable-setuid-sandbox']
            });

            // Create context with device emulation
            this.context = await this.browser.newContext({
                viewport: { width: 1920, height: 1080 },
                userAgent: 'TerraphimAI-AutomationTest/1.0',
                permissions: ['clipboard-read', 'clipboard-write'],
                colorScheme: 'light'
            });

            // Enable request/response logging for debugging
            this.context.on('request', request => {
                if (request.url().includes('/workflows/') || request.url().includes('/health')) {
                    console.log(`📤 ${request.method()} ${request.url()}`);
                }
            });

            this.context.on('response', response => {
                if (response.url().includes('/workflows/') || response.url().includes('/health')) {
                    console.log(`📥 ${response.status()} ${response.url()}`);
                }
            });

            console.log('✅ Browser initialized successfully');
        } catch (error) {
            console.error('❌ Failed to initialize browser:', error);
            throw error;
        }
    }

    async cleanup() {
        try {
            if (this.context) {
                await this.context.close();
            }
            if (this.browser) {
                await this.browser.close();
            }
            console.log('🧹 Browser cleanup completed');
        } catch (error) {
            console.error('⚠️ Cleanup error:', error);
        }
    }

    async runAllTests() {
        const startTime = Date.now();

        try {
            await this.initialize();

            // First verify backend is available
            await this.testBackendHealth();

            // Test comprehensive test suite page
            await this.testComprehensiveTestSuite();

            // Test individual workflow examples
            for (const workflow of this.workflows) {
                await this.testWorkflowExample(workflow);
            }

            // Generate summary report
            const duration = Date.now() - startTime;
            await this.generateReport(duration);

        } catch (error) {
            console.error('💥 Test suite failed:', error);
            this.results.failed++;
        } finally {
            await this.cleanup();
        }

        return this.results;
    }

    async testBackendHealth() {
        const testName = 'Backend Health Check';
        console.log(`\n🔍 Testing: ${testName}`);

        try {
            const page = await this.context.newPage();

            // Test direct API endpoint
            const response = await page.request.get(`${this.options.backendUrl}/health`);

            if (response.ok()) {
                console.log('✅ Backend is healthy and responding');
                this.recordTestResult(testName, 'passed', { status: response.status() });
            } else {
                throw new Error(`Backend health check failed: ${response.status()}`);
            }

            await page.close();

        } catch (error) {
            console.error(`❌ ${testName} failed:`, error.message);
            this.recordTestResult(testName, 'failed', { error: error.message });
        }
    }

    async testComprehensiveTestSuite() {
        const testName = 'Comprehensive Test Suite Page';
        console.log(`\n🔍 Testing: ${testName}`);

        try {
            const page = await this.context.newPage();

            // Load test suite page
            await page.goto('file://' + path.resolve(__dirname, 'test-all-workflows.html'), {
                waitUntil: 'networkidle'
            });

            // Wait for page to initialize
            await page.waitForSelector('#run-all-tests', { timeout: 10000 });

            // Check server connection status
            await page.waitForSelector('#health-status', { timeout: 5000 });
            const healthStatus = await page.textContent('#health-status');

            if (!healthStatus.includes('✅')) {
                console.warn('⚠️ Server connection status shows issues');
            }

            // Click run all tests button
            await page.click('#run-all-tests');

            // Wait for tests to start
            await page.waitForSelector('.workflow-test.testing', { timeout: 5000 });
            console.log('🏃 Tests are running...');

            // Wait for completion or timeout
            try {
                await page.waitForSelector('#summary-results[style*="block"]', {
                    timeout: this.options.timeout
                });
            } catch (timeoutError) {
                console.warn('⏰ Tests did not complete within timeout, checking current status');
            }

            // Get test results
            const testElements = await page.$$('.workflow-test');
            const testResults = [];

            for (const element of testElements) {
                const testId = await element.getAttribute('id');
                const statusElement = await element.$('.status-indicator');
                const statusClass = await statusElement.getAttribute('class');
                const resultText = await element.$eval('.test-results', el => el.textContent);

                testResults.push({
                    id: testId,
                    status: statusClass.includes('success') ? 'passed' :
                           statusClass.includes('error') ? 'failed' :
                           statusClass.includes('testing') ? 'running' : 'pending',
                    result: resultText.substring(0, 200) + '...'
                });
            }

            console.log('📊 Test Results Summary:');
            testResults.forEach(result => {
                const icon = result.status === 'passed' ? '✅' :
                           result.status === 'failed' ? '❌' :
                           result.status === 'running' ? '🏃' : '⏸️';
                console.log(`  ${icon} ${result.id}: ${result.status}`);
            });

            // Take screenshot for documentation
            if (this.options.screenshotOnFailure) {
                await this.takeScreenshot(page, 'comprehensive-test-suite');
            }

            const passedTests = testResults.filter(r => r.status === 'passed').length;
            const failedTests = testResults.filter(r => r.status === 'failed').length;

            this.recordTestResult(testName, passedTests > failedTests ? 'passed' : 'failed', {
                totalTests: testResults.length,
                passed: passedTests,
                failed: failedTests,
                details: testResults
            });

            await page.close();

        } catch (error) {
            console.error(`❌ ${testName} failed:`, error.message);
            this.recordTestResult(testName, 'failed', { error: error.message });
        }
    }

    async testWorkflowExample(workflow) {
        const testName = `Workflow: ${workflow.name}`;
        console.log(`\n🔍 Testing: ${testName}`);

        try {
            const page = await this.context.newPage();

            // Load workflow example page
            const filePath = path.resolve(__dirname, workflow.path);
            await page.goto(`file://${filePath}`, { waitUntil: 'networkidle' });

            // Wait for page to load completely
            await page.waitForLoadState('domcontentloaded');
            await page.waitForTimeout(2000); // Allow time for settings initialization

            // Check if settings integration is working
            const hasApiClient = await page.evaluate(() => {
                return window.apiClient !== null && window.apiClient !== undefined;
            });

            if (!hasApiClient) {
                console.warn('⚠️ API client not initialized, may affect test results');
            }

            // Find and click the primary action button
            const actionButtons = [
                '#start-chain', '#start-routing', '#start-analysis',
                '#start-pipeline', '#start-optimization', '.btn-primary'
            ];

            let actionButton = null;
            for (const selector of actionButtons) {
                try {
                    actionButton = await page.$(selector);
                    if (actionButton) {
                        const isVisible = await actionButton.isVisible();
                        if (isVisible) break;
                    }
                } catch (e) {
                    // Continue searching
                }
            }

            if (!actionButton) {
                throw new Error('Could not find primary action button');
            }

            // Click the action button to start workflow
            await actionButton.click();
            console.log('▶️ Started workflow execution');

            // Wait for workflow to show progress
            await page.waitForTimeout(3000);

            // Look for progress indicators or results
            const hasProgress = await page.evaluate(() => {
                // Check for common progress indicators
                const progressSelectors = [
                    '.progress-bar', '.workflow-progress', '.step-progress',
                    '[class*="progress"]', '[id*="progress"]', '.visualizer'
                ];

                return progressSelectors.some(selector => {
                    const elements = document.querySelectorAll(selector);
                    return elements.length > 0;
                });
            });

            // Look for API calls in network tab
            const apiCalls = [];
            page.on('response', response => {
                if (response.url().includes('/workflows/')) {
                    apiCalls.push({
                        url: response.url(),
                        status: response.status(),
                        method: response.request().method()
                    });
                }
            });

            // Wait a bit longer for API calls to complete
            await page.waitForTimeout(5000);

            // Check for error messages
            const hasErrors = await page.evaluate(() => {
                const errorText = document.body.textContent.toLowerCase();
                return errorText.includes('error') ||
                       errorText.includes('failed') ||
                       errorText.includes('timeout');
            });

            // Take screenshot for documentation
            if (this.options.screenshotOnFailure) {
                await this.takeScreenshot(page, `workflow-${workflow.testId}`);
            }

            // Evaluate test success
            const testPassed = hasProgress && !hasErrors && (apiCalls.length > 0 || hasApiClient);

            if (testPassed) {
                console.log('✅ Workflow example is functioning');
            } else {
                console.log('⚠️ Workflow example may have issues');
            }

            this.recordTestResult(testName, testPassed ? 'passed' : 'failed', {
                hasProgress,
                hasErrors,
                apiCalls: apiCalls.length,
                hasApiClient,
                url: filePath
            });

            await page.close();

        } catch (error) {
            console.error(`❌ ${testName} failed:`, error.message);

            // Take screenshot on failure
            if (this.options.screenshotOnFailure) {
                try {
                    const page = await this.context.newPage();
                    await page.goto(`file://${path.resolve(__dirname, workflow.path)}`);
                    await this.takeScreenshot(page, `error-${workflow.testId}`);
                    await page.close();
                } catch (screenshotError) {
                    console.warn('Could not take error screenshot:', screenshotError.message);
                }
            }

            this.recordTestResult(testName, 'failed', { error: error.message });
        }
    }

    async takeScreenshot(page, name) {
        try {
            const screenshotDir = path.resolve(__dirname, 'test-screenshots');
            await fs.mkdir(screenshotDir, { recursive: true });

            const filename = `${name}-${Date.now()}.png`;
            const filepath = path.join(screenshotDir, filename);

            await page.screenshot({
                path: filepath,
                fullPage: true,
                type: 'png'
            });

            console.log(`📸 Screenshot saved: ${filename}`);
            return filepath;
        } catch (error) {
            console.warn('Failed to take screenshot:', error.message);
        }
    }

    recordTestResult(testName, status, details = {}) {
        this.results.total++;

        if (status === 'passed') {
            this.results.passed++;
        } else if (status === 'failed') {
            this.results.failed++;
        } else {
            this.results.skipped++;
        }

        this.results.tests.push({
            name: testName,
            status,
            timestamp: new Date().toISOString(),
            details
        });
    }

    async generateReport(duration) {
        console.log('\n📋 Test Execution Report');
        console.log('=' .repeat(50));
        console.log(`Duration: ${Math.round(duration / 1000)}s`);
        console.log(`Total Tests: ${this.results.total}`);
        console.log(`✅ Passed: ${this.results.passed}`);
        console.log(`❌ Failed: ${this.results.failed}`);
        console.log(`⏸️ Skipped: ${this.results.skipped}`);
        console.log(`📊 Success Rate: ${Math.round((this.results.passed / this.results.total) * 100)}%`);

        console.log('\n📝 Detailed Results:');
        this.results.tests.forEach(test => {
            const icon = test.status === 'passed' ? '✅' : test.status === 'failed' ? '❌' : '⏸️';
            console.log(`  ${icon} ${test.name}`);

            if (test.details.error) {
                console.log(`      Error: ${test.details.error}`);
            }

            if (test.details.apiCalls !== undefined) {
                console.log(`      API Calls: ${test.details.apiCalls}`);
            }

            if (test.details.totalTests) {
                console.log(`      Subtests: ${test.details.passed}/${test.details.totalTests} passed`);
            }
        });

        // Save JSON report
        const reportPath = path.resolve(__dirname, 'test-results.json');
        await fs.writeFile(reportPath, JSON.stringify({
            ...this.results,
            duration,
            timestamp: new Date().toISOString(),
            options: this.options
        }, null, 2));

        console.log(`\n💾 Detailed report saved: ${reportPath}`);

        // Generate HTML report
        await this.generateHtmlReport(duration);
    }

    async generateHtmlReport(duration) {
        const htmlReport = `
<!DOCTYPE html>
<html>
<head>
    <title>Terraphim AI - Browser Automation Test Report</title>
    <style>
        body { font-family: -apple-system, system-ui, sans-serif; margin: 2rem; }
        .header { text-align: center; margin-bottom: 2rem; }
        .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
        .summary-card { background: #f5f5f5; padding: 1rem; border-radius: 8px; text-align: center; }
        .summary-card h3 { margin: 0; color: #666; font-size: 0.9rem; }
        .summary-card .number { font-size: 2rem; font-weight: bold; margin: 0.5rem 0; }
        .passed .number { color: #22c55e; }
        .failed .number { color: #ef4444; }
        .total .number { color: #3b82f6; }
        .test-results { margin-top: 2rem; }
        .test-item { border: 1px solid #e5e5e5; margin-bottom: 1rem; border-radius: 8px; overflow: hidden; }
        .test-header { padding: 1rem; background: #f9f9f9; display: flex; align-items: center; gap: 0.5rem; }
        .test-status { width: 20px; height: 20px; border-radius: 50%; }
        .status-passed { background: #22c55e; }
        .status-failed { background: #ef4444; }
        .status-skipped { background: #6b7280; }
        .test-details { padding: 1rem; font-size: 0.9rem; color: #666; }
        .test-details pre { background: #f5f5f5; padding: 0.5rem; border-radius: 4px; overflow-x: auto; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🧪 Terraphim AI Browser Automation Test Report</h1>
        <p>Generated on ${new Date().toLocaleString()}</p>
        <p>Duration: ${Math.round(duration / 1000)}s</p>
    </div>

    <div class="summary">
        <div class="summary-card total">
            <h3>Total Tests</h3>
            <div class="number">${this.results.total}</div>
        </div>
        <div class="summary-card passed">
            <h3>Passed</h3>
            <div class="number">${this.results.passed}</div>
        </div>
        <div class="summary-card failed">
            <h3>Failed</h3>
            <div class="number">${this.results.failed}</div>
        </div>
        <div class="summary-card">
            <h3>Success Rate</h3>
            <div class="number">${Math.round((this.results.passed / this.results.total) * 100)}%</div>
        </div>
    </div>

    <div class="test-results">
        <h2>Test Results</h2>
        ${this.results.tests.map(test => `
            <div class="test-item">
                <div class="test-header">
                    <div class="test-status status-${test.status}"></div>
                    <strong>${test.name}</strong>
                    <span style="margin-left: auto; color: #666;">${new Date(test.timestamp).toLocaleTimeString()}</span>
                </div>
                ${test.details.error || test.details.apiCalls !== undefined || test.details.totalTests ? `
                    <div class="test-details">
                        ${test.details.error ? `<div><strong>Error:</strong> ${test.details.error}</div>` : ''}
                        ${test.details.apiCalls !== undefined ? `<div><strong>API Calls:</strong> ${test.details.apiCalls}</div>` : ''}
                        ${test.details.totalTests ? `<div><strong>Subtests:</strong> ${test.details.passed}/${test.details.totalTests} passed</div>` : ''}
                        ${test.details.details ? `<pre>${JSON.stringify(test.details.details, null, 2)}</pre>` : ''}
                    </div>
                ` : ''}
            </div>
        `).join('')}
    </div>

    <div style="margin-top: 2rem; padding: 1rem; background: #f5f5f5; border-radius: 8px;">
        <h3>Test Configuration</h3>
        <pre>${JSON.stringify(this.options, null, 2)}</pre>
    </div>
</body>
</html>
        `;

        const htmlReportPath = path.resolve(__dirname, 'test-report.html');
        await fs.writeFile(htmlReportPath, htmlReport.trim());
        console.log(`📄 HTML report saved: ${htmlReportPath}`);
    }
}

// CLI Interface
async function main() {
    console.log('🎭 Terraphim AI - Browser Automation Test Suite');
    console.log('Testing multi-agent workflow integration end-to-end\n');

    const testSuite = new BrowserAutomationTestSuite();

    try {
        const results = await testSuite.runAllTests();

        // Exit with appropriate code
        const exitCode = results.failed > 0 ? 1 : 0;
        console.log(`\n🏁 Tests completed with exit code: ${exitCode}`);
        process.exit(exitCode);

    } catch (error) {
        console.error('💥 Test suite crashed:', error);
        process.exit(1);
    }
}

// Handle SIGINT gracefully
process.on('SIGINT', () => {
    console.log('\n⏹️ Tests interrupted by user');
    process.exit(130);
});

// Run if called directly
if (require.main === module) {
    main().catch(error => {
        console.error('Fatal error:', error);
        process.exit(1);
    });
}

module.exports = { BrowserAutomationTestSuite };
