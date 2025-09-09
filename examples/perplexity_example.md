# Running Terraphim AI with Perplexity Integration

This example shows how to set up and run Terraphim AI with Perplexity's AI-powered web search.

## Prerequisites

1. **Perplexity API Key**: Sign up at [https://perplexity.ai](https://perplexity.ai) and get your API key
2. **Rust Environment**: Ensure you have Rust installed (tested with 1.70+)
3. **Terraphim AI**: Clone and build the Terraphim AI repository

## Quick Start

### Step 1: Set Your API Key

```bash
# Set the API key as an environment variable
export PERPLEXITY_API_KEY="your-api-key-here"

# Or add to your shell profile for persistence
echo 'export PERPLEXITY_API_KEY="your-api-key-here"' >> ~/.bashrc
source ~/.bashrc
```

### Step 2: Run the Server

```bash
# From the terraphim-ai root directory
cargo run -- --config terraphim_server/default/perplexity_researcher_config.json
```

### Step 3: Test the Integration

```bash
# In another terminal, test the API
curl -X POST "http://localhost:8080/documents/search" \
  -H "Content-Type: application/json" \
  -d '{
    "search_term": "latest developments in artificial intelligence 2024",
    "limit": 5
  }'
```

## Example Queries to Try

### Technology Research
```bash
curl -X POST "http://localhost:8080/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "Rust programming language new features", "limit": 3}'
```

### Current Events
```bash
curl -X POST "http://localhost:8080/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "climate change news this week", "limit": 5}'
```

### Scientific Research
```bash
curl -X POST "http://localhost:8080/documents/search" \
  -H "Content-Type: application/json" \
  -d '{"search_term": "quantum computing breakthroughs 2024", "limit": 4}'
```

## Expected Response Format

You should receive responses like this:

```json
{
  "documents": [
    {
      "id": "perplexity_rust_programming_language_new_features",
      "url": "perplexity://search/Rust%20programming%20language%20new%20features",
      "title": "[Perplexity] Rust programming language new features",
      "body": "Rust has introduced several exciting new features in recent releases...\n\nSources:\nhttps://blog.rust-lang.org/...",
      "description": "AI-powered web search results from Perplexity for: Rust programming language new features",
      "tags": ["perplexity", "ai-search", "web-search", "real-time"],
      "rank": 1000
    }
  ]
}
```

## Using the Desktop Interface

If you prefer the desktop application:

```bash
# Run the desktop frontend (in a separate terminal)
cd desktop
yarn install
yarn dev

# Open your browser to the displayed URL (usually http://localhost:5173)
```

Then simply type your search query in the interface, and Perplexity results will be included alongside any local results.

## Customizing Your Configuration

Create your own configuration file based on your needs:

```json
{
  "id": "Server",
  "global_shortcut": "Ctrl+Shift+P",
  "roles": {
    "My Research Assistant": {
      "shortname": "my-researcher",
      "name": "My Research Assistant",
      "relevance_function": "title-scorer",
      "theme": "lumen",
      "kg": null,
      "haystacks": [
        {
          "location": "https://api.perplexity.ai",
          "service": "Perplexity",
          "read_only": true,
          "extra_parameters": {
            "model": "sonar-large-online",
            "max_tokens": "2000",
            "temperature": "0.1",
            "cache_ttl_hours": "2",
            "search_domains": "arxiv.org,github.com",
            "search_recency": "month"
          }
        }
      ],
      "extra": {}
    }
  },
  "default_role": "My Research Assistant",
  "selected_role": "My Research Assistant"
}
```

Save this as `my_perplexity_config.json` and run:

```bash
cargo run -- --config my_perplexity_config.json
```

## Monitoring and Debugging

### Enable Debug Logging

```bash
LOG_LEVEL=debug cargo run -- --config perplexity_researcher_config.json
```

This will show:
- API request/response times
- Cache hit/miss statistics
- Token usage information
- Detailed error messages

### Test the Integration

```bash
# Run configuration tests
cargo test -p terraphim_middleware test_perplexity

# Run live API test (requires API key)
cargo test -p terraphim_middleware perplexity_live_api_test -- --ignored
```

## Performance Tips

1. **Use caching effectively**: The default 1-hour cache reduces API costs
2. **Choose the right model**:
   - `sonar-small-online` for quick facts
   - `sonar-medium-online` for balanced performance
   - `sonar-large-online` for complex research
3. **Set appropriate token limits** to control response length and costs
4. **Use domain filtering** for focused, high-quality results

## Troubleshooting

### No Results Returned
- Check that `PERPLEXITY_API_KEY` is set correctly
- Verify network connectivity to api.perplexity.ai
- Look for error messages in the debug logs

### Slow Responses
- Try using `sonar-small-online` model for faster responses
- Reduce `max_tokens` to get shorter responses
- Check if results are being cached (subsequent identical queries should be faster)

### API Errors
- Verify your API key is valid and has sufficient quota
- Check Perplexity's service status
- Review the API usage guidelines for any restrictions

## Next Steps

- **Combine with local search**: Add local document haystacks alongside Perplexity
- **Create specialized roles**: Configure different profiles for different research needs
- **Explore advanced features**: Try domain filtering, recency filters, and different models
- **Monitor usage**: Keep track of API costs and optimize configuration accordingly

---

*This example demonstrates the power of combining Terraphim AI's local knowledge capabilities with Perplexity's real-time web search, creating a comprehensive research and information discovery platform.*
