const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

class RefactoredWorkflowsTestSuite {
    constructor() {
        this.browser = null;
        this.page = null;
        this.results = [];
        this.screenshotsDir = path.join(__dirname, 'test-screenshots-refactored');
        if (!fs.existsSync(this.screenshotsDir)) {
            fs.mkdirSync(this.screenshotsDir);
        }
    }

    async initialize() {
        console.log('Initializing test suite...');
        this.browser = await chromium.launch({ headless: true });
        this.page = await this.browser.newPage();
        console.log('Browser launched.');
    }

    async cleanup() {
        if (this.browser) {
            await this.browser.close();
            console.log('Browser closed.');
        }
    }

    async runTests() {
        try {
            await this.initialize();

            await this.testPromptChaining();
            await this.testRouting();
            await this.testParallelization();
            await this.testOrchestration();
            await this.testOptimization();

            this.generateReport();
        } catch (error) {
            console.error('An error occurred during the test run:', error);
        } finally {
            await this.cleanup();
        }
    }

    async testExample(name, url, startButtonSelector, outputSelector) {
        console.log(`--- Testing ${name} ---`);
        let success = true;
        let details = '';

        try {
            await this.page.goto(`file://${path.join(__dirname, url)}`);

            // 1. Check for role selector
            await this.page.waitForFunction(() => {
                const options = document.querySelectorAll('#role-selector option');
                return options.length > 1;
            }, { timeout: 15000 });
            const rolesCount = await this.page.locator('#role-selector option').count();
            if (rolesCount > 1) {
                details += `✅ Role selector populated with ${rolesCount} roles.\n`;
            } else {
                throw new Error('Role selector not populated.');
            }

            // 2. Change role and check system prompt
            const initialPrompt = await this.page.inputValue('#system-prompt');
            await this.page.selectOption('#role-selector', { index: 1 });
            await this.page.waitForTimeout(100); // give it a moment to update
            const newPrompt = await this.page.inputValue('#system-prompt');
            if (newPrompt && newPrompt.length > 0) {
                details += '✅ System prompt is populated and functional.\n';
            } else {
                throw new Error('System prompt is empty or not working.');
            }

            // 3. Test system prompt editing
            const customPrompt = "This is a custom system prompt for testing.";
            await this.page.fill('#system-prompt', customPrompt);
            await this.page.waitForTimeout(500);
            const editedPrompt = await this.page.inputValue('#system-prompt');
            if (editedPrompt === customPrompt) {
                details += '✅ System prompt editing works correctly.\n';
            } else {
                throw new Error(`System prompt editing failed. Expected "${customPrompt}", got "${editedPrompt}"`);
            }

            // 4. Run workflow
            if(name !== 'Routing') { // Routing has a special sequence
                await this.page.click(startButtonSelector);
            }

            // 5. Check for output
            await this.page.waitForSelector(`${outputSelector}:not(:empty)`, { timeout: 60000 }); // Wait for up to 60 seconds for workflow to run
            const outputText = await this.page.textContent(outputSelector);
            if (outputText.length > 50) { // Check for some meaningful output
                 details += '✅ Workflow executed and produced output.\n';
            } else {
                throw new Error('Workflow did not produce sufficient output.');
            }

            console.log(`${name} test PASSED.`);

        } catch (error) {
            success = false;
            details += `❌ TEST FAILED: ${error.message}\n`;
            console.error(`${name} test FAILED:`, error.message);
        }

        const screenshotPath = path.join(this.screenshotsDir, `${name.replace(/ /g, '_')}.png`);
        await this.page.screenshot({ path: screenshotPath });
        details += `📸 Screenshot saved to ${screenshotPath}\n`;

        this.results.push({ name, success, details });
    }

    async testPromptChaining() {
        await this.testExample(
            'Prompt Chaining',
            '1-prompt-chaining/index.html',
            '#start-chain',
            '#chain-output'
        );
    }

    async testRouting() {
        console.log(`--- Testing Routing ---`);
        let success = true;
        let details = '';
        const name = 'Routing';

        try {
            await this.page.goto(`file://${path.join(__dirname, '2-routing/index.html')}`);
            await this.page.waitForFunction(() => {
                const options = document.querySelectorAll('#role-selector option');
                return options.length > 1;
            }, { timeout: 15000 });

            // Basic config checks
            const rolesCount = await this.page.locator('#role-selector option').count();
             if (rolesCount > 1) { details += `✅ Role selector populated with ${rolesCount} roles.\n`; } else { throw new Error('Role selector not populated.'); }
            const initialPrompt = await this.page.inputValue('#system-prompt');
            await this.page.selectOption('#role-selector', { index: 1 });
            await this.page.waitForTimeout(100);
            const newPrompt = await this.page.inputValue('#system-prompt');
            if (newPrompt && newPrompt.length > 0) { details += '✅ System prompt is populated and functional.\n'; } else { throw new Error('System prompt is empty or not working.'); }
            const customPrompt = "This is a custom system prompt for testing.";
            await this.page.fill('#system-prompt', customPrompt);
            await this.page.waitForTimeout(500);
            const editedPrompt = await this.page.inputValue('#system-prompt');
            if (editedPrompt === customPrompt) { details += '✅ System prompt editing works correctly.\n'; } else { throw new Error(`System prompt editing failed. Expected "${customPrompt}", got "${editedPrompt}"`); }

            // Routing specific steps
            await this.page.click('#analyze-btn');
            await this.page.waitForSelector('#generate-btn:not([disabled])');
            details += `✅ Analysis step completed.\n`;
            await this.page.click('#generate-btn');
            await this.page.waitForSelector('#output-frame', { timeout: 60000 });

            await this.page.waitForFunction(() => {
                const iframe = document.querySelector('#output-frame');
                return iframe.contentDocument && iframe.contentDocument.body.innerHTML.length > 50;
            }, null, { timeout: 60000 });
            details += '✅ Workflow executed and produced output in iframe.\n';

            console.log(`${name} test PASSED.`);

        } catch (error) {
            success = false;
            details += `❌ TEST FAILED: ${error.message}\n`;
            console.error(`${name} test FAILED:`, error.message);
        }

        const screenshotPath = path.join(this.screenshotsDir, `${name.replace(/ /g, '_')}.png`);
        await this.page.screenshot({ path: screenshotPath });
        details += `📸 Screenshot saved to ${screenshotPath}\n`;

        this.results.push({ name, success, details });
    }

    async testParallelization() {
        await this.testExample(
            'Parallelization',
            '3-parallelization/index.html',
            '#start-analysis',
            '#results-container'
        );
    }

    async testOrchestration() {
        await this.testExample(
            'Orchestration',
            '4-orchestrator-workers/index.html',
            '#start-pipeline',
            '#results-container'
        );
    }

    async testOptimization() {
        await this.testExample(
            'Optimization',
            '5-evaluator-optimizer/index.html',
            '#generate-btn',
            '#initial-content-container'
        );
    }

    generateReport() {
        console.log('\n--- Test Report ---');
        let allPassed = true;
        this.results.forEach(result => {
            console.log(`\nTest: ${result.name}`);
            console.log(`Status: ${result.success ? '✅ PASSED' : '❌ FAILED'}`);
            console.log('Details:\n' + result.details);
            if (!result.success) {
                allPassed = false;
            }
        });
        console.log(`\n--- End of Report ---\n`);
        if (allPassed) {
            console.log('✅ All tests passed!');
        } else {
            console.log('❌ Some tests failed.');
        }
    }
}

(async () => {
    const testSuite = new RefactoredWorkflowsTestSuite();
    await testSuite.runTests();
})();
