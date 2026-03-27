# terraphim_agent/src/client.rs Summary

## Purpose
Provides HTTP client functionality for communicating with the Terraphim server API, handling requests for configuration, chat, document summarization, thesaurus access, and VM management.

## Key Functionality
- Creates HTTP client with configurable timeouts and user agent
- Fetches server configuration via GET /config endpoint
- Resolves role strings to RoleName objects using server config (with fallback)
- Handles chat interactions with LLM models via POST /chat
- Summarizes documents via POST /documents/summarize
- Accesses thesaurus data via GET /thesaurus/{role_name}
- Provides autocomplete suggestions via GET /autocomplete/{role_name}/{query}
- Manages VM operations (listing, status, execution, metrics)
- Supports async document summarization and task management

## Important Details
- Role resolution falls back to creating RoleName from raw string if not found in config (line 69)
- Uses async/await pattern with Tokio for non-blocking HTTP requests
- Implements proper error handling with anyhow::Result
- Includes configurable timeout via TERRAPHIM_CLIENT_TIMEOUT environment variable
- Provides both live API methods and dead-code-allowed variants for testing
- Handles URL encoding for query parameters in various endpoints
- Supports VM pool management, execution, and monitoring capabilities