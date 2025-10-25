# Terraphim CSS Custom Properties Theme System

Complete CSS custom properties theme system with Bulmaswatch integration for Terraphim AI Web Components.

## Overview

This theme system provides:
- **CSS Custom Properties**: Semantic design tokens for colors, spacing, typography, shadows, and transitions
- **22 Bulmaswatch Themes**: All light and dark Bulmaswatch themes supported
- **Zero Dependencies**: Pure vanilla JavaScript, no frameworks or build tools
- **localStorage Persistence**: Theme preferences saved and restored automatically
- **System Preference Detection**: Respects user's OS color scheme preference
- **FOUC Prevention**: Instant theme application on page load
- **Smooth Transitions**: Animated theme switching with configurable timing

## Table of Contents

1. [CSS Custom Properties](#css-custom-properties)
2. [Theme Switcher Component](#theme-switcher-component)
3. [Usage Examples](#usage-examples)
4. [Event Handling](#event-handling)
5. [Migration Guide](#migration-guide)
6. [FOUC Prevention](#fouc-prevention)
7. [Available Themes](#available-themes)
8. [Accessibility](#accessibility)

---

## CSS Custom Properties

### Import Variables

```html
<!-- In your HTML <head> -->
<link rel="stylesheet" href="/components/styles/variables.css">
```

### Variable Categories

#### Colors

```css
/* Primary Colors */
--color-primary: #3273dc;
--color-primary-light: #4a86e8;
--color-primary-dark: #2366d1;
--color-primary-contrast: #ffffff;

/* Semantic Colors */
--color-success: #48c774;
--color-warning: #ffdd57;
--color-danger: #f14668;
--color-info: #3298dc;
```

#### Backgrounds

```css
--bg-page: #ffffff;          /* Main page background */
--bg-primary: #f5f5f5;       /* Primary container background */
--bg-secondary: #fafafa;     /* Secondary container background */
--bg-elevated: #ffffff;      /* Elevated surfaces (cards, modals) */
--bg-hover: rgba(0, 0, 0, 0.05);   /* Hover state background */
--bg-active: rgba(0, 0, 0, 0.08);  /* Active state background */
--bg-code: #f5f5f5;          /* Code block background */
--bg-overlay: rgba(0, 0, 0, 0.4);  /* Modal/overlay background */
```

#### Text Colors

```css
--text-primary: #363636;      /* Primary text color */
--text-secondary: #4a4a4a;    /* Secondary text color */
--text-tertiary: #7a7a7a;     /* Tertiary/muted text */
--text-link: #3273dc;         /* Link text color */
--text-link-hover: #2366d1;   /* Link hover color */
--text-disabled: #b5b5b5;     /* Disabled text color */
--text-inverse: #ffffff;      /* Inverse text (for dark backgrounds) */
```

#### Borders

```css
--border-primary: #dbdbdb;    /* Default border color */
--border-hover: #b5b5b5;      /* Hover border color */
--border-focus: #3273dc;      /* Focus border color */
--border-error: #f14668;      /* Error state border */
--border-success: #48c774;    /* Success state border */
```

#### Spacing Scale

```css
--spacing-xs: 0.25rem;   /* 4px */
--spacing-sm: 0.5rem;    /* 8px */
--spacing-md: 1rem;      /* 16px */
--spacing-lg: 1.5rem;    /* 24px */
--spacing-xl: 2rem;      /* 32px */
--spacing-2xl: 3rem;     /* 48px */
--spacing-3xl: 4rem;     /* 64px */
```

#### Border Radius

```css
--border-radius-sm: 2px;
--border-radius-md: 4px;
--border-radius-lg: 6px;
--border-radius-xl: 8px;
--border-radius-full: 9999px;
```

#### Shadows

```css
--shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
--shadow-md: 0 2px 4px rgba(0, 0, 0, 0.1);
--shadow-lg: 0 4px 8px rgba(0, 0, 0, 0.15);
--shadow-xl: 0 8px 16px rgba(0, 0, 0, 0.2);
--shadow-focus: 0 0 0 3px rgba(50, 115, 220, 0.25);
--shadow-inner: inset 0 2px 4px rgba(0, 0, 0, 0.06);
```

#### Typography

```css
/* Font Families */
--font-family-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
--font-family-mono: "SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, monospace;

/* Font Sizes */
--font-size-xs: 0.75rem;    /* 12px */
--font-size-sm: 0.875rem;   /* 14px */
--font-size-base: 1rem;     /* 16px */
--font-size-lg: 1.125rem;   /* 18px */
--font-size-xl: 1.25rem;    /* 20px */
--font-size-2xl: 1.5rem;    /* 24px */
--font-size-3xl: 1.875rem;  /* 30px */

/* Font Weights */
--font-weight-normal: 400;
--font-weight-medium: 500;
--font-weight-semibold: 600;
--font-weight-bold: 700;

/* Line Heights */
--line-height-tight: 1.25;
--line-height-normal: 1.5;
--line-height-relaxed: 1.75;
```

#### Transitions

```css
--transition-fast: 100ms ease;
--transition-base: 200ms ease;
--transition-slow: 300ms ease;
--transition-theme: background-color 200ms ease, color 200ms ease, border-color 200ms ease;
```

#### Z-Index Scale

```css
--z-dropdown: 1000;
--z-sticky: 1020;
--z-fixed: 1030;
--z-modal-backdrop: 1040;
--z-modal: 1050;
--z-popover: 1060;
--z-tooltip: 1070;
```

---

## Theme Switcher Component

### Basic Usage

```html
<!-- Import component -->
<script type="module" src="/components/shell/terraphim-theme-switcher.js"></script>

<!-- Use component -->
<terraphim-theme-switcher></terraphim-theme-switcher>
```

### Component Attributes

```html
<terraphim-theme-switcher
  current-theme="spacelab"
  show-label="true"
  storage-key="terraphim-theme">
</terraphim-theme-switcher>
```

#### Attributes

- **current-theme** (string, default: 'spacelab'): Initial theme to load
- **show-label** (boolean, default: true): Whether to show "Theme:" label
- **storage-key** (string, default: 'terraphim-theme'): localStorage key for persistence

### Component API

#### Properties

```javascript
const switcher = document.querySelector('terraphim-theme-switcher');

// Get/set current theme
console.log(switcher.currentTheme);  // "spacelab"
switcher.currentTheme = "darkly";

// Show/hide label
switcher.showLabel = false;

// Change storage key
switcher.storageKey = "my-custom-theme-key";
```

#### Methods

```javascript
// Switch to a theme
await switcher.switchTheme('darkly');

// Get current theme name
const currentTheme = switcher.getCurrentTheme();  // "darkly"

// Get all available themes
const allThemes = switcher.getThemes();
/*
[
  { name: 'spacelab', type: 'light', label: 'Spacelab', description: '...' },
  { name: 'darkly', type: 'dark', label: 'Darkly', description: '...' },
  ...
]
*/

// Detect system color scheme preference
const systemPref = switcher.detectSystemTheme();  // "dark" or "light"
```

---

## Usage Examples

### Using CSS Variables in Components

```css
/* In your component's Shadow DOM styles */
.my-component {
  background-color: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-primary);
  padding: var(--spacing-md);
  border-radius: var(--border-radius-md);
  box-shadow: var(--shadow-sm);
  transition: var(--transition-theme);
}

.my-component:hover {
  background-color: var(--bg-hover);
  border-color: var(--border-hover);
}

.my-component__title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--text-primary);
}

.my-component__subtitle {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
```

### Creating a Card Component

```javascript
class MyCard extends TerraphimElement {
  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        .card {
          background: var(--bg-elevated);
          border: 1px solid var(--border-primary);
          border-radius: var(--border-radius-lg);
          padding: var(--spacing-lg);
          box-shadow: var(--shadow-md);
          transition: var(--transition-theme);
        }

        .card:hover {
          box-shadow: var(--shadow-lg);
        }

        .card__title {
          color: var(--text-primary);
          font-size: var(--font-size-xl);
          margin-bottom: var(--spacing-sm);
        }

        .card__content {
          color: var(--text-secondary);
          line-height: var(--line-height-normal);
        }
      </style>

      <div class="card">
        <h3 class="card__title">${this.title}</h3>
        <p class="card__content">${this.content}</p>
      </div>
    `);
  }
}
```

### Responsive Theme Behavior

```css
/* Component adapts to theme changes automatically */
.button {
  background-color: var(--color-primary);
  color: var(--color-primary-contrast);
  border: none;
  padding: var(--spacing-sm) var(--spacing-lg);
  border-radius: var(--border-radius-md);
  font-weight: var(--font-weight-medium);
  transition: var(--transition-base);
}

.button:hover {
  background-color: var(--color-primary-dark);
  box-shadow: var(--shadow-md);
}

.button:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}
```

---

## Event Handling

### Theme Changed Event

```javascript
const switcher = document.querySelector('terraphim-theme-switcher');

switcher.addEventListener('theme-changed', (event) => {
  const { oldTheme, newTheme, isDark } = event.detail;

  console.log(`Theme changed from ${oldTheme} to ${newTheme}`);
  console.log(`New theme is ${isDark ? 'dark' : 'light'}`);

  // Update other components, analytics, etc.
  if (isDark) {
    console.log('Switched to dark mode');
  }
});
```

### Theme Loaded Event

```javascript
switcher.addEventListener('theme-loaded', (event) => {
  const { theme, loadTime } = event.detail;

  console.log(`Theme "${theme}" loaded in ${loadTime.toFixed(2)}ms`);
});
```

### Theme Error Event

```javascript
switcher.addEventListener('theme-error', (event) => {
  const { theme, error } = event.detail;

  console.error(`Failed to load theme "${theme}":`, error);

  // Fallback to default theme
  switcher.switchTheme('spacelab');
});
```

### Complete Example

```javascript
const switcher = document.querySelector('terraphim-theme-switcher');

// Listen to all theme events
switcher.addEventListener('theme-changed', (e) => {
  console.log('Theme changed:', e.detail);

  // Update analytics
  analytics.track('Theme Changed', {
    theme: e.detail.newTheme,
    isDark: e.detail.isDark
  });

  // Update other UI elements
  updateHeaderForTheme(e.detail.newTheme);
});

switcher.addEventListener('theme-loaded', (e) => {
  console.log('Theme loaded:', e.detail);

  // Hide loading indicator
  document.getElementById('theme-loader').style.display = 'none';
});

switcher.addEventListener('theme-error', (e) => {
  console.error('Theme error:', e.detail);

  // Show error message to user
  showNotification(`Failed to load theme: ${e.detail.theme}`, 'error');
});
```

---

## Migration Guide

### Migrating from Hardcoded Values

#### Before (hardcoded colors)

```css
.component {
  background: #ffffff;
  color: #363636;
  border: 1px solid #dbdbdb;
  padding: 16px;
  border-radius: 4px;
}
```

#### After (CSS variables)

```css
.component {
  background: var(--bg-elevated);
  color: var(--text-primary);
  border: 1px solid var(--border-primary);
  padding: var(--spacing-md);
  border-radius: var(--border-radius-md);
  transition: var(--transition-theme);
}
```

### Migrating from Svelte Stores

#### Before (Svelte theme store)

```svelte
<script>
  import { theme } from '$lib/stores/theme';
</script>

<style>
  .component {
    background: {$theme === 'dark' ? '#1a1a1a' : '#ffffff'};
    color: {$theme === 'dark' ? '#f5f5f5' : '#363636'};
  }
</style>
```

#### After (Web Component with CSS variables)

```javascript
class MyComponent extends TerraphimElement {
  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        .component {
          background: var(--bg-primary);
          color: var(--text-primary);
          transition: var(--transition-theme);
        }
      </style>
      <div class="component">Content</div>
    `);
  }
}
```

### Migrating Theme Switching Logic

#### Before (Svelte ThemeManager)

```typescript
import { applyTheme } from './themeManager';

function handleThemeChange(themeName: string) {
  applyTheme(themeName);
  localStorage.setItem('theme', themeName);
}
```

#### After (Web Component)

```html
<terraphim-theme-switcher></terraphim-theme-switcher>

<script>
  // Theme switching and persistence handled automatically
  // Listen to events if needed
  const switcher = document.querySelector('terraphim-theme-switcher');
  switcher.addEventListener('theme-changed', (e) => {
    console.log('New theme:', e.detail.newTheme);
  });
</script>
```

---

## FOUC Prevention

To prevent Flash of Unstyled Content (FOUC) when the page loads, add this to your main `index.html`:

### index.html Setup

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Terraphim AI</title>

  <!-- 1. Load CSS variables first -->
  <link rel="stylesheet" href="/components/styles/variables.css">

  <!-- 2. Apply saved theme immediately (before page render) -->
  <script>
    (function() {
      const theme = localStorage.getItem('terraphim-theme') || 'spacelab';
      document.documentElement.setAttribute('data-theme', theme);
    })();
  </script>

  <!-- 3. Preload Bulmaswatch CSS for saved theme -->
  <script>
    (function() {
      const theme = localStorage.getItem('terraphim-theme') || 'spacelab';
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = `/assets/bulmaswatch/${theme}/bulmaswatch.min.css`;
      document.head.appendChild(link);
    })();
  </script>
</head>
<body>
  <!-- Your app content -->
  <terraphim-theme-switcher></terraphim-theme-switcher>

  <script type="module" src="/components/shell/terraphim-theme-switcher.js"></script>
</body>
</html>
```

This ensures:
1. CSS variables are available immediately
2. Theme attribute is set before first paint
3. Bulmaswatch CSS loads as fast as possible
4. No theme "flash" on page load

---

## Available Themes

### Light Themes (16)

| Name | Label | Description |
|------|-------|-------------|
| `cerulean` | Cerulean | Calm blue theme |
| `cosmo` | Cosmo | Modern flat design |
| `default` | Default | Bulma default theme |
| `flatly` | Flatly | Flat and modern |
| `journal` | Journal | Newspaper style |
| `litera` | Litera | Clean serif theme |
| `lumen` | Lumen | Light and airy |
| `lux` | Lux | Luxurious gold accents |
| `materia` | Materia | Material design inspired |
| `minty` | Minty | Fresh green theme |
| `pulse` | Pulse | Vibrant purple accents |
| `sandstone` | Sandstone | Warm earthy tones |
| `simplex` | Simplex | Simple and clean |
| `spacelab` | Spacelab | Default theme |
| `united` | United | Bold and unified |
| `yeti` | Yeti | Cool blue-gray theme |

### Dark Themes (6)

| Name | Label | Description |
|------|-------|-------------|
| `cyborg` | Cyborg | Dark blue tech theme |
| `darkly` | Darkly | Popular dark theme |
| `nuclear` | Nuclear | Dark green accents |
| `slate` | Slate | Dark purple theme |
| `solar` | Solar | Solarized dark |
| `superhero` | Superhero | Dark with red accents |

---

## Accessibility

### Keyboard Navigation

The theme switcher is fully keyboard accessible:
- **Tab**: Focus the dropdown
- **Arrow keys**: Navigate theme options
- **Enter/Space**: Select theme
- **Escape**: Close dropdown

### Screen Readers

The component includes proper ARIA labels:

```html
<select class="theme-select" aria-label="Select theme">
  <optgroup label="Light Themes">
    <option value="spacelab" title="Default theme">Spacelab</option>
    ...
  </optgroup>
  <optgroup label="Dark Themes">
    <option value="darkly" title="Popular dark theme">Darkly</option>
    ...
  </optgroup>
</select>
```

### Reduced Motion

The system respects `prefers-reduced-motion`:

```css
@media (prefers-reduced-motion: reduce) {
  :root {
    --transition-fast: 0ms;
    --transition-base: 0ms;
    --transition-slow: 0ms;
    --transition-theme: none;
  }
}
```

### Focus Indicators

All focusable elements have clear focus indicators:

```css
:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}
```

---

## Best Practices

### 1. Always Use Variables

```css
/* Good */
.component {
  color: var(--text-primary);
  padding: var(--spacing-md);
}

/* Bad */
.component {
  color: #363636;
  padding: 16px;
}
```

### 2. Include Transition for Theme Changes

```css
/* Good - smooth theme transitions */
.component {
  background: var(--bg-primary);
  transition: var(--transition-theme);
}

/* Also good - custom transition */
.component {
  background: var(--bg-primary);
  transition: background-color 300ms ease;
}
```

### 3. Use Semantic Variables

```css
/* Good - semantic meaning */
.button--primary {
  background: var(--color-primary);
  color: var(--color-primary-contrast);
}

/* Bad - specific color values */
.button--primary {
  background: #3273dc;
  color: white;
}
```

### 4. Respect Shadow DOM Boundaries

CSS variables automatically cascade into Shadow DOM:

```javascript
// CSS variables work in Shadow DOM
class MyComponent extends TerraphimElement {
  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        /* These variables are inherited from :root */
        .title { color: var(--text-primary); }
      </style>
    `);
  }
}
```

---

## Troubleshooting

### Theme Not Loading

**Problem**: Theme doesn't apply on page load

**Solution**: Ensure FOUC prevention script runs before rendering:

```html
<head>
  <link rel="stylesheet" href="/components/styles/variables.css">
  <script>
    (function() {
      const theme = localStorage.getItem('terraphim-theme') || 'spacelab';
      document.documentElement.setAttribute('data-theme', theme);
    })();
  </script>
</head>
```

### CSS Variables Not Working

**Problem**: Variables showing as literal text

**Solution**: Ensure `variables.css` is loaded:

```html
<link rel="stylesheet" href="/components/styles/variables.css">
```

### localStorage Not Persisting

**Problem**: Theme not saved after refresh

**Solution**: Check browser localStorage permissions and ensure component has correct storage-key:

```html
<terraphim-theme-switcher storage-key="terraphim-theme"></terraphim-theme-switcher>
```

---

## Performance

- **Theme Switch Time**: < 200ms (includes CSS load and DOM update)
- **localStorage Operations**: Debounced to prevent excessive writes
- **CSS Variables**: Near-zero performance impact (native browser feature)
- **Shadow DOM**: Encapsulated styles prevent global CSS conflicts

---

## Browser Support

- Chrome/Edge: 88+
- Firefox: 85+
- Safari: 14+
- All browsers supporting Web Components and CSS Custom Properties

---

## License

MIT License - See project LICENSE file for details.
