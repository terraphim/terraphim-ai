# TruthForge API Usage Examples

## Quick Start

### Prerequisites
1. Start the Terraphim server:
```bash
cd terraphim-ai
cargo run -p terraphim_server --release
```

2. (Optional) Set OpenRouter API key for real LLM analysis:
```bash
export OPENROUTER_API_KEY=your_key_here
```

Without the API key, the server will use mock implementations.

## REST API Examples

### 1. Submit Narrative for Analysis

**Endpoint**: `POST /api/v1/truthforge`

**Full Request**:
```bash
curl -X POST http://localhost:8080/api/v1/truthforge \
  -H "Content-Type: application/json" \
  -d '{
    "text": "We achieved a 40% cost reduction this quarter through process optimization. This will improve our operational efficiency and deliver value to shareholders.",
    "urgency": "Low",
    "stakes": ["Financial", "Reputational"],
    "audience": "Internal"
  }'
```

**Minimal Request** (uses defaults):
```bash
curl -X POST http://localhost:8080/api/v1/truthforge \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Company announces layoffs affecting 500 employees."
  }'
```

**Response**:
```json
{
  "status": "success",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "analysis_url": "/api/v1/truthforge/550e8400-e29b-41d4-a716-446655440000"
}
```

### 2. Get Analysis Result

**Endpoint**: `GET /api/v1/truthforge/{session_id}`

```bash
SESSION_ID="550e8400-e29b-41d4-a716-446655440000"
curl http://localhost:8080/api/v1/truthforge/${SESSION_ID}
```

**Response** (when complete):
```json
{
  "status": "success",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "narrative": {
      "text": "...",
      "context": {
        "urgency": "Low",
        "stakes": ["Financial"],
        "audience": "Internal"
      }
    },
    "omission_catalog": {
      "omissions": [
        {
          "id": "...",
          "category": "MissingEvidence",
          "description": "No evidence provided for 40% cost reduction claim",
          "severity": 0.8,
          "exploitability": 0.7,
          "composite_risk": 0.56
        }
      ],
      "total_risk_score": 2.34
    },
    "response_strategies": [
      {
        "strategy_type": "Reframe",
        "strategic_rationale": "...",
        "drafts": {
          "social_media": "...",
          "press_statement": "...",
          "internal_memo": "...",
          "qa_brief": []
        }
      }
    ],
    "executive_summary": "...",
    "processing_time_ms": 45000
  },
  "error": null
}
```

**Response** (still processing):
```json
{
  "status": "success",
  "result": null,
  "error": null
}
```

### 3. List All Analyses

**Endpoint**: `GET /api/v1/truthforge/analyses`

```bash
curl http://localhost:8080/api/v1/truthforge/analyses
```

**Response**:
```json
[
  "550e8400-e29b-41d4-a716-446655440000",
  "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "7c9e6679-7425-40de-944b-e07fc1f90ae7"
]
```

## WebSocket Real-Time Progress

### Connect to WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('WebSocket connected');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.message_type === 'truthforge_progress') {
    const { stage, details } = message.data;
    const sessionId = message.session_id;

    console.log(`[${sessionId}] ${stage}:`, details);

    // Handle different stages
    switch (stage) {
      case 'started':
        console.log(`Analysis started for ${details.narrative_length} char narrative`);
        break;

      case 'completed':
        console.log(`Analysis complete!`);
        console.log(`- Omissions detected: ${details.omissions_count}`);
        console.log(`- Response strategies: ${details.strategies_count}`);
        console.log(`- Total risk score: ${details.total_risk_score}`);
        console.log(`- Processing time: ${details.processing_time_ms}ms`);
        break;

      case 'failed':
        console.error(`Analysis failed: ${details.error}`);
        break;
    }
  }
};
```

### Progress Event Examples

**Started Event**:
```json
{
  "message_type": "truthforge_progress",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": {
    "stage": "started",
    "details": {
      "message": "Analysis workflow initiated",
      "narrative_length": 156
    }
  },
  "timestamp": "2025-10-08T12:34:56.789Z"
}
```

**Completed Event**:
```json
{
  "message_type": "truthforge_progress",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": {
    "stage": "completed",
    "details": {
      "omissions_count": 12,
      "strategies_count": 3,
      "total_risk_score": 8.45,
      "processing_time_ms": 45000
    }
  },
  "timestamp": "2025-10-08T12:35:41.789Z"
}
```

**Failed Event**:
```json
{
  "message_type": "truthforge_progress",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": {
    "stage": "failed",
    "details": {
      "error": "LLM request timeout after 60s"
    }
  },
  "timestamp": "2025-10-08T12:35:56.789Z"
}
```

## Complete Workflow Example

### Bash Script

```bash
#!/bin/bash

# 1. Submit narrative
echo "Submitting narrative for analysis..."
RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/truthforge \
  -H "Content-Type: application/json" \
  -d '{
    "text": "We reduced operational costs by 30% through automation.",
    "urgency": "Low",
    "stakes": ["Financial"],
    "audience": "Internal"
  }')

# 2. Extract session ID
SESSION_ID=$(echo $RESPONSE | jq -r '.session_id')
echo "Analysis started. Session ID: $SESSION_ID"

