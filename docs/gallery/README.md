# Terraphim Web Components Gallery

A pure vanilla JavaScript Web Components gallery and documentation system, built with no frameworks, no build tools, and no dependencies.

## Phase 1 Implementation - Complete

Phase 1 establishes the core gallery infrastructure with navigation, routing, theming, and responsive design.

### What's Included

#### CSS Foundation
- **theme-light.css** - Light theme design tokens (colors, spacing, typography, shadows)
- **theme-dark.css** - Dark theme overrides
- **gallery.css** - Core gallery styles and component base styles
- **responsive.css** - Mobile-first responsive design with breakpoints

#### JavaScript Utilities
- **router.js** - Hash-based client-side routing
- **search.js** - Search functionality (basic implementation, ready for Fuse.js in Phase 2)

#### Web Components
- **theme-toggle.js** - Dark/light mode switcher with localStorage persistence
- **nav-item.js** - Individual navigation link with active state
- **nav-category.js** - Collapsible navigation category group
- **gallery-header.js** - Top navigation bar with logo, search, and theme toggle
- **gallery-sidebar.js** - Left sidebar with navigation tree
- **gallery-main.js** - Main content area with welcome page and placeholders
- **terraphim-gallery.js** - Root application component orchestrating everything

#### Data Files
- **components.json** - Component metadata for base components
- **nav-structure.json** - Navigation tree structure

#### HTML Entry Point
- **index.html** - Main gallery page with all scripts and styles

## Features

### Core Features (Phase 1)
- Pure vanilla JavaScript - no frameworks, no build tools
- Shadow DOM encapsulation for style isolation
- Hash-based routing for navigation without page reloads
- Theme switching (light/dark) with system preference detection
- Responsive design (mobile, tablet, desktop)
- Mobile menu with overlay
- Keyboard navigation support
- ARIA accessibility attributes
- Component search (basic substring matching)

### Design System
- Consistent spacing scale (xs through 2xl)
- Typography scale with proper line heights
- Color palette with semantic naming
- Elevation system with shadows
- Border radius scale
- Z-index scale for layering
- Transition timing variables

### Accessibility
- Skip links for keyboard navigation
- ARIA labels and roles
- Focus-visible indicators
- Semantic HTML5 structure
- High contrast mode support
- Reduced motion support

## File Structure

```
docs/gallery/
├── index.html                  # Main entry point
├── README.md                   # This file
├── styles/
│   ├── theme-light.css        # Light theme tokens
│   ├── theme-dark.css         # Dark theme overrides
│   ├── gallery.css            # Core styles
│   └── responsive.css         # Responsive breakpoints
├── scripts/
│   ├── router.js              # Client-side routing
│   └── search.js              # Search functionality
└── data/
    ├── components.json        # Component metadata
    └── nav-structure.json     # Navigation structure

components/gallery/
├── terraphim-gallery.js       # Root component
├── gallery-header.js          # Header with search
├── gallery-sidebar.js         # Sidebar navigation
├── gallery-main.js            # Main content area
├── nav-category.js            # Navigation category
├── nav-item.js                # Navigation item
└── theme-toggle.js            # Theme switcher
```

## Usage

### Running Locally

Simply open `index.html` in a modern web browser:

```bash
# Using Python's built-in server
cd docs/gallery
python3 -m http.server 8000

# Or using any other static file server
npx serve .
```

Then navigate to `http://localhost:8000`

### File Protocol

The gallery works with the `file://` protocol for local development:

```bash
open docs/gallery/index.html
```

### Navigation

- Click sidebar items to navigate between pages
- Use browser back/forward buttons
- Routes are hash-based: `#/components/base/terraphim-element`

### Theme Switching

- Click the theme toggle button in the header
- Theme preference is saved to localStorage
- Respects system preference if no saved preference exists

### Mobile Menu

- Tap the hamburger menu (☰) to open sidebar on mobile
- Tap overlay or press Escape to close
- Auto-closes when navigating to a new page

## Adding New Components

### 1. Add Component Metadata

Edit `docs/gallery/data/components.json`:

```json
{
  "id": "my-component",
  "name": "MyComponent",
  "category": "ui",
  "description": "Component description",
  "tags": ["tag1", "tag2"],
  "status": "stable",
  "version": "1.0.0",
  "path": "/components/ui/my-component",
  "file": "components/ui/my-component.js"
}
```

### 2. Add Navigation Entry

Edit `docs/gallery/data/nav-structure.json`:

```json
{
  "id": "my-component",
  "label": "My Component",
  "path": "/components/ui/my-component"
}
```

### 3. Router Will Handle Navigation Automatically

The router will detect the new path and render the component page.

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Opera 76+

Requires support for:
- Custom Elements v1
- Shadow DOM v1
- ES6 Modules
- CSS Custom Properties
- CSS Grid and Flexbox

## What's Next - Phase 2

Phase 2 will add:
- Live component examples with code preview
- Interactive property editors
- Syntax highlighting with Prism.js
- Fuzzy search with Fuse.js
- Component documentation rendering from metadata
- Code copy buttons
- Download/installation instructions
- API documentation tables
- Event documentation
- CSS custom properties documentation

## License

Part of the Terraphim AI project.

## Contributing

This gallery follows the "Zestic Strategy" - pure vanilla implementations with no frameworks or build tools. All contributions must adhere to this constraint.
