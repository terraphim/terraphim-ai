# Perplexity Integration for Terraphim AI

This document describes the integration of Perplexity AI's web search API as a haystack type in Terraphim AI, providing real-time, AI-powered web search capabilities.

## Overview

The Perplexity integration allows Terraphim AI to access current information from the web through Perplexity's AI-powered search API. This enables users to get real-time insights, current events, and up-to-date information that complements local knowledge bases.

## Features

- **Real-time Web Search**: Access current information from across the web
- **AI-Powered Summaries**: Get concise, relevant summaries of search results
- **Citation Tracking**: Automatic source attribution and verification
- **Configurable Models**: Support for different Perplexity models (sonar-small, medium, large)
- **Domain Filtering**: Restrict searches to specific domains for focused results
- **Response Caching**: Reduce API costs and improve performance with intelligent caching
- **Graceful Degradation**: System continues to work even if API is unavailable
- **Multiple Search Contexts**: Configure different search profiles for different use cases

## Configuration

### API Key Setup

First, obtain a Perplexity API key from [https://perplexity.ai](https://perplexity.ai) and set it as an environment variable:

```bash
export PERPLEXITY_API_KEY="your-api-key-here"
```

Alternatively, you can specify the API key directly in the haystack configuration.

### Basic Configuration

Here's a minimal configuration for a Perplexity haystack:

```json
{
  "haystacks": [
    {
      "location": "https://api.perplexity.ai",
      "service": "Perplexity",
      "read_only": true,
      "extra_parameters": {
        "model": "sonar-medium-online"
      }
    }
  ]
}
```

### Advanced Configuration

For more control, use the full configuration options:

```json
{
  "haystacks": [
    {
      "location": "https://api.perplexity.ai",
      "service": "Perplexity",
      "read_only": true,
      "extra_parameters": {
        "api_key": "your-api-key",
        "model": "sonar-large-online",
        "max_tokens": "2000",
        "temperature": "0.1",
        "cache_ttl_hours": "2",
        "search_domains": "arxiv.org,github.com,stackoverflow.com",
        "search_recency": "week"
      }
    }
  ]
}
```

## Configuration Parameters

### Required Parameters

- **service**: Must be set to `"Perplexity"`
- **location**: Should be `"https://api.perplexity.ai"`
- **API key**: Either via `PERPLEXITY_API_KEY` environment variable or `api_key` in extra_parameters

### Optional Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `model` | `"sonar-medium-online"` | Perplexity model to use |
| `max_tokens` | None | Maximum tokens in response |
| `temperature` | None | Response randomness (0.0-1.0) |
| `cache_ttl_hours` | `1` | How long to cache responses |
| `search_domains` | None | Comma-separated list of domains to search |
| `search_recency` | None | Time filter: "hour", "day", "week", "month" |

## Available Models

Perplexity offers several models optimized for different use cases:

- **sonar-small-online**: Fastest, most cost-effective for simple queries
- **sonar-medium-online**: Balanced performance and quality (recommended)
- **sonar-large-online**: Highest quality responses for complex queries

## Example Configurations

### Research Scientist Profile

```json
{
  "name": "AI Research Scientist",
  "haystacks": [
    {
      "location": "https://api.perplexity.ai",
      "service": "Perplexity",
      "read_only": true,
      "extra_parameters": {
        "model": "sonar-large-online",
        "max_tokens": "2000",
        "temperature": "0.1",
        "search_domains": "arxiv.org,huggingface.co,papers.nips.cc",
        "search_recency": "month",
        "cache_ttl_hours": "4"
      }
    }
  ]
}
```

### News Analyst Profile

```json
{
  "name": "News Analyst",
  "haystacks": [
    {
      "location": "https://api.perplexity.ai",
      "service": "Perplexity",
      "read_only": true,
      "extra_parameters": {
        "model": "sonar-medium-online",
        "max_tokens": "1500",
        "temperature": "0.3",
        "search_domains": "reuters.com,bbc.com,cnn.com,npr.org",
        "search_recency": "day",
        "cache_ttl_hours": "0.5"
      }
    }
  ]
}
```

### General Research Profile

```json
{
  "name": "General Researcher",
  "haystacks": [
    {
      "location": "https://api.perplexity.ai",
      "service": "Perplexity",
      "read_only": true,
      "extra_parameters": {
        "model": "sonar-medium-online",
        "max_tokens": "1000",
        "temperature": "0.2",
        "cache_ttl_hours": "2"
      }
    }
  ]
}
```

## Usage Examples

### Running a Search

Once configured, searches work through the standard Terraphim interface:

```bash
# Using the server API
curl -X POST "http://localhost:8080/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "latest developments in AI safety", "limit": 5}'

# Or through the desktop interface
# Just type your query and the Perplexity results will be included
```

### Testing the Integration

Run the test suite to verify your configuration:

```bash
# Run basic configuration tests
cargo test -p terraphim_middleware test_perplexity

# Run live API test (requires PERPLEXITY_API_KEY)
cargo test -p terraphim_middleware perplexity_live_api_test -- --ignored
```

## Caching and Performance

### Response Caching

The integration includes intelligent caching to:
- Reduce API costs by avoiding duplicate requests
- Improve response times for repeated queries
- Maintain performance during high-usage periods

Cache behavior:
- **Default TTL**: 1 hour (configurable)
- **Cache Key**: Based on normalized query text
- **Storage**: Uses Terraphim's persistence layer
- **Invalidation**: Time-based expiration

### Performance Optimization Tips

1. **Choose the right model**:
   - Use `sonar-small-online` for simple factual queries
   - Use `sonar-medium-online` for balanced performance
   - Use `sonar-large-online` only for complex research tasks

2. **Configure caching appropriately**:
   - Longer cache times for stable information
   - Shorter cache times for rapidly changing topics

3. **Use domain filtering**:
   - Speeds up searches by focusing on relevant sources
   - Improves result quality for specialized topics

4. **Set appropriate token limits**:
   - Higher limits for detailed research
   - Lower limits for quick facts and summaries

## Error Handling and Resilience

The integration includes robust error handling:

### Graceful Degradation
- System continues working if Perplexity API is unavailable
- Returns empty results rather than failing entire searches
- Logs warnings for debugging without breaking user experience

### Common Error Scenarios
- **Missing API Key**: Returns empty results with warning log
- **API Rate Limits**: Respects limits and caches responses
- **Network Issues**: Timeout handling with configurable retries
- **Invalid Configuration**: Clear error messages for debugging

### Monitoring and Debugging

Enable debug logging to monitor API usage:

```bash
LOG_LEVEL=debug cargo run
```

This will show:
- API request/response times
- Cache hit/miss ratios
- Token usage statistics
- Error details for debugging

## Best Practices

### API Usage
1. **Set reasonable token limits** to control costs
2. **Use caching effectively** to reduce redundant requests
3. **Choose appropriate models** based on query complexity
4. **Monitor usage** to stay within API limits

### Configuration
1. **Environment variables** for sensitive data (API keys)
2. **Domain filtering** for focused, relevant results
3. **Appropriate recency filters** based on content type
4. **Role-based configurations** for different user needs

### Integration
1. **Combine with local haystacks** for comprehensive coverage
2. **Use different cache TTLs** based on information freshness needs
3. **Configure multiple Perplexity profiles** for different use cases
4. **Test configurations** before deploying to users

## Troubleshooting

### Common Issues

**No results returned**
- Check API key is set correctly
- Verify network connectivity
- Check Perplexity API status
- Review log messages for specific errors

**Slow response times**
- Try a smaller/faster model
- Reduce max_tokens setting
- Check cache hit rates
- Consider domain filtering

**High API costs**
- Increase cache TTL
- Reduce max_tokens
- Use smaller models where appropriate
- Monitor usage patterns

**Configuration errors**
- Validate JSON syntax
- Check parameter names and types
- Verify API key format
- Test with minimal configuration first

### Debug Commands

```bash
# Test basic configuration
cargo test -p terraphim_middleware test_perplexity_config_parsing

# Test live API (requires API key)
PERPLEXITY_API_KEY=your-key cargo test -p terraphim_middleware perplexity_live_api_test -- --ignored

# Check server logs
LOG_LEVEL=debug cargo run -- --config perplexity_researcher_config.json
```

## API Reference

### Supported Perplexity API Features

- **Chat Completions**: Primary interface for search queries
- **Online Models**: Real-time web search capabilities
- **Domain Filtering**: Restrict search to specific websites
- **Recency Filtering**: Time-based result filtering
- **Citation Tracking**: Automatic source attribution

### Response Format

Perplexity responses are converted to Terraphim Documents with:

- **Title**: Generated from query with "[Perplexity]" prefix
- **Body**: AI-generated summary with citations
- **Description**: Context about the search
- **URL**: Custom perplexity:// scheme with encoded query
- **Tags**: ["perplexity", "ai-search", "web-search", "real-time"]
- **Rank**: High ranking (1000) for prioritization
- **Sources**: Separate documents for each citation (optional)

## Security Considerations

### API Key Management
- Store API keys as environment variables
- Never commit API keys to version control
- Rotate keys regularly
- Monitor usage for unauthorized access

### Data Privacy
- Search queries are sent to Perplexity's servers
- Responses are cached locally
- Consider data sensitivity when using external APIs
- Review Perplexity's privacy policy for compliance

## Future Enhancements

Potential improvements for the integration:

1. **Streaming Responses**: Real-time result streaming
2. **Advanced Caching**: Smart cache invalidation based on content type
3. **Usage Analytics**: Detailed API usage and cost tracking
4. **Custom Prompts**: Configurable system prompts for different use cases
5. **Multi-language Support**: International search capabilities
6. **Result Ranking**: Integration with Terraphim's relevance scoring

## Contributing

To contribute improvements to the Perplexity integration:

1. Follow the existing code patterns in `crates/terraphim_middleware/src/haystack/perplexity.rs`
2. Add tests for new functionality in `tests/perplexity_haystack_test.rs`
3. Update this documentation for any new features
4. Ensure all tests pass: `cargo test -p terraphim_middleware`
5. Submit a pull request with clear description of changes

## Support

For issues specific to the Perplexity integration:

1. Check this documentation for configuration guidance
2. Run the test suite to verify setup
3. Review logs with debug level enabled
4. Check Perplexity API documentation for service updates
5. Open an issue in the Terraphim AI repository

---

*This integration enables Terraphim AI to access real-time web information through Perplexity's advanced AI search capabilities, making it a powerful tool for research, current events analysis, and staying up-to-date with rapidly changing information.*
