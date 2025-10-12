# 🧠 AI Routing - Smart Prototyping Environment

An intelligent prototyping environment that demonstrates the **Routing** workflow pattern by automatically selecting the optimal AI model based on task complexity. Inspired by Lovable's approach to smart development tooling.

## 🎯 Overview

This interactive example showcases how intelligent routing can optimize both cost and quality by analyzing task complexity and automatically selecting the most appropriate AI model for the job. The system evaluates factors like prompt complexity, template requirements, and feature sophistication to make smart routing decisions.

## 🚀 Features

### Smart Model Selection
- **3 AI Models**: GPT-3.5 Turbo, GPT-4, Claude 3 Opus with different capabilities and costs
- **Automatic Routing**: Real-time complexity analysis with intelligent model recommendations
- **Cost Optimization**: Balance between performance and cost based on task requirements
- **Visual Routing**: Interactive network diagram showing model selection process

### Prototyping Templates
- **Landing Page**: Simple marketing sites with hero sections
- **Dashboard**: Analytics interfaces with charts and metrics
- **E-commerce**: Product catalogs with shopping functionality
- **SaaS App**: Complex applications with advanced features
- **Portfolio**: Creative showcases with project galleries

### Real-time Analysis
- **Complexity Meter**: Visual indicator of task sophistication
- **Factor Breakdown**: Detailed analysis of complexity contributors
- **Live Recommendations**: Dynamic model suggestions as you type
- **Template Intelligence**: Base complexity varies by prototype type

### Interactive Generation
- **Live Preview**: Rendered prototypes with actual HTML/CSS
- **Refinement Tools**: Iterative improvement capabilities
- **Results Dashboard**: Performance metrics and cost analysis
- **Auto-save**: Preserves work across sessions

## 🧪 How Routing Works

### 1. Task Analysis
The system analyzes multiple factors to determine complexity:

- **Content Length**: Word count and sentence structure
- **Technical Features**: Keywords like "authentication", "payment", "API"
- **Template Complexity**: Base complexity varies by prototype type
- **Requirements Sophistication**: Mobile, responsive, interactive elements

### 2. Model Selection Algorithm
```javascript
// Complexity scoring (0.0 - 1.0)
complexity = baseComplexity + contentFactors + technicalFeatures

// Model routing logic
if (complexity <= 0.6) → GPT-3.5 Turbo (Fast, $0.002/1k)
if (complexity <= 0.9) → GPT-4 (Advanced, $0.03/1k)
if (complexity = 1.0) → Claude 3 Opus (Expert, $0.075/1k)
```

### 3. Quality-Cost Balance
- **Simple Tasks**: Route to faster, cheaper models
- **Complex Tasks**: Route to more capable, expensive models
- **Automatic Optimization**: Best model for the job without manual selection

## 🎮 User Experience

### Getting Started
1. **Select Template**: Choose from 5 prototype categories
2. **Describe Project**: Enter detailed requirements and features
3. **Analyze Task**: Watch real-time complexity analysis
4. **Generate**: AI automatically routes and creates prototype
5. **Refine**: Iterate and improve the generated output

### Visual Feedback
- **Pipeline Visualization**: Step-by-step routing process
- **Complexity Meter**: Real-time sophistication analysis
- **Model Recommendations**: AI-suggested optimal routing
- **Live Preview**: Actual HTML/CSS rendering of prototypes

## 📊 Example Routing Scenarios

### Simple Landing Page (GPT-3.5 Turbo)
```
Prompt: "Create a landing page for a local bakery with contact info"
Complexity: 25% → Routes to GPT-3.5 Turbo
Cost: $0.002/1k tokens
Result: Clean, simple marketing page
```

### SaaS Dashboard (GPT-4)
```
Prompt: "Build an analytics dashboard with real-time charts, user management, and API integration"
Complexity: 78% → Routes to GPT-4
Cost: $0.03/1k tokens
Result: Feature-rich dashboard with complex UI
```

### Enterprise Application (Claude 3 Opus)
```
Prompt: "Create a comprehensive project management platform with advanced workflows, team collaboration, AI insights, and custom reporting"
Complexity: 95% → Routes to Claude 3 Opus
Cost: $0.075/1k tokens
Result: Sophisticated application with enterprise features
```

## 🔧 Technical Implementation

### Frontend Architecture
- **Vanilla JavaScript**: No framework dependencies for maximum compatibility
- **Real-time Analysis**: Live complexity calculation as you type
- **Component System**: Reusable UI components for consistency
- **Responsive Design**: Works seamlessly across all device sizes

### Routing Algorithm
```javascript
class RoutingPrototypingDemo {
  calculateComplexity(prompt) {
    let complexity = this.templates[template].baseComplexity;

    // Content analysis
    if (wordCount > 100) complexity += 0.2;
    if (wordCount > 200) complexity += 0.2;

    // Feature detection
    complexity += featureMatches * 0.1;

    // Technical requirements
    if (hasResponsive) complexity += 0.1;
    if (hasInteractive) complexity += 0.15;

    return Math.min(1.0, complexity);
  }
}
```

### Model Configuration
```javascript
models = [
  {
    id: 'openai_gpt35',
    name: 'GPT-3.5 Turbo',
    maxComplexity: 0.6,
    cost: 0.002,
    speed: 'Fast'
  },
  // Additional models...
]
```

## 📈 Benefits Demonstrated

### Cost Optimization
- **80% Cost Savings**: Simple tasks use cheaper models automatically
- **No Over-engineering**: Complex models only for complex tasks
- **Transparent Pricing**: Real-time cost estimation

### Quality Assurance
- **Right Tool for Job**: Each model excels in its complexity range
- **Consistent Results**: Reliable routing based on proven algorithms
- **Performance Metrics**: Track quality scores and success rates

### Developer Experience
- **Zero Configuration**: Automatic model selection
- **Visual Feedback**: Clear understanding of routing decisions
- **Iterative Refinement**: Easy to adjust and improve

## 🎨 Visual Design

The interface features a clean, professional design:

- **Split Layout**: Sidebar for controls, main canvas for generation
- **Color-coded Models**: Visual distinction between model capabilities
- **Complexity Visualization**: Intuitive meter showing task sophistication
- **Real-time Feedback**: Live updates throughout the routing process

## 🔄 Integration Points

This example integrates with the terraphim_agent_evolution system:
- **Routing Workflow**: Uses the routing pattern implementation
- **LLM Adapter**: Connects to configured language models
- **Cost Tracking**: Monitors usage and expenses
- **Performance Metrics**: Tracks routing effectiveness

## 💡 Key Learning Outcomes

### Routing Pattern Understanding
- **Task Analysis**: How to evaluate complexity automatically
- **Model Selection**: Criteria for choosing appropriate AI models
- **Cost-Quality Tradeoffs**: Balancing performance and expense

### Practical Applications
- **Prototype Generation**: Rapid creation of web applications
- **Smart Automation**: Intelligent decision-making in AI workflows
- **Resource Optimization**: Efficient use of AI capabilities

## 🚀 Getting Started

1. Open `index.html` in a modern web browser
2. Select a prototype template (Landing Page, Dashboard, etc.)
3. Describe your project requirements in detail
4. Click "Analyze Task" to see complexity analysis
5. Watch the AI route to the optimal model automatically
6. Click "Generate Prototype" to create your application
7. Use "Refine Output" to iterate and improve

The system demonstrates how intelligent routing can make AI workflows more efficient, cost-effective, and user-friendly while maintaining high-quality results.

Experience the power of smart model selection and see how routing can optimize your AI development workflow!
