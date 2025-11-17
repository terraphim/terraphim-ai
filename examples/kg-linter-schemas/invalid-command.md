---
name: "123-invalid-name"
description: "This command has various validation errors"
version: "not-semver"
execution_mode: "unsafe"
risk_level: "extreme"

parameters:
  - name: "param1"
    type: "invalid_type"
    required: true
    default_value: "Should not have default if required"

  - name: "param1"
    type: "string"
    description: "Duplicate parameter name"

  - name: "Invalid Name With Spaces"
    type: "number"

permissions:
  - "invalid-format"
  - "should:be:resource:action"

knowledge_graph_required:
  - "nonexistent concept"
  - "another missing term"

resource_limits:
  max_memory_mb: 0
  max_cpu_time: -5
  max_disk_mb: 999999999999

timeout: 0
---

# Invalid Command Example

This command demonstrates various validation errors that the linter should catch:

## Errors

1. **Command name**: Starts with number "123-invalid-name"
2. **Version**: Not semver format "not-semver"
3. **Execution mode**: Invalid value "unsafe" (should be local/firecracker/hybrid)
4. **Risk level**: Invalid value "extreme" (should be low/medium/high/critical)
5. **Parameter type**: Invalid type "invalid_type"
6. **Required + default**: param1 has both required=true and default_value
7. **Duplicate parameter**: param1 defined twice
8. **Invalid parameter name**: Contains spaces
9. **Permission format**: Should be "resource:action" not "invalid-format"
10. **Resource limits**: Zero or negative values
11. **Timeout**: Cannot be zero
12. **KG concepts**: References nonexistent concepts
