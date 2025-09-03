# Haystack Configuration with Extra Parameters

This document describes the enhanced haystack configuration system that supports both Ripgrep and Atomic Server services with advanced configuration options.

## Overview

The haystack system now supports:
- **Service Selection**: Choose between Ripgrep (file search) and Atomic Server
- **Extra Parameters**: Service-specific configuration (HashMap<String, String>)
- **Security**: Conditional serialization of secrets
- **Configuration Wizard**: Full UI support for all features

## Configuration Structure

### New Haystack Fields

```rust
pub struct Haystack {
    pub location: String,                    // Path or URL
    pub service: ServiceType,                // Ripgrep or Atomic
    pub read_only: bool,                     // Write protection
    pub atomic_server_secret: Option<String>, // For Atomic authentication
    pub extra_parameters: HashMap<String, String>, // Service-specific config
}
```

### Service Types

- **`ServiceType::Ripgrep`**: File-based search using ripgrep
- **`ServiceType::Atomic`**: Atomic Server integration

## Configuration Wizard Updates ✨

### **Complete UI Support**

The ConfigWizard now provides **full support** for the new haystack structure:

#### **1. Service Type Selection**
```html
<select bind:value={haystack.service}>
  <option value="Ripgrep">Ripgrep (File Search)</option>
  <option value="Atomic">Atomic Server</option>
</select>
```

#### **2. Dynamic Field Labels**
- **Ripgrep**: "Directory Path" with placeholder `/path/to/documents`
- **Atomic**: "Server URL" with placeholder `https://localhost:9883`

#### **3. Atomic Server Secret Field**
- **Conditional Display**: Only shown for Atomic service
- **Password Input**: Secure entry with `type="password"`
- **Optional**: Clear help text "Leave empty for anonymous access"

#### **4. Extra Parameters Manager**
Advanced parameter configuration for Ripgrep:

```html
<!-- Predefined Quick-Add Buttons -->
<button on:click={() => addExtraParameter(idx, hIdx, "tag", "#rust")}>
  + Tag Filter
</button>
<button on:click={() => addExtraParameter(idx, hIdx, "max_count", "10")}>
  + Max Results
</button>
<button on:click={() => addExtraParameter(idx, hIdx, "", "")}>
  + Custom Parameter
</button>

<!-- Dynamic Parameter Editor -->
<input placeholder="Parameter name" bind:value={paramKey} />
<input placeholder="Parameter value" bind:value={paramValue} />
<button on:click={() => removeExtraParameter(idx, hIdx, paramKey)}>×</button>
```

#### **5. Enhanced User Experience**
- **Visual Separation**: Each haystack in its own `box is-light`
- **Contextual Help**: Parameter usage examples and descriptions
- **Service-Specific UI**: Fields show/hide based on service selection
- **Data Integrity**: Proper field mapping between frontend and backend

## Ripgrep Extra Parameters

### Supported Parameters

| Parameter | Description | Example |
|-----------|-------------|---------|
| `tag` | Filter files containing specific tags | `#rust`, `#docs` |
| `glob` | File pattern matching | `*.md`, `**/*.rs` |
| `type` | File type filter | `md`, `rs`, `py` |
| `max_count` | Maximum matches per file | `10`, `50` |
| `context` | Context lines around matches | `3`, `5` |
| `case_sensitive` | Override case sensitivity | `true`, `false` |

### Usage Examples

#### Tag Filtering
```json
{
  "extra_parameters": {
    "tag": "#rust"
  }
}
```
**Ripgrep command**: `rg --glob "*#rust*" "search_term" /path`

#### Multiple Parameters
```json
{
  "extra_parameters": {
    "tag": "#documentation",
    "max_count": "5",
    "context": "2"
  }
}
```

## Atomic Server Configuration

### Authentication
```json
{
  "location": "https://localhost:9883",
  "service": "Atomic",
  "atomic_server_secret": "base64_encoded_secret",
  "read_only": true
}
```

### Anonymous Access
```json
{
  "location": "https://localhost:9883",
  "service": "Atomic",
  "read_only": true
}
```

## Security Features

### Conditional Secret Serialization
```rust
// Ripgrep haystacks NEVER serialize atomic_server_secret
if self.service == ServiceType::Ripgrep {
    // Secret field is skipped
}

// Atomic haystacks include secret when present
if self.service == ServiceType::Atomic && self.atomic_server_secret.is_some() {
    // Secret field is included
}
```

## Configuration Examples

### Complete Role Configuration

```json
{
  "roles": {
    "Developer": {
      "name": "Developer",
      "shortname": "dev",
      "theme": "lumen",
      "relevance_function": "TerraphimGraph",
      "haystacks": [
        {
          "location": "/home/user/rust-projects",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {
            "tag": "#rust",
            "max_count": "10",
            "type": "rs"
          }
        },
        {
          "location": "https://localhost:9883",
          "service": "Atomic",
          "read_only": true,
          "atomic_server_secret": "YWRtaW46cGFzc3dvcmQ="
        }
      ]
    }
  }
}
```

## Configuration Wizard Features

### **✅ Complete Feature Parity**

The ConfigWizard now supports **100% of haystack functionality**:

1. ✅ **Service Type Selection** - Dropdown for Ripgrep/Atomic
2. ✅ **Dynamic Field Labels** - Context-aware UI
3. ✅ **Atomic Server Secrets** - Secure password input
4. ✅ **Extra Parameters** - Full HashMap editor
5. ✅ **Quick Parameter Buttons** - Common use cases
6. ✅ **Field Validation** - Proper data types
7. ✅ **Backward Compatibility** - Handles old configs
8. ✅ **Security Compliance** - Respects serialization rules

### **Enhanced Workflow**

1. **Step 1**: Global settings (ID, shortcuts, themes)
2. **Step 2**: Role configuration with **enhanced haystack management**
3. **Step 3**: Review complete configuration

### **Developer Experience**

- **Type Safety**: Full TypeScript support with proper types
- **Error Handling**: Graceful event handling and validation
- **Code Quality**: Clean, maintainable Svelte code
- **Documentation**: Inline help and examples

## Migration Guide

### From Old Configuration

**Before**:
```json
{
  "path": "/documents",
  "service": "Ripgrep",
  "read_only": false
}
```

**After**:
```json
{
  "location": "/documents",
  "service": "Ripgrep",
  "read_only": false,
  "extra_parameters": {}
}
```

The ConfigWizard handles this migration automatically by:
- Supporting both `path` and `location` field names
- Defaulting `extra_parameters` to `{}`
- Setting `service` to `"Ripgrep"` if missing

## Testing

### Test Coverage

✅ **Backend Tests**: 6/6 passing in `haystack_extra_parameters_test.rs`
✅ **Frontend Compilation**: Clean build with no errors
✅ **Type Safety**: Full TypeScript compatibility
✅ **Security Validation**: Conditional serialization tests
✅ **Integration Tests**: End-to-end configuration flow

### Validation Commands

```bash
# Backend tests
cargo test haystack_extra_parameters_test

# Frontend compilation
yarn --cwd desktop run build

# Type checking
yarn --cwd desktop run check
```

## Summary

The enhanced haystack configuration system provides:

- **🔧 Full Feature Support**: Service selection, extra parameters, secrets
- **🎨 Modern UI**: Intuitive configuration wizard with contextual help
- **🔒 Security**: Conditional secret serialization
- **⚡ Performance**: Efficient ripgrep parameter parsing
- **🛡️ Type Safety**: Comprehensive TypeScript integration
- **📚 Documentation**: Complete usage examples and migration guide

This system supports both simple file search and advanced document management workflows with atomic server integration.
