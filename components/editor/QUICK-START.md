# Terraphim Editor - Quick Start Guide

## 1-Minute Setup

### Basic Usage

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>My Editor</title>
</head>
<body>
  <terraphim-editor
    content="# Hello World"
    output-format="markdown">
  </terraphim-editor>

  <script type="module">
    import './components/editor/terraphim-editor.js';
  </script>
</body>
</html>
```

**That's it!** No npm, no build, no configuration.

---

## Common Patterns

### 1. Listen for Content Changes

```javascript
const editor = document.querySelector('terraphim-editor');

editor.addEventListener('content-changed', (event) => {
  const { content, format } = event.detail;
  console.log('Content:', content);
  console.log('Format:', format);
});
```

### 2. Get/Set Content Programmatically

```javascript
// Get content
const markdown = editor.getContent('markdown');
const html = editor.getContent('html');
const text = editor.getContent('text');

// Set content
editor.setContent('# New Content');
```

### 3. Read-Only Mode

```html
<terraphim-editor
  content="This is read-only"
  read-only="true">
</terraphim-editor>
```

Or toggle dynamically:

```javascript
editor.setAttribute('read-only', 'true');  // Enable
editor.setAttribute('read-only', 'false'); // Disable
```

### 4. Split View (Editor + Preview)

```html
<terraphim-editor
  content="# Split View"
  show-preview="true">
</terraphim-editor>
```

### 5. No Toolbar (Minimal)

```html
<terraphim-editor
  content="# Minimal"
  show-toolbar="false">
</terraphim-editor>
```

### 6. Dark Theme

```html
<terraphim-editor
  content="# Dark Mode"
  theme="dark">
</terraphim-editor>
```

---

## All Attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `content` | string | `""` | Initial markdown content |
| `output-format` | string | `"markdown"` | Output format: `markdown`, `html`, `text` |
| `read-only` | boolean | `false` | Prevent editing |
| `show-toolbar` | boolean | `true` | Show formatting toolbar |
| `show-preview` | boolean | `false` | Show live preview pane |
| `theme` | string | `"light"` | Theme: `light`, `dark` |

**Phase 2+ attributes** (not yet implemented):
- `enable-autocomplete`
- `role`
- `show-snippets`
- `suggestion-trigger`
- `max-suggestions`
- `min-query-length`
- `debounce-delay`

---

## All Methods

```javascript
// Get content in specific format
editor.getContent('markdown')  // Returns markdown string
editor.getContent('html')      // Returns HTML string
editor.getContent('text')      // Returns plain text

// Set content
editor.setContent('# New content')

// Get current tier
editor.getEditorTier()  // Returns 'core' (Phase 1)

// Phase 2+ methods
editor.rebuildAutocompleteIndex()  // Not yet implemented
```

---

## All Events

```javascript
// Fired when content changes
editor.addEventListener('content-changed', (event) => {
  const { content, format } = event.detail;
});

// Fired when tier is detected on load
editor.addEventListener('tier-detected', (event) => {
  const { tier } = event.detail;  // 'core' in Phase 1
});

// Fired on keyboard events in editor
editor.addEventListener('editor-keydown', (event) => {
  const { key, ctrlKey, metaKey } = event.detail;
});
```

---

## Toolbar Buttons

| Button | Markdown | Shortcut |
|--------|----------|----------|
| **B** | `**bold**` | Ctrl+B |
| *I* | `*italic*` | Ctrl+I |
| Code | `` `code` `` | - |
| Link | `[text](url)` | - |
| H | `# heading` | - |
| List | `- item` | - |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+B (Cmd+B) | Bold |
| Ctrl+I (Cmd+I) | Italic |
| Tab | Insert 2 spaces |

---

## Styling and Theming

### CSS Custom Properties

```css
/* Override theme colors */
terraphim-editor {
  --editor-bg-primary: #ffffff;
  --editor-text-primary: #333333;
  --editor-border-primary: #dddddd;
  --editor-color-primary: #0066cc;
  --editor-font-family-base: 'My Font', sans-serif;
  --editor-font-size-base: 16px;
}
```

### Built-in Themes

```html
<!-- Light theme (default) -->
<terraphim-editor theme="light"></terraphim-editor>

<!-- Dark theme -->
<terraphim-editor theme="dark"></terraphim-editor>
```

