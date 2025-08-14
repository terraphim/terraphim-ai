# Ollama LLM Integration for Terraphim AI

This directory contains comprehensive integration tests and role configurations for using Ollama as a local LLM provider with Terraphim AI, specifically configured for the `llama3.2:3b` model.

## 🚀 Quick Start

### Prerequisites

1. **Ollama installed and running**
   ```bash
   # Install Ollama (if not already installed)
   curl -fsSL https://ollama.ai/install.sh | sh
   
   # Start Ollama service
   ollama serve
   ```

2. **Pull the llama3.2:3b model**
   ```bash
   ollama pull llama3.2:3b
   ```

3. **Verify Ollama is running**
   ```bash
   curl http://127.0.0.1:11434/api/tags
   ```

### Run Integration Tests

```bash
# Run the comprehensive test suite
./run_ollama_llama_tests.sh

# Or run individual tests
cargo test --features ollama ollama_llama_integration_comprehensive
cargo test --features ollama ollama_llama_performance_test
```

## 📋 Test Coverage

### 1. Comprehensive Integration Test (`ollama_llama_integration_comprehensive`)

Tests the complete Ollama LLM integration workflow:

- **Connectivity Test**: Verifies Ollama instance is reachable
- **Direct LLM Client Test**: Tests summarization with llama3.2:3b
- **Role Configuration Test**: Validates role-based LLM client building
- **End-to-End Search Test**: Tests search with auto-summarization
- **Model Listing Test**: Verifies model availability and listing

### 2. Performance Test (`ollama_llama_performance_test`)

Tests reliability and performance under load:

- **Concurrent Requests**: Multiple summarization requests
- **Success Rate**: Measures reliability across requests
- **Response Times**: Tracks performance metrics
- **Error Handling**: Validates graceful degradation

## 🎭 Role Configurations

### Llama Rust Engineer
- **Purpose**: Rust-focused development and documentation
- **Relevance Function**: Title Scorer
- **Theme**: Cosmo
- **Features**: Auto-summarization enabled

### Llama AI Assistant
- **Purpose**: AI-powered assistance with knowledge graph
- **Relevance Function**: Terraphim Graph
- **Theme**: Lumen
- **Features**: Knowledge graph integration + auto-summarization

### Llama Developer
- **Purpose**: General development with BM25 scoring
- **Relevance Function**: BM25
- **Theme**: Spacelab
- **Features**: BM25 scoring + auto-summarization

## ⚙️ Configuration

### Environment Variables

```bash
export OLLAMA_BASE_URL="http://127.0.0.1:11434"
export RUST_LOG="info"
```

### Role Configuration Structure

```json
{
  "extra": {
    "llm_provider": "ollama",
    "llm_model": "llama3.2:3b",
    "llm_base_url": "http://127.0.0.1:11434",
    "llm_auto_summarize": true,
    "llm_description": "Description of the role's LLM capabilities"
  }
}
```

## 🔧 Technical Details

### LLM Client Implementation

The Ollama integration is implemented in `crates/terraphim_service/src/llm.rs`:

- **OllamaClient**: Implements the `LlmClient` trait
- **HTTP Client**: Uses reqwest for API communication
- **Retry Logic**: Built-in retry mechanism for reliability
- **Timeout Handling**: Configurable timeouts for different operations

### API Endpoints

- **Chat Completion**: `POST /api/chat` for summarization
- **Model Listing**: `GET /api/tags` for available models
- **Health Check**: `GET /api/tags` for connectivity testing

### Error Handling

- **Network Errors**: Automatic retry with exponential backoff
- **Model Errors**: Graceful degradation with user feedback
- **Timeout Errors**: Configurable timeouts with fallback behavior

## 📊 Performance Characteristics

### llama3.2:3b Model

- **Model Size**: ~3B parameters
- **Memory Usage**: ~2GB RAM
- **Response Time**: 1-5 seconds for typical summarization
- **Quality**: Good for technical content and summarization

### Optimization Features

- **Content Truncation**: Automatic content length management
- **Token Calculation**: Smart token limit calculation
- **Concurrent Processing**: Support for multiple concurrent requests
- **Caching**: Built-in response caching for repeated queries

## 🧪 Testing Strategy

### Test Categories

1. **Unit Tests**: Individual component testing
2. **Integration Tests**: End-to-end workflow testing
3. **Performance Tests**: Load and reliability testing
4. **Configuration Tests**: Role and setting validation

### Test Data

- **Rust Documentation**: Technical content for summarization
- **Markdown Files**: Real-world document formats
- **Varied Content Lengths**: Different input sizes for testing
- **Edge Cases**: Empty content, very long content, special characters

### Test Environment

- **Local Ollama**: Real Ollama instance for authentic testing
- **Temporary Files**: Isolated test data with cleanup
- **Mock Services**: Fallback for CI environments
- **Serial Execution**: Prevents test interference

## 🚨 Troubleshooting

### Common Issues

1. **Ollama Not Running**
   ```bash
   # Check if Ollama is running
   ps aux | grep ollama
   
   # Start Ollama service
   ollama serve
   ```

2. **Model Not Available**
   ```bash
   # List available models
   ollama list
   
   # Pull the required model
   ollama pull llama3.2:3b
   ```

3. **Connection Timeout**
   ```bash
   # Check Ollama port
   netstat -an | grep 11434
   
   # Test connectivity
   curl -v http://127.0.0.1:11434/api/tags
   ```

4. **Test Failures**
   ```bash
   # Run with verbose output
   cargo test --features ollama ollama -- --nocapture
   
   # Check logs
   export RUST_LOG=debug
   ```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug

# Run tests with detailed output
cargo test --features ollama ollama -- --nocapture --test-threads=1
```

## 🔮 Future Enhancements

### Planned Features

1. **Streaming Support**: Real-time response streaming
2. **Model Switching**: Dynamic model selection
3. **Advanced Prompting**: Custom prompt templates
4. **Batch Processing**: Multiple document summarization
5. **Quality Metrics**: Response quality assessment

### Performance Improvements

1. **Response Caching**: Intelligent caching strategies
2. **Connection Pooling**: Optimized HTTP connections
3. **Async Batching**: Efficient request batching
4. **Memory Optimization**: Reduced memory footprint

## 📚 Additional Resources

- [Ollama Documentation](https://ollama.ai/docs)
- [Llama 3.2 Model Card](https://ollama.ai/library/llama3.2)
- [Terraphim AI Documentation](https://terraphim.ai)
- [Rust Async Programming](https://rust-lang.github.io/async-book/)

## 🤝 Contributing

To contribute to the Ollama integration:

1. **Run Tests**: Ensure all tests pass
2. **Add Tests**: Include tests for new features
3. **Update Docs**: Keep documentation current
4. **Performance**: Monitor performance impact
5. **Compatibility**: Maintain backward compatibility

---

**Status**: ✅ **Production Ready** - Comprehensive testing and validation complete
**Last Updated**: 2025-01-31
**Version**: v1.0.0
