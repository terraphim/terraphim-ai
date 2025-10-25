# Integration Guide: Terraphim JSON Editor

This guide demonstrates how to integrate the `terraphim-json-editor` component into the Terraphim application.

## Quick Start

### 1. Import the Component

```javascript
// In your main app file or router
import './components/config/terraphim-json-editor.js';
```

### 2. Add to HTML

```html
<terraphim-json-editor
  id="config-editor"
  mode="tree"
  auto-save="true"
  auto-save-delay="2000"
  api-endpoint="/config/"
  tauri-command="update_config">
</terraphim-json-editor>
```

### 3. Initialize with Data

```javascript
const editor = document.getElementById('config-editor');

// Set initial configuration
editor.value = {
  name: "Terraphim Engineer",
  theme: "light",
  relevance_function: "BM25Plus"
};

// Optional: Set JSON Schema
editor.schema = {
  type: "object",
  required: ["name", "relevance_function"],
  properties: {
    name: { type: "string" },
    theme: { type: "string", enum: ["light", "dark"] },
    relevance_function: {
      type: "string",
      enum: ["TitleScorer", "BM25", "BM25F", "BM25Plus", "TerraphimGraph"]
    }
  }
};
```

## Integration with Terraphim Router

### Update Route Handler

In your router configuration (e.g., `terraphim-app-router.js`):

```javascript
// Route: /config/json
case 'config-json':
  const configEditor = document.createElement('terraphim-config-json');

  // Load current configuration
  const currentConfig = await loadConfig(); // Your config loading logic
  configEditor.config = currentConfig;

  // Set schema
  configEditor.schema = getConfigSchema(); // Your schema definition

  // Listen for changes
  configEditor.addEventListener('config-saved', (e) => {
    console.log('Configuration saved:', e.detail.value);
    // Optionally refresh application state
  });

  mainContent.appendChild(configEditor);
  break;
```

## Backend Integration Examples

### Example 1: Tauri Backend

```rust
// In your Tauri backend (src-tauri/src/main.rs)

#[tauri::command]
async fn update_config(config_new: serde_json::Value) -> Result<String, String> {
    // Validate configuration
    // Save to file or database
    // Return success message
    Ok("Configuration updated successfully".to_string())
}

#[tauri::command]
async fn get_config() -> Result<serde_json::Value, String> {
    // Load configuration from file or database
    // Return configuration as JSON
    Ok(serde_json::json!({
        "name": "Terraphim Engineer",
        "theme": "light"
    }))
}

// Register commands in main function
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            update_config,
            get_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Example 2: HTTP Backend (Rust/Axum)

```rust
use axum::{
    extract::Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde_json::Value;

// Save configuration endpoint
async fn save_config(Json(config): Json<Value>) -> Result<Json<Value>, StatusCode> {
    // Validate and save configuration

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Configuration saved"
    })))
}

// Load configuration endpoint
async fn load_config() -> Result<Json<Value>, StatusCode> {
    // Load configuration from storage

    Ok(Json(serde_json::json!({
        "name": "Terraphim Engineer",
        "theme": "light",
        "relevance_function": "BM25Plus"
    })))
}

// Router setup
pub fn create_router() -> Router {
    Router::new()
        .route("/config/", post(save_config))
        .route("/config/", get(load_config))
}
```

### Example 3: HTTP Backend (Salvo)

```rust
use salvo::prelude::*;
use serde_json::Value;

#[handler]
async fn save_config(req: &mut Request, res: &mut Response) {
    let config: Value = req.parse_json().await.unwrap();

    // Validate and save configuration

    res.render(Json(serde_json::json!({
        "success": true,
        "message": "Configuration saved"
    })));
}

#[handler]
async fn load_config(res: &mut Response) {
    // Load configuration from storage

    res.render(Json(serde_json::json!({
        "name": "Terraphim Engineer",
        "theme": "light"
    })));
}

