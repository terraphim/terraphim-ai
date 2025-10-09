/**
 * Test: Technology Stack Prompt Validation
 * Verifies that the prompt-chaining example uses the correct default technology stack
 */

class TechStackTest {
  constructor() {
    this.expectedTechStack = 'JavaScript, Bun, Express, SQLite, JWT';
    this.results = [];
  }

  async runTests() {
    console.log('🧪 Running Technology Stack Tests...\n');
    
    await this.testDefaultTechStackValue();
    await this.testPlaceholderText();
    await this.testPromptGeneration();
    
    this.printResults();
  }

  async testDefaultTechStackValue() {
    const testName = 'Default Technology Stack Value';
    console.log(`Testing: ${testName}`);
    
    try {
      // Check if promptChainDemo global variable exists and has getProjectTemplates method
      if (typeof window.promptChainDemo !== 'undefined' && window.promptChainDemo.getProjectTemplates) {
        const templates = window.promptChainDemo.getProjectTemplates();
        const webAppTemplate = templates['web-app'];
        const actualTechStack = webAppTemplate.techStack;
        
        if (actualTechStack === this.expectedTechStack) {
          this.results.push({ test: testName, status: 'PASS', message: `Correct tech stack: ${actualTechStack}` });
        } else {
          this.results.push({ test: testName, status: 'FAIL', message: `Expected: ${this.expectedTechStack}, Got: ${actualTechStack}` });
        }
      } else {
        this.results.push({ test: testName, status: 'ERROR', message: 'PromptChainingDemo instance not found or getProjectTemplates method missing' });
      }
    } catch (error) {
      this.results.push({ test: testName, status: 'ERROR', message: error.message });
    }
  }

  async testPlaceholderText() {
    const testName = 'Placeholder Text Contains New Stack';
    console.log(`Testing: ${testName}`);
    
    try {
      const techStackInput = document.getElementById('tech-stack');
      const placeholder = techStackInput?.placeholder || '';
      
      const expectedPlaceholder = 'JavaScript, Bun, Express, SQLite';
      
      if (placeholder === expectedPlaceholder) {
        this.results.push({ test: testName, status: 'PASS', message: `Correct placeholder: ${placeholder}` });
      } else {
        this.results.push({ test: testName, status: 'FAIL', message: `Expected: ${expectedPlaceholder}, Got: ${placeholder}` });
      }
    } catch (error) {
      this.results.push({ test: testName, status: 'ERROR', message: error.message });
    }
  }

  async testPromptGeneration() {
    const testName = 'Prompt Generation Uses Tech Stack';
    console.log(`Testing: ${testName}`);
    
    try {
      // Check if promptChainDemo global variable exists and has buildMainPrompt method
      if (typeof window.promptChainDemo !== 'undefined' && window.promptChainDemo.buildMainPrompt) {
        // Set the tech stack field to our expected value
        const techStackInput = document.getElementById('tech-stack');
        if (techStackInput) {
          techStackInput.value = this.expectedTechStack;
        }
        
        // Set project description
        const projectDescInput = document.getElementById('project-description');
        if (projectDescInput) {
          projectDescInput.value = 'Build a task management application';
        }
        
        // Generate the main prompt
        const prompt = window.promptChainDemo.buildMainPrompt();
        
        // Check if the prompt contains all expected technologies
        const technologies = ['JavaScript', 'Bun', 'Express', 'SQLite', 'JWT'];
        const missingTech = technologies.filter(tech => !prompt.includes(tech));
        
        if (missingTech.length === 0) {
          this.results.push({ test: testName, status: 'PASS', message: 'All technologies found in prompt' });
        } else {
          this.results.push({ test: testName, status: 'FAIL', message: `Missing technologies: ${missingTech.join(', ')}` });
        }
      } else {
        this.results.push({ test: testName, status: 'ERROR', message: 'PromptChainingDemo instance not found or buildMainPrompt method missing' });
      }
    } catch (error) {
      this.results.push({ test: testName, status: 'ERROR', message: error.message });
    }
  }

  printResults() {
    console.log('\n📊 Test Results:');
    console.log('================');
    
    let passed = 0;
    let failed = 0;
    let errors = 0;
    
    this.results.forEach(result => {
      const emoji = result.status === 'PASS' ? '✅' : result.status === 'FAIL' ? '❌' : '⚠️';
      console.log(`${emoji} ${result.test}: ${result.status}`);
      console.log(`   ${result.message}\n`);
      
      if (result.status === 'PASS') passed++;
      else if (result.status === 'FAIL') failed++;
      else errors++;
    });
    
    console.log(`Summary: ${passed} passed, ${failed} failed, ${errors} errors`);
    
    if (failed === 0 && errors === 0) {
      console.log('🎉 All tests passed! Technology stack is correctly configured.');
    } else {
      console.log('⚠️  Some tests failed. Please check the configuration.');
    }
  }
}

// Integration test function to be called from the browser
async function validateTechStack() {
  const test = new TechStackTest();
  await test.runTests();
}

// Auto-run if this file is loaded directly
if (typeof window !== 'undefined' && window.location) {
  // Wait for DOM to be ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', validateTechStack);
  } else {
    validateTechStack();
  }
}

// Export for Node.js testing
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { TechStackTest, validateTechStack };
}