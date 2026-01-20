# Terraphim AI Frontend Testing Guide

This guide covers the comprehensive end-to-end testing setup for Terraphim AI, including chat functionality, document summarization, and Ollama integration.

## 📋 Overview

The testing suite provides comprehensive coverage for:

- **Chat Functionality**: Complete chat system with Ollama integration
- **Document Summarization**: AI-powered summarization with local LLM
- **Ollama Integration**: Local LLM connectivity and model management
- **Configuration Management**: LLM provider setup and validation
- **Haystack Integration**: External service configuration with secrets

## 🚀 Quick Start

### 1. Environment Setup

```bash
# Copy environment template
cp .env.example .env

# Edit .env with your actual API keys and secrets
# Required for haystack integrations (Atomic Server, ClickUp, etc.)
```

### 2. Automated Setup

```bash
# Run the complete setup script
npm run setup:test
```

This script will:
- Check and install Ollama
- Pull required models (llama3.2:3b)
- Start backend server with Ollama configuration
- Install frontend dependencies
- Setup Playwright browsers

### 3. Validate Ollama

```bash
# Validate Ollama configuration
npm run validate:ollama
```

### 4. Run Tests

```bash
# Run all LLM-related tests
npm run test:llm

# Run specific test suites
npm run test:chat           # Chat functionality
npm run test:summarization  # Document summarization
npm run test:ollama         # Ollama integration
npm run test:config         # Configuration wizard

# Run comprehensive test suite
npm run test:comprehensive
```

### 5. Cleanup

```bash
# Clean up test environment
npm run cleanup:test
```

## 🧪 Test Suites

### Chat Functionality (`chat-functionality.spec.ts`)

Tests the complete chat system including:

- **Interface Initialization**: Chat UI components and navigation
- **Message Handling**: Send/receive messages with Ollama
- **Context Management**: Add, edit, delete conversation context
- **KG Search Integration**: Knowledge graph search within chat
- **Conversation Management**: Create, load, persist conversations
- **Error Handling**: Graceful handling of LLM failures

**Key Test Cases:**
- Chat interface displays correctly
- Messages send and receive with Ollama
- Context panel management
- KG search modal integration
- Error recovery and retry functionality

### Summarization (`summarization.spec.ts`)

Tests document summarization features:

- **Basic Summarization**: Generate AI summaries for search results
- **Auto-summarization**: Automatic summarization based on role config
- **Cache Management**: Summary caching and regeneration
- **Error Handling**: Timeout and service unavailable scenarios
- **Performance Testing**: Loading indicators and cancellation

**Key Test Cases:**
- Generate summaries for search results
- Respect auto-summarize configuration
- Handle Ollama service failures gracefully
- Provide visual feedback during generation

### Ollama Integration (`ollama-integration.spec.ts`)

Tests Ollama connectivity and functionality:

- **Health Checks**: Service availability and model validation
- **Model Quality**: Response coherence and relevance
- **Streaming Responses**: Real-time response generation
- **Performance Testing**: Concurrent requests and memory usage
- **Configuration**: Model switching and parameter validation

**Key Test Cases:**
- Verify Ollama service is running
- Validate model availability (llama3.2:3b)
- Test response quality for programming questions
- Handle service unavailable scenarios

### Configuration Wizard (`config-wizard-complete.spec.ts`)

Tests the complete configuration system:

- **LLM Provider Setup**: Ollama and OpenRouter configuration
- **Validation**: API key and connectivity testing
- **Haystack Configuration**: External service setup with secrets
- **Role Management**: Create, edit, delete roles
- **Persistence**: Configuration save and reload

**Key Test Cases:**
- Configure Ollama with base URL and model
- Configure OpenRouter with API key validation
- Setup haystack services (Atomic Server, ClickUp)
- Validate required fields and handle conflicts

## 🔧 Configuration

### Environment Variables

Required environment variables in `.env`:

```env
# Ollama Configuration
OLLAMA_BASE_URL=http://127.0.0.1:11434
OLLAMA_MODEL=llama3.2:3b

# Atomic Server
ATOMIC_SERVER_URL=http://localhost:9883
ATOMIC_SERVER_SECRET=your_secret_here

# OpenRouter (optional)
OPENROUTER_API_KEY=sk-or-v1-your-key-here

# ClickUp (optional)
CLICKUP_API_TOKEN=pk_your_token_here
CLICKUP_TEAM_ID=your_team_id

# GitHub (optional)
GITHUB_TOKEN=ghp_your_token_here
```