// Router setup
pub fn create_router() -> Router {
    Router::new()
        .push(Router::with_path("/config/").post(save_config).get(load_config))
}
```

## Schema Definition

Define a comprehensive schema for Terraphim configuration:

```javascript
const terraphimConfigSchema = {
  type: "object",
  required: ["name", "relevance_function"],
  properties: {
    name: {
      type: "string",
      minLength: 1,
      description: "Configuration name"
    },
    theme: {
      type: "string",
      enum: ["light", "dark"],
      default: "light",
      description: "UI theme"
    },
    relevance_function: {
      type: "string",
      enum: [
        "TitleScorer",
        "BM25",
        "BM25F",
        "BM25Plus",
        "TerraphimGraph"
      ],
      description: "Search relevance algorithm"
    },
    haystacks: {
      type: "array",
      items: {
        type: "object",
        required: ["name", "service"],
        properties: {
          name: {
            type: "string",
            description: "Haystack name"
          },
          service: {
            type: "string",
            enum: [
              "Ripgrep",
              "AtomicServer",
              "QueryRs",
              "MCP",
              "ClickUp",
              "Logseq"
            ],
            description: "Haystack service type"
          },
          extra_parameters: {
            type: "object",
            description: "Service-specific parameters"
          }
        }
      }
    },
    extra: {
      type: "object",
      properties: {
        llm_provider: {
          type: "string",
          enum: ["ollama", "openrouter"],
          description: "LLM provider"
        },
        ollama_base_url: {
          type: "string",
          format: "uri",
          description: "Ollama server URL"
        },
        ollama_model: {
          type: "string",
          description: "Ollama model name"
        }
      }
    }
  }
};
```

## Event Handling

### Complete Event Listener Setup

```javascript
const editor = document.getElementById('config-editor');

// Configuration change
editor.addEventListener('change', (e) => {
  console.log('Config changed:', e.detail.value);

  // Update application state
  updateAppState(e.detail.value);
});

// Validation errors
editor.addEventListener('validation-error', (e) => {
  console.error('Validation failed:', e.detail.errors);

  // Display error messages to user
  displayValidationErrors(e.detail.errors);
});

// Successful save
editor.addEventListener('config-saved', (e) => {
  console.log('Config saved successfully');

  // Show success notification
  showNotification('Configuration saved', 'success');

  // Optionally reload application components
  reloadComponents();
});

// Save error
editor.addEventListener('save-error', (e) => {
  console.error('Save failed:', e.detail.error);

  // Show error notification
  showNotification('Failed to save configuration', 'error');
});

// Load error
editor.addEventListener('load-error', (e) => {
  console.error('Load failed:', e.detail.error);

  // Show error notification
  showNotification('Failed to load configuration', 'error');
});
```

## Advanced Usage

### Auto-load Configuration on Mount

```javascript
// Create editor with auto-load
const editor = document.createElement('terraphim-json-editor');
editor.setAttribute('auto-load', 'true');
editor.setAttribute('auto-save', 'true');
editor.setAttribute('auto-save-delay', '2000');

document.body.appendChild(editor);
```

### Programmatic Control

```javascript
const editor = document.getElementById('config-editor');

// Get current value
const currentConfig = editor.get();
console.log('Current config:', currentConfig);

// Set new value
editor.set({
  name: "New Config",
  theme: "dark"
});

// Validate
const isValid = editor.validate();
if (!isValid) {
  console.log('Configuration is invalid');
}

// Format JSON
editor.format();

// Save manually
await editor.save();

// Load from backend
await editor.load();
```

### Using with State Management

```javascript
import { TerraphimState } from './base/terraphim-state.js';

const appState = new TerraphimState({
  config: {}
});

const editor = document.getElementById('config-editor');

// Bind editor to state
editor.addEventListener('change', (e) => {
  appState.set('config', e.detail.value);
});

// Update editor when state changes
appState.subscribe('config', (newConfig) => {
  editor.set(newConfig);
});
```

## Styling Integration

### Apply Terraphim Theme

```html
<!-- Wrap editor in themed container -->
<div class="terraphim-app" data-theme="dark">
  <terraphim-json-editor
    id="config-editor">
  </terraphim-json-editor>
