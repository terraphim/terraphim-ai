# Terraphim JSON Editor Component

A comprehensive, vanilla JavaScript Web Component for editing JSON configurations with schema validation, auto-save, and dual backend support (Tauri/HTTP).

## Features

- **Multiple View Modes**: Tree, text, and table views
- **Schema Validation**: Real-time JSON Schema validation using AJV
- **Auto-save**: Configurable debounced auto-save
- **Dual Backend Support**: Works with both Tauri invoke and HTTP fetch
- **Keyboard Shortcuts**: Ctrl+S/Cmd+S to save
- **Format & Repair**: Built-in JSON formatting and repair
- **Event System**: Comprehensive event emission for integration
- **Dark Theme Support**: Automatic dark theme integration
- **No Build Required**: Pure vanilla JavaScript, CDN dependencies

## Installation

### As a Module

```javascript
import './components/config/terraphim-json-editor.js';
```

### In HTML

```html
<script type="module" src="./components/config/terraphim-json-editor.js"></script>
```

## Usage

### Basic Usage

```html
<terraphim-json-editor
  mode="tree"
  auto-save="true"
  auto-save-delay="2000">
</terraphim-json-editor>
```

### With Schema Validation

```javascript
const editor = document.querySelector('terraphim-json-editor');

// Set JSON Schema
editor.schema = {
  type: "object",
  required: ["name", "version"],
  properties: {
    name: { type: "string" },
    version: { type: "string" },
    config: { type: "object" }
  }
};

// Set initial value
editor.value = {
  name: "My Config",
  version: "1.0.0",
  config: {}
};
```

### Loading Configuration

```javascript
// Auto-load on mount
<terraphim-json-editor auto-load="true"></terraphim-json-editor>

// Manual load
const editor = document.querySelector('terraphim-json-editor');
await editor.load();
```

### Saving Configuration

```javascript
const editor = document.querySelector('terraphim-json-editor');

// Manual save
await editor.save();

// Auto-save on changes
editor.setAttribute('auto-save', 'true');
editor.setAttribute('auto-save-delay', '1000');
```

## Properties/Attributes

| Property | Attribute | Type | Default | Description |
|----------|-----------|------|---------|-------------|
| `value` | - | Object | `{}` | Current JSON value |
| `schema` | - | Object | `null` | JSON Schema for validation |
| `mode` | `mode` | String | `'tree'` | Editor mode: 'tree', 'text', or 'table' |
| `readOnly` | `read-only` | Boolean | `false` | Read-only mode |
| `autoLoad` | `auto-load` | Boolean | `false` | Auto-load on mount |
| `autoSave` | `auto-save` | Boolean | `false` | Auto-save on change |
| `autoSaveDelay` | `auto-save-delay` | Number | `1000` | Auto-save debounce delay (ms) |
| `mainMenuBar` | `main-menu-bar` | Boolean | `true` | Show main menu bar |
| `navigationBar` | `navigation-bar` | Boolean | `true` | Show navigation bar |
| `statusBar` | `status-bar` | Boolean | `true` | Show status bar |
| `apiEndpoint` | `api-endpoint` | String | `'/config/'` | HTTP API endpoint |
| `tauriCommand` | `tauri-command` | String | `'update_config'` | Tauri command name |

## Methods

### `get()`

Get current JSON value from editor.

```javascript
const value = editor.get();
console.log(value);
```

**Returns:** `Object` - Current JSON value

---

### `set(value)`

Set JSON value in editor.

```javascript
editor.set({
  name: "New Config",
  version: "2.0.0"
});
```

**Parameters:**
- `value` (Object) - New JSON value

---

### `validate()`

Validate current content against schema.

```javascript
const isValid = editor.validate();
if (!isValid) {
  console.log('Validation failed');
}
```

**Returns:** `boolean` - True if valid

---

### `format()`

Format/beautify current JSON.

```javascript
editor.format();
```

---

### `repair()`

Attempt to repair invalid JSON.

```javascript
await editor.repair();
```

---

### `save()`

Save current content to backend.

```javascript
await editor.save();
```

**Returns:** `Promise<void>`

---

### `load()`

Load content from backend.

```javascript
await editor.load();
```

**Returns:** `Promise<void>`

## Events

All events include a `detail` object with relevant data.

### `change`

Emitted when content changes.

```javascript
editor.addEventListener('change', (e) => {
  console.log('New value:', e.detail.value);
  console.log('Previous value:', e.detail.previousValue);
});
```

**Detail:**
- `value` (Object) - New value
- `previousValue` (Object) - Previous value

---

### `blur`

Emitted when editor loses focus.

```javascript
editor.addEventListener('blur', () => {
  console.log('Editor lost focus');
});
```

---

### `focus`

Emitted when editor gains focus.

```javascript
editor.addEventListener('focus', () => {
  console.log('Editor gained focus');
});
```

---

### `validation-error`

Emitted when validation fails.

```javascript
editor.addEventListener('validation-error', (e) => {
  console.log('Validation errors:', e.detail.errors);
});
```

