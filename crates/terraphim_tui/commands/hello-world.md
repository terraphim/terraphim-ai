---
name: hello-world
description: Simple hello world command for testing
usage: "hello-world [name] [--greeting]"
category: Testing
version: "1.0.0"
risk_level: Low
execution_mode: Local
permissions:
  - read
aliases:
  - hello
  - hi
parameters:
  - name: name
    type: string
    required: false
    default_value: "World"
    description: Name to greet
  - name: greeting
    type: string
    required: false
    allowed_values: ["hello", "hi", "hey", "greetings"]
    default_value: "hello"
    description: Greeting type
timeout: 10
---

# Hello World Command

A simple greeting command used for testing the custom command system.

## Examples

```bash
# Basic greeting
hello-world

# Custom name
hello-world --name Alice

# Different greeting
hello-world --greeting hi --name Bob
```

## Parameters

- **name**: Optional. Name to greet (default: "World")
- **greeting**: Optional. Type of greeting (default: "hello")

## Notes

This command demonstrates the markdown-based command definition system with:
- YAML frontmatter metadata
- Parameter validation
- Default values
- Allowed value restrictions