### Custom Theme

```css
terraphim-editor[theme="custom"] {
  --editor-bg-primary: #f0f8ff;
  --editor-text-primary: #003366;
  --editor-border-primary: #0066cc;
  --editor-color-primary: #0099ff;
}
```

---

## Examples

### Multiple Editors on One Page

```html
<terraphim-editor id="editor1" content="# Editor 1"></terraphim-editor>
<terraphim-editor id="editor2" content="# Editor 2"></terraphim-editor>
<terraphim-editor id="editor3" content="# Editor 3"></terraphim-editor>

<script type="module">
  import './components/editor/terraphim-editor.js';

  document.querySelectorAll('terraphim-editor').forEach((editor, index) => {
    editor.addEventListener('content-changed', (event) => {
      console.log(`Editor ${index + 1}:`, event.detail.content);
    });
  });
</script>
```

### Sync Two Editors

```javascript
const editor1 = document.getElementById('editor1');
const editor2 = document.getElementById('editor2');

editor1.addEventListener('content-changed', (event) => {
  editor2.setContent(event.detail.content);
});
```

### Save to LocalStorage

```javascript
const editor = document.querySelector('terraphim-editor');

// Load saved content
const saved = localStorage.getItem('editor-content');
if (saved) {
  editor.setContent(saved);
}

// Auto-save on change
editor.addEventListener('content-changed', (event) => {
  localStorage.setItem('editor-content', event.detail.content);
});
```

### Character Counter

```html
<terraphim-editor id="editor"></terraphim-editor>
<div id="counter">0 characters</div>

<script type="module">
  import './components/editor/terraphim-editor.js';

  const editor = document.getElementById('editor');
  const counter = document.getElementById('counter');

  editor.addEventListener('content-changed', (event) => {
    const length = event.detail.content.length;
    counter.textContent = `${length} characters`;
  });
</script>
```

---

## File Protocol Testing

Open any example directly in your browser:

```
file:///path/to/terraphim-ai/examples/editor/basic.html
file:///path/to/terraphim-ai/examples/editor/test-phase1.html
file:///path/to/terraphim-ai/examples/editor/visual-test.html
```

No server needed!

---

## Troubleshooting

### Editor doesn't render

**Check**:
1. ES6 modules enabled (modern browser required)
2. Correct import path in `<script type="module">`
3. Browser console for errors

### Events not firing

**Check**:
1. Event listener added after element is in DOM
2. Using correct event name (`content-changed`, not `contentChanged`)
3. Event detail structure: `event.detail.content` not `event.content`

### Styles not applied

**Check**:
1. Shadow DOM isolation (styles inside component are separate)
2. CSS custom properties set on `<terraphim-editor>` element
3. Theme attribute is correctly set

### Content not updating

**Check**:
1. Using `setContent()` method, not setting `content` property directly
2. Read-only mode is not enabled
3. Editor is fully initialized (listen for `tier-detected` event)

---

## Browser Compatibility

**Supported**:
- ✅ Chrome 90+
- ✅ Firefox 88+
- ✅ Safari 14+
- ✅ Edge 90+

**Not Supported**:
- ❌ Internet Explorer (no Custom Elements support)
- ❌ Older browsers without ES6 modules

---

## File Structure

```
components/editor/
├── terraphim-editor.js           # Main component (import this)
├── vanilla-markdown-editor.js    # Core editor (auto-imported)
└── styles/
    ├── editor-base.css           # (injected automatically)
    └── markdown-syntax.css       # (injected automatically)

examples/editor/
├── basic.html                    # Full demo
├── test-phase1.html              # Simple test
└── visual-test.html              # Theme showcase
```

**You only need to import**: `terraphim-editor.js`

Everything else is loaded automatically.

---

## Next Steps

1. **Try the examples**: Open `examples/editor/basic.html`
2. **Read the docs**: See `README.md` for full documentation
3. **Check the summary**: See `PHASE1-SUMMARY.md` for implementation details
4. **Wait for Phase 2**: Autocomplete coming soon!

---

## Questions?

- Full docs: `components/editor/README.md`
- Implementation details: `components/editor/PHASE1-SUMMARY.md`
- Source code: `components/editor/terraphim-editor.js`

**Happy editing!** 🚀
