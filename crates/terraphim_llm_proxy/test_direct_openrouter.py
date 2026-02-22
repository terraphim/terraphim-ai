#!/usr/bin/env python3
import requests
import json
import os

# Test direct OpenRouter API call with exact same format as proxy
url = "https://openrouter.ai/api/v1/chat/completions"
headers = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {os.environ['OPENROUTER_API_KEY']}",
    "HTTP-Referer": "https://terraphim.ai",
    "X-Title": "Terraphim LLM Proxy"
}

data = {
    "model": "anthropic/claude-3.5-sonnet",
    "messages": [{"role": "user", "content": "Hello, what is 2+2?"}],
    "max_tokens": 50
}

response = requests.post(url, headers=headers, json=data)
print(f"Status: {response.status_code}")
print(f"Headers: {dict(response.headers)}")
print(f"Response: {response.text}")