</div>
```

### Custom Container Styling

```css
.config-editor-container {
  padding: 2rem;
  background: #f5f5f5;
  border-radius: 8px;
  height: 100vh;
}

.config-editor-container terraphim-json-editor {
  height: calc(100vh - 4rem);
}
```

## Testing Integration

### Unit Test Example

```javascript
describe('TerraphimJsonEditor Integration', () => {
  let editor;

  beforeEach(() => {
    editor = document.createElement('terraphim-json-editor');
    document.body.appendChild(editor);
  });

  afterEach(() => {
    editor.remove();
  });

  it('should load configuration', async () => {
    const testConfig = {
      name: "Test Config",
      theme: "light"
    };

    editor.set(testConfig);
    const result = editor.get();

    expect(result).toEqual(testConfig);
  });

  it('should validate against schema', () => {
    editor.schema = {
      type: "object",
      required: ["name"],
      properties: {
        name: { type: "string" }
      }
    };

    editor.set({ name: "Valid" });
    expect(editor.validate()).toBe(true);

    editor.set({ invalid: "field" });
    expect(editor.validate()).toBe(false);
  });

  it('should emit change events', (done) => {
    editor.addEventListener('change', (e) => {
      expect(e.detail.value).toBeDefined();
      done();
    });

    editor.set({ name: "Changed" });
  });
});
```

## Deployment Considerations

### CDN Dependencies

The component loads dependencies from CDN:
- vanilla-jsoneditor: https://cdn.jsdelivr.net/npm/vanilla-jsoneditor@1.0.6/
- ajv: https://cdn.jsdelivr.net/npm/ajv@8.12.0/

For offline or production use, consider:

1. **Local hosting**: Download and serve libraries locally
2. **Import maps**: Configure import maps for version control
3. **Bundling**: Use build tools if bundling is acceptable

### Performance Optimization

```javascript
// Lazy load editor only when needed
async function loadEditor() {
  await import('./components/config/terraphim-json-editor.js');
  const editor = document.createElement('terraphim-json-editor');
  return editor;
}

// Use when navigating to config page
router.on('/config/json', async () => {
  const editor = await loadEditor();
  mainContent.appendChild(editor);
});
```

## Troubleshooting

### Issue: Editor not initializing

**Solution**: Ensure dependencies are loaded before editor creation

```javascript
// Wait for dependencies
await customElements.whenDefined('terraphim-json-editor');
const editor = document.createElement('terraphim-json-editor');
```

### Issue: Schema validation not working

**Solution**: Set schema before or immediately after setting value

```javascript
editor.schema = mySchema;
editor.value = myData;
```

### Issue: Auto-save triggering too frequently

**Solution**: Increase auto-save delay

```javascript
editor.setAttribute('auto-save-delay', '3000'); // 3 seconds
```

## Migration from Svelte Version

Replace Svelte component:

```svelte
<!-- OLD: Svelte version -->
<script>
  import { JSONEditor } from "svelte-jsoneditor";
  import { invoke } from "@tauri-apps/api/tauri";
</script>

<JSONEditor {content} onChange={handleChange} />
```

With vanilla component:

```html
<!-- NEW: Vanilla version -->
<terraphim-json-editor
  id="editor"
  auto-save="true">
</terraphim-json-editor>

<script type="module">
  import './components/config/terraphim-json-editor.js';

  const editor = document.getElementById('editor');
  editor.value = myConfig;

  editor.addEventListener('change', (e) => {
    handleChange(e.detail.value);
  });
</script>
```

## Complete Integration Example

See `json-editor-demo.html` for a complete, working example demonstrating:
- Full feature set
- Event handling
- Backend integration (mocked)
- Control panel
- Event logging

Run demo:

```bash
cd components/config
python3 -m http.server 8000
open http://localhost:8000/json-editor-demo.html
```
