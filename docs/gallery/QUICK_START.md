# Quick Start Guide

## Running the Gallery Locally

### Option 1: Python HTTP Server (Recommended)

```bash
cd docs/gallery
python3 -m http.server 8000
```

Then open http://localhost:8000 in your browser.

### Option 2: File Protocol

Simply open the file directly:

```bash
open docs/gallery/index.html
```

or drag `docs/gallery/index.html` into your browser.

### Option 3: Other Static Servers

```bash
# Using npx serve
cd docs/gallery
npx serve .

# Using PHP
cd docs/gallery
php -S localhost:8000

# Using Node.js http-server
cd docs/gallery
npx http-server -p 8000
```

## Testing the Gallery

### Visual Checklist

1. **Welcome Page**
   - Open the gallery
   - Verify the welcome page displays with 3 feature cards
   - Check that the Terraphim logo appears in the header

2. **Theme Toggle**
   - Click the theme toggle button in the top right
   - Verify the page switches between light and dark themes
   - Reload the page - theme should persist

3. **Navigation**
   - Click "Base Components" in the sidebar
   - Verify it expands to show 3 items
   - Click "TerraphimElement"
   - Verify the main content updates
   - Check that the browser URL shows `#/components/base/terraphim-element`

4. **Search**
   - Type "state" in the search box
   - Check browser console for search results
   - (Full search UI coming in Phase 2)

5. **Responsive Design**
   - Resize browser window to mobile size (< 768px)
   - Click the hamburger menu (☰)
   - Verify sidebar slides in from left
   - Click outside sidebar or press Escape
   - Verify sidebar closes

### Keyboard Navigation

1. Press `Tab` to navigate through interactive elements
2. Press `Enter` to activate links/buttons
3. Press `Escape` to close mobile menu
4. Verify focus indicators are visible

### Browser Testing

Test in these browsers:
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile Safari
- Chrome Mobile

## Expected Behavior

### Welcome Page (/)
- Shows welcome message
- Displays 3 feature cards
- Lists key features with checkmarks

### Component Pages (/components/*/*)
- Shows "Component Documentation" heading
- Displays current path
- Shows "Phase 1 Implementation" info box
- Lists coming features

### Navigation
- Clicking sidebar items updates main content
- Browser back/forward buttons work
- Active item is highlighted in sidebar
- Categories can expand/collapse

### Theme
- Toggle switches between light and dark
- Preference saved to localStorage
- Respects system preference on first visit

### Mobile
- Hamburger menu appears < 768px
- Sidebar slides in/out with animation
- Dark overlay appears when menu open
- Body scroll locked when menu open

## Troubleshooting

### Styles not loading
- Check that CSS files are in `docs/gallery/styles/`
- Verify file paths in `index.html` are correct
- Check browser console for 404 errors

### Components not rendering
- Verify JavaScript files are in `components/gallery/`
- Check browser console for JavaScript errors
- Ensure browser supports Custom Elements

### Navigation not working
- Check `data/nav-structure.json` exists
- Verify router.js is loaded
- Check browser console for errors

### Theme not persisting
- Check localStorage is enabled
- Verify theme-toggle.js is loaded
- Check browser console for errors

## File Structure Reference

```
docs/gallery/
├── index.html              # Entry point
├── styles/
│   ├── theme-light.css    # Light theme
│   ├── theme-dark.css     # Dark theme
│   ├── gallery.css        # Core styles
│   └── responsive.css     # Breakpoints
├── scripts/
│   ├── router.js          # Routing
│   └── search.js          # Search
└── data/
    ├── components.json    # Component data
    └── nav-structure.json # Navigation tree

components/gallery/
├── terraphim-gallery.js   # Root component
├── gallery-header.js      # Header
├── gallery-sidebar.js     # Sidebar
├── gallery-main.js        # Main content
├── nav-category.js        # Category
├── nav-item.js            # Nav item
└── theme-toggle.js        # Theme toggle
```

## Next Steps

After verifying Phase 1 works:

1. Review the code in each component
2. Check the design token system in `styles/theme-light.css`
3. Understand the router implementation in `scripts/router.js`
4. Plan Phase 2 enhancements (live examples, docs rendering)

## Getting Help

- Check `README.md` for full documentation
- Review `IMPLEMENTATION_SUMMARY.md` for implementation details
- Check browser console for error messages
- Verify all files exist with `./validate.sh`