**Detail:**
- `errors` (Array) - AJV validation errors
- `error` (Error) - Error object if validation failed

---

### `config-saved`

Emitted after successful save.

```javascript
editor.addEventListener('config-saved', (e) => {
  console.log('Saved value:', e.detail.value);
});
```

**Detail:**
- `value` (Object) - Saved value

---

### `config-loaded`

Emitted after successful load.

```javascript
editor.addEventListener('config-loaded', (e) => {
  console.log('Loaded value:', e.detail.value);
});
```

**Detail:**
- `value` (Object) - Loaded value

---

### `save-error`

Emitted when save fails.

```javascript
editor.addEventListener('save-error', (e) => {
  console.error('Save failed:', e.detail.error);
});
```

**Detail:**
- `error` (Error) - Error object

---

### `load-error`

Emitted when load fails.

```javascript
editor.addEventListener('load-error', (e) => {
  console.error('Load failed:', e.detail.error);
});
```

**Detail:**
- `error` (Error) - Error object

## Backend Integration

### Tauri Mode

When `window.__TAURI__` is detected, the editor uses Tauri invoke:

```javascript
// Save
await invoke('update_config', { configNew: content });

// Load
const content = await invoke('get_config');
```

**Required Tauri Commands:**
- `update_config` - Save configuration
- `get_config` - Load configuration

### HTTP Mode

When not in Tauri environment, uses HTTP fetch:

```javascript
// Save (POST)
POST /config/
Content-Type: application/json
Body: { ...config }

// Load (GET)
GET /config/
Accept: application/json
```

**Default Endpoint:** `/config/`

Configure custom endpoint:

```html
<terraphim-json-editor api-endpoint="/api/config/"></terraphim-json-editor>
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+S / Cmd+S | Save configuration |

## Styling

The component uses Shadow DOM for style encapsulation and includes:

- Vanilla JSONEditor dark theme
- Helper text box (blue border)
- Status indicators (success, error, info)
- Dark theme support via `data-theme="dark"` attribute on parent

### Custom Styling

To customize appearance, wrap in a container and use CSS variables:

```html
<div data-theme="dark">
  <terraphim-json-editor></terraphim-json-editor>
</div>
```

## Dependencies

Loaded via CDN (no build step required):

- **vanilla-jsoneditor** v1.0.6 - JSON editor UI
- **ajv** v8.12.0 - JSON Schema validation

## Browser Support

- Modern browsers with Web Components support
- ES6+ JavaScript required
- Shadow DOM required

## Example: Complete Integration

```html
<!DOCTYPE html>
<html>
<head>
  <title>Config Editor</title>
</head>
<body>
  <terraphim-json-editor
    id="config-editor"
    mode="tree"
    auto-save="true"
    auto-save-delay="2000"
    main-menu-bar="true"
    navigation-bar="true"
    status-bar="true">
  </terraphim-json-editor>

  <script type="module">
    import './components/config/terraphim-json-editor.js';

    const editor = document.getElementById('config-editor');

    // Set schema
    editor.schema = {
      type: "object",
      required: ["name"],
      properties: {
        name: { type: "string", minLength: 1 },
        theme: { type: "string", enum: ["light", "dark"] }
      }
    };

    // Set initial value
    editor.value = {
      name: "My Config",
      theme: "light"
    };

    // Listen for changes
    editor.addEventListener('change', (e) => {
      console.log('Config changed:', e.detail.value);
    });

    // Listen for save events
    editor.addEventListener('config-saved', () => {
      console.log('Configuration saved successfully');
    });

    // Listen for validation errors
    editor.addEventListener('validation-error', (e) => {
      console.error('Validation failed:', e.detail.errors);
    });
  </script>
</body>
</html>
```

## Testing

A comprehensive demo page is available at `json-editor-demo.html` demonstrating:

- Mode switching (tree, text, table)
- Auto-save configuration
- Manual save/load operations
- Schema validation
- Format and repair functions
- Event logging
- Read-only mode
- Keyboard shortcuts
- Mock backend integration

To run the demo:

```bash
# Serve the component directory
python3 -m http.server 8000

# Open demo in browser
open http://localhost:8000/components/config/json-editor-demo.html
```

## Migration from Svelte

This component is a vanilla JavaScript replacement for the Svelte `ConfigJsonEditor.svelte`. Key differences:

- No build step required
- Shadow DOM encapsulation
- Consistent API across all frameworks
- CDN-based dependencies
- Same visual appearance and functionality

## Troubleshooting

### Editor not loading

Ensure CDN dependencies are accessible. Check browser console for errors.

### Validation not working

Verify schema is set before content:

```javascript
editor.schema = mySchema;
editor.value = myData;
```

### Auto-save not triggering

Check that `auto-save` attribute is set:

```javascript
editor.setAttribute('auto-save', 'true');
```

### Dark theme not applying

Ensure parent element has `data-theme="dark"` attribute:

```html
<div data-theme="dark">
  <terraphim-json-editor></terraphim-json-editor>
</div>
```

## License

Part of the Terraphim AI project.