# 3. Poll for results (in production, use WebSocket instead)
echo "Waiting for analysis to complete..."
for i in {1..10}; do
  sleep 5
  RESULT=$(curl -s http://localhost:8080/api/v1/truthforge/${SESSION_ID})

  if [ "$(echo $RESULT | jq -r '.result')" != "null" ]; then
    echo "Analysis complete!"
    echo $RESULT | jq '.result | {
      omissions: .omission_catalog.omissions | length,
      risk_score: .omission_catalog.total_risk_score,
      strategies: .response_strategies | length,
      processing_time: .processing_time_ms
    }'
    break
  else
    echo "Still processing... ($i/10)"
  fi
done
```

### Python Example

```python
import requests
import time
import json

BASE_URL = "http://localhost:8080/api/v1/truthforge"

# 1. Submit narrative
narrative = {
    "text": "Company pivots to sustainable energy solutions.",
    "urgency": "Low",
    "stakes": ["Reputational"],
    "audience": "PublicMedia"
}

response = requests.post(BASE_URL, json=narrative)
session_id = response.json()["session_id"]
print(f"Analysis started: {session_id}")

# 2. Poll for results
max_attempts = 20
for attempt in range(max_attempts):
    time.sleep(3)

    result_response = requests.get(f"{BASE_URL}/{session_id}")
    data = result_response.json()

    if data["result"]:
        print("Analysis complete!")
        result = data["result"]
        print(f"- Omissions detected: {len(result['omission_catalog']['omissions'])}")
        print(f"- Total risk score: {result['omission_catalog']['total_risk_score']:.2f}")
        print(f"- Strategies generated: {len(result['response_strategies'])}")
        print(f"- Processing time: {result['processing_time_ms']}ms")

        # Print top 3 omissions
        print("\nTop 3 Omissions:")
        for i, omission in enumerate(result['omission_catalog']['omissions'][:3], 1):
            print(f"{i}. {omission['description']}")
            print(f"   Severity: {omission['severity']:.2f}, "
                  f"Exploitability: {omission['exploitability']:.2f}")
        break
    else:
        print(f"Processing... ({attempt + 1}/{max_attempts})")
else:
    print("Timeout waiting for results")
```

## Request Parameters

### NarrativeInput

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `text` | string | Yes | - | The narrative text to analyze |
| `urgency` | enum | No | `"Low"` | `"Low"` or `"High"` |
| `stakes` | array | No | `["Reputational"]` | Array of stake types |
| `audience` | enum | No | `"Internal"` | `"PublicMedia"` or `"Internal"` |

### Stake Types

- `"Reputational"` - Brand reputation and public perception
- `"Legal"` - Legal compliance and liability
- `"Financial"` - Financial performance and shareholder value
- `"Operational"` - Business operations and processes
- `"SocialLicense"` - Community trust and social standing

## Response Structure

### TruthForgeAnalysisResult

```typescript
interface TruthForgeAnalysisResult {
  session_id: string;
  narrative: NarrativeInput;

  // Pass One Analysis
  bias_analysis: BiasAnalysis;
  narrative_mapping: NarrativeMapping;
  taxonomy_linking: TaxonomyLinking;
  omission_catalog: OmissionCatalog;

  // Debate Results
  pass_one_debate: DebateResult;
  pass_two_debate: DebateResult;
  cumulative_analysis: CumulativeAnalysis;

  // Response Strategies
  response_strategies: ResponseStrategy[];

  // Summary
  executive_summary: string;
  processing_time_ms: number;
  created_at: string;
}
```

## Error Handling

### Error Response Format

```json
{
  "status": "error",
  "error": "Error message here"
}
```

### Common Errors

| Status Code | Error | Cause |
|-------------|-------|-------|
| 400 | Invalid request | Malformed JSON or missing required fields |
| 404 | Session not found | Invalid session_id |
| 500 | Internal server error | LLM failure or system error |

## Performance Considerations

### Expected Processing Times

- **Mock mode** (no API key): 100-500ms
- **Real LLM** (with OpenRouter):
  - Short narrative (<200 chars): 30-60s
  - Medium narrative (200-500 chars): 60-90s
  - Long narrative (>500 chars): 90-120s

### Best Practices

1. **Use WebSocket for real-time updates** instead of polling
2. **Implement client-side timeout** (2-3 minutes recommended)
3. **Cache session IDs** for result retrieval
4. **Batch multiple narratives** if analyzing many at once
5. **Monitor progress events** to show user feedback

## Testing

Run the integration tests:

```bash
# Run all TruthForge API tests
cargo test -p terraphim_server --test truthforge_api_test

# Run specific test
cargo test -p terraphim_server --test truthforge_api_test test_analyze_narrative_endpoint
```

## Production Deployment

### Environment Variables

```bash
# Required for real LLM analysis
export OPENROUTER_API_KEY=your_openrouter_api_key

# Optional server configuration
export RUST_LOG=info  # Logging level
```

### Future Production Features

- **Redis persistence**: Session storage across server restarts
- **Rate limiting**: 100 requests/hour per user
- **Authentication**: User-based access control
- **Cost tracking**: Per-user analysis cost monitoring
- **Error recovery**: Automatic retry with exponential backoff
