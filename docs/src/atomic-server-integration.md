# Atomic Server Integration

## Overview

Terraphim provides comprehensive integration with Atomic Data servers, enabling both read and write operations for document management and search functionality. This integration supports both public access (for reading documents) and authenticated access (for writing documents).

## Features

### ✅ **Public Access (Read Operations)**
- Access public documents without authentication
- Search across atomic server content
- Integrate with existing haystack infrastructure
- Perfect for read-only use cases

### ✅ **Authenticated Access (Write Operations)**
- Save articles and documents to atomic server
- Manage atomic server resources
- Full CRUD operations with proper authentication
- Secure access with base64-encoded secrets

### ✅ **Dual Haystack Integration**
- Combine atomic server with ripgrep haystacks
- Unified search across multiple data sources
- Configurable read-only and read-write access

## Configuration

### Environment Setup

Create a `.env` file in the project root with atomic server configuration:

```bash
# Atomic Server Configuration
ATOMIC_SERVER_URL=http://localhost:9883/
ATOMIC_SERVER_SECRET=your_base64_encoded_secret_here
```

### Role Configuration

Configure roles to use atomic server haystacks:

```json
{
  "id": "Server",
  "roles": {
    "Atomic Reader": {
      "shortname": "AtomicReader",
      "name": "Atomic Reader",
      "relevance_function": "title-scorer",
      "theme": "spacelab",
      "kg": null,
      "haystacks": [
        {
          "location": "http://localhost:9883/",
          "service": "Atomic",
          "read_only": true,
          "atomic_server_secret": null
        }
      ],
      "extra": {},
      "terraphim_it": false
    },
    "Atomic Writer": {
      "shortname": "AtomicWriter",
      "name": "Atomic Writer",
      "relevance_function": "title-scorer",
      "theme": "darkly",
      "kg": null,
      "haystacks": [
        {
          "location": "http://localhost:9883/",
          "service": "Atomic",
          "read_only": false,
          "atomic_server_secret": "your_base64_encoded_secret"
        }
      ],
      "extra": {},
      "terraphim_it": false
    }
  }
}
```

## API Endpoints

### Search Documents
```bash
POST /documents/search
Content-Type: application/json

{
  "search_term": "your search query",
  "role": "Atomic Reader",
  "limit": 10
}
```

### Save Article to Atomic Server
```bash
POST /api/atomic/save
Content-Type: application/json
Authorization: Bearer your_base64_encoded_secret

{
  "title": "Article Title",
  "content": "Article content...",
  "description": "Article description",
  "tags": ["tag1", "tag2"]
}
```

## Testing

### Running Atomic Server Tests

```bash
# Run all atomic server tests
yarn run test:atomic:only

# Run specific test suites
yarn run test:atomic:secret    # Authentication tests
yarn run test:atomic:save      # Save widget tests
yarn run test:atomic:connection # Connection tests
```

### Test Coverage

The atomic server integration includes comprehensive test coverage:

- **Public Access Tests**: Validate read-only access to atomic server documents
- **Authentication Tests**: Verify secret validation and authenticated access
- **Save Widget Tests**: Test UI and API for saving articles
- **Error Handling Tests**: Ensure graceful handling of network and authentication errors
- **Dual Haystack Tests**: Validate integration with ripgrep and other haystacks

### Test Results

**Current Status**: 7/7 atomic haystack tests passing (100% success rate)

- ✅ Atomic server connectivity
- ✅ Configuration management
- ✅ Search functionality (all searches returning results)
- ✅ Dual haystack integration (Atomic + Ripgrep)
- ✅ Error handling
- ✅ CI-friendly features
- ✅ Environment variable loading

## Best Practices

### 1. **Use Public Access for Read Operations**
```json
{
  "atomic_server_secret": null  // Use null for public access
}
```

### 2. **Proper Secret Management**
- Store secrets in `.env` file (not in version control)
- Use base64-encoded secrets for authenticated access
- Validate secret format before use

### 3. **Error Handling**
- Always handle network connectivity issues
- Validate atomic server availability
- Provide fallback for authentication failures

### 4. **Environment Variables**
```bash
# Load from project root in tests
config({ path: '../../.env' });
```

### 5. **Test Configuration**
- Use correct enum values (`"Server"`, `"Desktop"`, `"Embedded"`)
- Include multiple status codes in test expectations
- Add proper timeouts for network operations

## Troubleshooting

### Common Issues

1. **Base64 Decode Errors**
   - Ensure secret is properly base64-encoded
   - Check secret format in `.env` file
   - Use public access if authentication not required

2. **Network Connectivity**
   - Verify atomic server is running on correct port
   - Check firewall and network settings
   - Validate URL format (include trailing slash)

3. **Authentication Issues**
   - Verify secret is valid and properly formatted
   - Check atomic server authentication settings
   - Use public access for read-only operations

4. **Test Failures**
   - Ensure environment variables are loaded
   - Check server startup time (may need longer timeouts)
   - Validate test configuration matches server expectations

### Debug Commands

```bash
# Test atomic server connectivity
curl -s -H "Accept: application/json" "http://localhost:9883/agents"

# Test authenticated access
curl -s -H "Accept: application/json" -H "Authorization: Bearer $ATOMIC_SERVER_SECRET" "http://localhost:9883/agents"

# Validate secret format
echo "$ATOMIC_SERVER_SECRET" | base64 -d
```

## Integration Examples

### Basic Search Integration
```javascript
// Search atomic server documents
const response = await fetch('/documents/search', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    search_term: 'test',
    role: 'Atomic Reader',
    limit: 10
  })
});
```

### Save Article Integration
```javascript
// Save article to atomic server
const saveResponse = await fetch('/api/atomic/save', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${atomicServerSecret}`
  },
  body: JSON.stringify({
    title: 'My Article',
    content: 'Article content...',
    description: 'Article description',
    tags: ['tag1', 'tag2']
  })
});
```

## Performance Considerations

- **Public Access**: Faster, no authentication overhead
- **Authenticated Access**: Secure but adds authentication latency
- **Dual Haystack**: Combines multiple data sources efficiently
- **Caching**: Results are cached for improved performance

## Security

- **Public Access**: Safe for read-only operations
- **Authenticated Access**: Requires valid base64-encoded secret
- **Secret Management**: Store secrets securely, never in version control
- **Network Security**: Use HTTPS in production environments

## Future Enhancements

- **Advanced Search**: Full-text search with relevance scoring
- **Batch Operations**: Bulk import/export of documents
- **Real-time Updates**: Live synchronization with atomic server
- **Advanced Permissions**: Role-based access control
- **Offline Support**: Local caching and offline operations