### Test Timeouts

Test timeouts are configured for LLM operations:

- **Standard tests**: 120 seconds
- **LLM response timeout**: 60 seconds
- **Summarization timeout**: 45 seconds
- **Health check timeout**: 10 seconds

### Model Requirements

Required Ollama models:
- `llama3.2:3b` (primary model for testing)
- `llama3:8b` (alternative model)

## 📝 NPM Scripts

### Setup and Validation
- `npm run setup:test` - Complete environment setup
- `npm run validate:ollama` - Validate Ollama configuration
- `npm run cleanup:test` - Clean up test environment

### Individual Test Suites
- `npm run test:chat` - Chat functionality tests
- `npm run test:summarization` - Summarization tests
- `npm run test:ollama` - Ollama integration tests
- `npm run test:config` - Configuration wizard tests

### Test Variations
- `npm run test:chat:headed` - Run with browser UI visible
- `npm run test:chat:ci` - CI mode with retries and reporting
- `npm run test:llm` - All LLM-related tests
- `npm run test:comprehensive` - Complete test suite

### CI/CD Scripts
- `npm run test:comprehensive:ci` - Full CI test run with reporting
- All `:ci` variants include retries and structured reporting

## 🚨 Troubleshooting

### Common Issues

**Ollama Not Running**
```bash
# Start Ollama service
ollama serve

# Verify it's running
curl http://127.0.0.1:11434/api/tags
```

**Model Not Available**
```bash
# Pull required model
ollama pull llama3.2:3b

# List available models
ollama list
```

**Backend Server Not Starting**
```bash
# Check if port is in use
netstat -an | grep 8080

# Start backend manually with Ollama config
cd ..
cargo run --release -- --config terraphim_server/default/ollama_llama_config.json
```

**Test Failures Due to Timeouts**
- Increase timeout values in test files
- Check system resources (CPU, memory)
- Verify Ollama model is loaded and warm

**Environment Variable Issues**
```bash
# Verify .env file exists and has correct values
cat .env

# Check if variables are loaded
npm run validate:ollama
```

### Debug Mode

Run tests with debug output:
```bash
# Debug specific test
npm run test:chat -- --debug

# Run with browser visible
npm run test:chat:headed

# Verbose output
DEBUG=pw:* npm run test:chat
```

### Performance Issues

If tests are slow:
1. Ensure Ollama model is pre-loaded
2. Check available system memory
3. Reduce concurrent test workers
4. Use smaller models for testing

## 🏗️ CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests with Ollama

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install Ollama
        run: |
          curl https://ollama.ai/install.sh | sh
          ollama serve &
          sleep 10
          ollama pull llama3.2:3b

      - name: Install dependencies
        run: |
          cd desktop
          npm install

      - name: Setup test environment
        run: |
          cd desktop
          npm run setup:test

      - name: Run comprehensive tests
        run: |
          cd desktop
          npm run test:comprehensive:ci

      - name: Upload test results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: desktop/test-results/
```

### Environment Secrets

Configure these secrets in your CI environment:
- `ATOMIC_SERVER_SECRET`
- `OPENROUTER_API_KEY`
- `CLICKUP_API_TOKEN`
- `GITHUB_TOKEN`

## 📊 Test Coverage

The test suite provides comprehensive coverage across:

- **Frontend Components**: All major UI components tested
- **API Integration**: Backend API calls and responses
- **LLM Integration**: Complete Ollama workflow
- **Configuration**: All configuration scenarios
- **Error Handling**: Network failures, timeouts, invalid inputs
- **Performance**: Load testing and concurrent operations

### Coverage Reports

Generate test coverage reports:
```bash
# Run tests with coverage
npm run test:coverage

# View coverage report
open coverage/index.html
```

## 🔍 Monitoring and Metrics

Tests include performance monitoring:

- **Response Times**: LLM response latency
- **Memory Usage**: Frontend and Ollama memory consumption
- **Error Rates**: Failed requests and timeouts
- **Model Performance**: Token generation speed

View metrics in test output and reports.

## 📚 Additional Resources

- [Playwright Documentation](https://playwright.dev/)
- [Ollama API Reference](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [Terraphim AI Architecture](../docs/architecture.md)
- [Configuration Guide](../docs/configuration.md)

## 🤝 Contributing

When adding new tests:

1. Follow existing test patterns
2. Include proper error handling
3. Add timeout configurations
4. Update this documentation
5. Test in both CI and local environments

## 📄 License

This testing suite is part of the Terraphim AI project and follows the same license terms.
