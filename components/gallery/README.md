# Terraphim Component Gallery

A comprehensive component documentation and showcase system built with vanilla JavaScript Web Components.

## Overview

The Terraphim Component Gallery is a self-documenting system that provides:

- **Interactive Component Preview**: Browse and explore all Terraphim components
- **Live Code Examples**: View syntax-highlighted code with copy functionality
- **Complete Documentation**: Properties, methods, events, and usage examples
- **Search & Filter**: Find components quickly by name, category, or tags
- **Theme Support**: Light/dark theme with persistence
- **Responsive Layout**: Grid and list view options

## Architecture

### Core Components

1. **terraphim-gallery.js** - Main container managing layout and state
2. **terraphim-sidebar.js** - Category navigation with component counts
3. **terraphim-search.js** - Debounced search with keyboard shortcuts (Cmd/Ctrl+K)
4. **terraphim-component-card.js** - Component preview cards (grid/list views)
5. **terraphim-code-viewer.js** - Syntax-highlighted code display
6. **terraphim-tabs.js** - Tab navigation (Demo/Code/Docs)
7. **terraphim-theme-switcher.js** - Light/dark theme toggle
8. **terraphim-layout-switcher.js** - Grid/list view toggle

### State Management

The gallery uses **TerraphimState** for centralized state management:

```javascript
{
  view: 'grid',              // 'grid' | 'list'
  theme: 'light',            // 'light' | 'dark'
  searchQuery: '',
  selectedCategory: 'all',
  selectedTags: [],
  components: [],            // Loaded from .meta.json files
  currentComponent: null,    // Currently viewing
  currentTab: 'demo'         // 'demo' | 'code' | 'docs'
}
```

### Documentation Format

Components are documented using `.meta.json` files in `components/gallery/data/`:

```json
{
  "name": "ComponentName",
  "category": "base",
  "tags": ["tag1", "tag2"],
  "description": "Component description",
  "properties": [
    {
      "name": "propName",
      "type": "String",
      "default": "defaultValue",
      "description": "Property description"
    }
  ],
  "methods": [
    {
      "name": "methodName",
      "params": ["param1: Type", "param2: Type"],
      "returns": "Type",
      "description": "Method description"
    }
  ],
  "events": [
    {
      "name": "eventName",
      "detail": "Event payload",
      "description": "When this fires"
    }
  ],
  "examples": [
    {
      "title": "Example Title",
      "code": "// Code example"
    }
  ]
}
```

## Usage

### Running the Gallery

1. **Local Development**:
   ```bash
   # From components/gallery directory
   python3 -m http.server 8080
   # Open http://localhost:8080
   ```

2. **Direct File Access**:
   ```bash
   # Open in browser (file:// protocol supported)
   open index.html
   ```

### Adding New Components

1. Create a `.meta.json` file in `components/gallery/data/`:
   ```json
   {
     "name": "MyComponent",
     "category": "custom",
     "tags": ["ui", "interactive"],
     "description": "My awesome component",
     "properties": [...],
     "methods": [...],
     "events": [...],
     "examples": [...]
   }
   ```

2. Update `terraphim-gallery.js` to load the new metadata file:
   ```javascript
   const metaFiles = [
     'terraphim-element.meta.json',
     'terraphim-state.meta.json',
     'state-helpers.meta.json',
     'my-component.meta.json'  // Add here
   ];
   ```

3. The gallery will automatically display the new component!

### Categories

Built-in categories:
- `all` - All components
- `base` - Base/core components
- `gallery` - Gallery-specific components
- `examples` - Example components

Add custom categories by updating `terraphim-sidebar.js`:
```javascript
this.categories = {
  'all': 'All Components',
  'base': 'Base Components',
  'custom': 'Custom Components'  // Add here
};
```

## Features

### Search

- **Real-time search** with 300ms debounce
- Searches component names, descriptions, and tags
- **Keyboard shortcut**: `Cmd/Ctrl + K` to focus search
- Clear button to reset search

### Filtering

- Filter by category in sidebar
- Active category highlighting
- Component count badges

### Themes

- Light and dark themes
- Persists to localStorage
- CSS custom properties for easy customization:
  ```css
  --color-bg
  --color-bg-secondary
  --color-text
  --color-text-secondary
  --color-border
  --color-primary
  --color-primary-hover
  ```

### Views

- **Grid view**: Card layout for browsing
- **List view**: Compact list for scanning
- Preference persists to localStorage

### Code Viewer

- Syntax highlighting for JavaScript
- Line numbers
- Copy to clipboard
- File name display

### Tab Navigation

- Demo, Code, and Documentation tabs
- Keyboard navigation: `Alt + ← →`
- State-based active highlighting

## Testing

Run the test suite:

```bash
# Open test-gallery.html in browser
open test-gallery.html
```

The test suite validates:
- State management
- Component definitions
- Theme switching
- View toggling
- Search functionality
- Category selection
- Metadata loading

## File Structure

```
components/gallery/
├── data/                           # Component metadata
│   ├── terraphim-element.meta.json
│   ├── terraphim-state.meta.json
│   └── state-helpers.meta.json
├── terraphim-gallery.js            # Main container
├── terraphim-sidebar.js            # Navigation
├── terraphim-search.js             # Search input
├── terraphim-component-card.js     # Component cards
├── terraphim-code-viewer.js        # Code display
├── terraphim-tabs.js               # Tab navigation
├── terraphim-theme-switcher.js     # Theme toggle
├── terraphim-layout-switcher.js    # View toggle
├── index.html                      # Gallery entry point
├── test-gallery.html               # Test suite
└── README.md                       # This file
```

## Dependencies

**None!** The gallery is built with pure vanilla JavaScript:

- No frameworks (React, Vue, etc.)
- No build tools (webpack, vite, etc.)
- No npm packages
- No TypeScript compilation

Uses only:
- Web Components (Custom Elements API)
- Shadow DOM
- TerraphimElement base class
- TerraphimState for state management

## Browser Support

Tested and working in:
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

Requires:
- ES6+ JavaScript
- Custom Elements v1
- Shadow DOM v1
- CSS Grid
- CSS Custom Properties

## Implementation Details

### No Build Step

All components work via `file://` protocol - no server required! This is achieved by:
- Using ES6 modules with relative paths
- No external dependencies
- Pure CSS (no preprocessors)
- Inline templates with template literals

### State Management

Gallery state uses TerraphimState with:
- Path-based subscriptions (`gallery.view`, `gallery.theme`)
- localStorage persistence
- Automatic cleanup on component disconnect

### Component Patterns

All components follow the same pattern:
```javascript
class MyComponent extends TerraphimElement {
  static get properties() {
    return { /* property definitions */ };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    this.bindState(galleryState, 'path', 'property');
  }

  render() {
    this.setHTML(this.shadowRoot, `...`);
  }
}

customElements.define('my-component', MyComponent);
```

### Performance

- Debounced search (300ms)
- Scheduled renders with `requestAnimationFrame`
- Event delegation where applicable
- Lazy metadata loading

## Future Enhancements

Potential improvements:
- [ ] Live demo iframe component
- [ ] Interactive code editor
- [ ] Component playground
- [ ] Export documentation as markdown
- [ ] Screenshot generation
- [ ] Accessibility audit tool
- [ ] Performance metrics

## License

Part of the Terraphim AI project.

## Contributing

To add documentation for your component:

1. Create a `.meta.json` file following the schema
2. Add comprehensive examples
3. Include all public properties, methods, and events
4. Test in the gallery
5. Submit a PR

For questions or issues, see the main Terraphim AI repository.
