/**
 * Technology Stack Validation Test
 * Validates that the new JavaScript/Bun/SQLite stack is properly configured
 */

async function runTechStackValidation() {
  console.log('🧪 Running Technology Stack Validation Tests...\n');
  
  const results = [];
  const expectedTechStack = 'JavaScript, Bun, Express, SQLite, JWT';
  
  // Test 1: Validate placeholder text
  console.log('Testing: Placeholder Text Update');
  try {
    const techStackInput = document.getElementById('tech-stack');
    const actualPlaceholder = techStackInput?.placeholder;
    const expectedPlaceholder = 'JavaScript, Bun, Express, SQLite';
    
    if (actualPlaceholder === expectedPlaceholder) {
      results.push({ test: 'Placeholder Text Update', status: 'PASS', message: `✅ Placeholder correctly shows: ${actualPlaceholder}` });
    } else {
      results.push({ test: 'Placeholder Text Update', status: 'FAIL', message: `❌ Expected: ${expectedPlaceholder}, Got: ${actualPlaceholder}` });
    }
  } catch (error) {
    results.push({ test: 'Placeholder Text Update', status: 'ERROR', message: `⚠️ Error: ${error.message}` });
  }
  
  // Test 2: Validate project description placeholder
  console.log('Testing: Project Description Placeholder');
  try {
    const projectDescInput = document.getElementById('project-description');
    const placeholder = projectDescInput?.placeholder;
    
    if (placeholder && placeholder.includes('JavaScript and Bun')) {
      results.push({ test: 'Project Description Placeholder', status: 'PASS', message: '✅ Project description mentions JavaScript and Bun' });
    } else {
      results.push({ test: 'Project Description Placeholder', status: 'FAIL', message: `❌ Project description placeholder doesn't mention JavaScript and Bun` });
    }
  } catch (error) {
    results.push({ test: 'Project Description Placeholder', status: 'ERROR', message: `⚠️ Error: ${error.message}` });
  }
  
  // Test 3: Validate buildMainPrompt includes technology stack
  console.log('Testing: Main Prompt Generation');
  try {
    // Set up the form fields with test data
    const techStackInput = document.getElementById('tech-stack');
    const projectDescInput = document.getElementById('project-description');
    const requirementsInput = document.getElementById('requirements');
    
    if (techStackInput && projectDescInput && requirementsInput) {
      // Save original values
      const originalTechStack = techStackInput.value;
      const originalProject = projectDescInput.value;
      const originalRequirements = requirementsInput.value;
      
      // Set test values
      techStackInput.value = expectedTechStack;
      projectDescInput.value = 'Test project for validation';
      requirementsInput.value = 'Test requirements';
      
      // Check if we can access the global promptChainDemo instance
      if (typeof window.promptChainDemo !== 'undefined' && window.promptChainDemo.buildMainPrompt) {
        const prompt = window.promptChainDemo.buildMainPrompt();
        
        // Check if all technologies are included
        const technologies = ['JavaScript', 'Bun', 'Express', 'SQLite', 'JWT'];
        const missingTech = technologies.filter(tech => !prompt.includes(tech));
        
        if (missingTech.length === 0) {
          results.push({ test: 'Main Prompt Generation', status: 'PASS', message: '✅ All technologies included in generated prompt' });
        } else {
          results.push({ test: 'Main Prompt Generation', status: 'FAIL', message: `❌ Missing technologies in prompt: ${missingTech.join(', ')}` });
        }
        
        // Restore original values
        techStackInput.value = originalTechStack;
        projectDescInput.value = originalProject;
        requirementsInput.value = originalRequirements;
      } else {
        results.push({ test: 'Main Prompt Generation', status: 'ERROR', message: '⚠️ PromptChainingDemo instance not available' });
      }
    } else {
      results.push({ test: 'Main Prompt Generation', status: 'ERROR', message: '⚠️ Required form inputs not found' });
    }
  } catch (error) {
    results.push({ test: 'Main Prompt Generation', status: 'ERROR', message: `⚠️ Error: ${error.message}` });
  }
  
  // Test 4: Validate template configuration
  console.log('Testing: Template Configuration');
  try {
    if (typeof window.promptChainDemo !== 'undefined' && window.promptChainDemo.getProjectTemplates) {
      const templates = window.promptChainDemo.getProjectTemplates();
      const webAppTemplate = templates['web-app'];
      
      if (webAppTemplate && webAppTemplate.techStack === expectedTechStack) {
        results.push({ test: 'Template Configuration', status: 'PASS', message: `✅ Template uses correct tech stack: ${webAppTemplate.techStack}` });
      } else {
        const actualStack = webAppTemplate?.techStack || 'not found';
        results.push({ test: 'Template Configuration', status: 'FAIL', message: `❌ Expected: ${expectedTechStack}, Got: ${actualStack}` });
      }
    } else {
      results.push({ test: 'Template Configuration', status: 'ERROR', message: '⚠️ PromptChainingDemo getProjectTemplates method not available' });
    }
  } catch (error) {
    results.push({ test: 'Template Configuration', status: 'ERROR', message: `⚠️ Error: ${error.message}` });
  }
  
  // Print results
  console.log('\n📊 Validation Results:');
  console.log('====================');
  
  let passed = 0;
  let failed = 0;
  let errors = 0;
  
  results.forEach(result => {
    console.log(`${result.message}`);
    
    if (result.status === 'PASS') passed++;
    else if (result.status === 'FAIL') failed++;
    else errors++;
  });
  
  console.log(`\n📈 Summary: ${passed} passed, ${failed} failed, ${errors} errors`);
  
  if (failed === 0 && errors === 0) {
    console.log('🎉 ALL TESTS PASSED! Technology stack is correctly configured.');
    return true;
  } else {
    console.log('❌ Some tests failed. Technology stack configuration needs attention.');
    return false;
  }
}

// Auto-run the validation
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', runTechStackValidation);
} else {
  runTechStackValidation();
}