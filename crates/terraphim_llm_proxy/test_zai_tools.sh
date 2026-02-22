#!/bin/bash
# Test ZAI with tools to see the exact error

echo "=== Testing ZAI with properly formatted tools ==="

# Test with correct OpenAI tool format
curl -X POST "https://api.z.ai/api/coding/paas/v4/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer REDACTED_ZAI_KEY" \
  -d '{
    "model": "glm-4.7",
    "messages": [
      {"role": "user", "content": "What is the weather in Beijing?"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_weather",
          "description": "Get weather for a city",
          "parameters": {
            "type": "object",
            "properties": {
              "city": {"type": "string", "description": "City name"}
            },
            "required": ["city"]
          }
        }
      }
    ],
    "tool_choice": "auto",
    "max_tokens": 50
  }' 2>&1 | jq .

echo -e "\n=== Testing ZAI without tools (should work) ==="
curl -X POST "https://api.z.ai/api/coding/paas/v4/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer REDACTED_ZAI_KEY" \
  -d '{
    "model": "glm-4.7",
    "messages": [
      {"role": "user", "content": "Hello"}
    ],
    "max_tokens": 10
  }' 2>&1 | jq -r '.choices[0].message.content'
