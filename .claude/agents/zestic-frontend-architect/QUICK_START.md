# Quick Start: CSS Custom Properties Theme System

**For**: @zestic-front-craftsman and implementation team
**Blueprint**: Phase 2.4 CSS Custom Properties Theme System
**Time Required**: 5 weeks (phased implementation)

## TL;DR

Replace Svelte theme system with pure CSS custom properties:
- 200+ CSS variables for complete theming
- Zero FOUC via inline script
- Web Component theme switcher
- localStorage persistence
- Full Bulmaswatch compatibility

## Before You Start

### Prerequisites

- [x] Read full blueprint: `phase-2.4-theme-system-blueprint.md`
- [x] Understand TerraphimElement and TerraphimState APIs
- [x] Review existing ThemeSwitcher.svelte implementation
- [x] Test environment set up for Web Components

### Key Files to Review

1. `/desktop/src/lib/ThemeSwitcher.svelte` - Current implementation
2. `/components/base/terraphim-element.js` - Base component class
3. `/components/base/terraphim-state.js` - State management
4. `/desktop/public/assets/bulmaswatch/*` - Existing theme CSS files

## Week 1: Foundation

### Goal
Create CSS variables system and FOUC prevention

### Tasks

#### 1. Create Directory Structure
```bash
mkdir -p components/styles/themes
```

#### 2. Create variables.css
File: `/components/styles/variables.css`

Copy the complete CSS variables from blueprint section "Complete CSS Variables Structure" (~500 lines)

**Critical Variables**:
```css
:root {
  --color-primary: #446e9b;
  --text-primary: #333333;
  --bg-page: #ffffff;
  --transition-colors: color 250ms, background-color 250ms, border-color 250ms;
  /* ... 200+ more ... */
}
```

#### 3. Create Theme Files
Extract colors from `/desktop/public/assets/bulmaswatch/spacelab/bulmaswatch.min.css`

File: `/components/styles/themes/spacelab.css`
```css
:root[data-theme="spacelab"] {
  --brand-primary: #446e9b;
  --brand-secondary: #807f7f;
  /* ... theme overrides ... */
}
```

Repeat for `light.css` and `dark.css`

#### 4. Create FOUC Prevention Script
File: `/components/styles/theme-loader.js`

Copy complete script from blueprint section "FOUC Prevention Strategy"

**Key Functions**:
- `getSavedTheme()` - Read from localStorage
- `applyTheme()` - Set data attributes
- `loadThemeCSS()` - Load theme stylesheet
- `window.__TERRAPHIM_THEME__` - Global API

#### 5. Update HTML
File: `/desktop/index.html` or equivalent

```html
<head>
  <!-- 1. CSS Variables FIRST -->
  <link rel="stylesheet" href="/components/styles/variables.css">

  <!-- 2. INLINE Theme Loader (critical!) -->
  <script>
    /* Paste entire theme-loader.js here */
  </script>

  <!-- 3. Rest of CSS -->
  <!-- ... -->
</head>
```

### Validation

- [ ] No FOUC on page load
- [ ] Console shows: `window.__TERRAPHIM_THEME__.get()` returns current theme
- [ ] DevTools Elements panel shows `:root` variables
- [ ] Manual theme switch works: `window.__TERRAPHIM_THEME__.set('dark')`
- [ ] Theme persists after page reload

## Week 2: Theme Switcher Component

### Goal
Build Web Component for theme selection UI

### Tasks

#### 1. Create Component File
File: `/components/shell/terraphim-theme-switcher.js`

Copy complete implementation from blueprint section "Component Implementation"

**Key Features**:
- Dropdown, list, and buttons variants
- Keyboard navigation (Arrow keys, Enter, Escape)
- ARIA accessibility
- Theme preview colors
- Event emission

#### 2. Create Test File
File: `/components/shell/terraphim-theme-switcher.test.js`

Copy tests from blueprint section "Component Tests"

Run tests:
```bash
npm test -- terraphim-theme-switcher.test.js
```

#### 3. Register Component
In your main application entry point:
```javascript
import './components/shell/terraphim-theme-switcher.js';
```

#### 4. Add to UI
Replace existing ThemeSwitcher.svelte:
```html
<!-- Before -->
<ThemeSwitcher />

<!-- After -->
<terraphim-theme-switcher></terraphim-theme-switcher>
```

### Validation

- [ ] Component renders without errors
- [ ] Dropdown opens/closes
- [ ] Theme changes on selection
- [ ] Keyboard navigation works
- [ ] Tests pass: `npm test`
- [ ] Accessibility audit passes: Lighthouse 100

## Week 3: Bulma Integration

### Goal
Make existing Bulma components respect theme variables

### Tasks

#### 1. Create Bulma Overrides
File: `/components/styles/bulma-overrides.css`

```css
/* Override Bulma classes with CSS variables */
.button {
  background-color: var(--button-bg);
  color: var(--button-text);
  border-color: var(--button-border);
}

.input, .textarea, .select select {
  background-color: var(--input-bg);
  color: var(--input-text);
  border-color: var(--input-border);
}

/* ... all Bulma components ... */
```

#### 2. Test All Components
Check each Bulma component:
- [ ] Buttons (all variants)
- [ ] Forms (inputs, selects, textareas)
- [ ] Cards
- [ ] Navigation
- [ ] Modals
- [ ] Tables
- [ ] Notifications

#### 3. Update Existing Components
Migrate components to use CSS variables:

```javascript
// Before
const styles = `
  .element {
    background: #ffffff;
    color: #333333;
  }
`;

// After
const styles = `
  .element {
    background: var(--bg-surface);
    color: var(--text-primary);
    transition: var(--transition-colors);
  }
`;
```

### Validation

- [ ] No visual regressions
- [ ] All themes work with Bulma
- [ ] Smooth transitions on theme switch
- [ ] Dark mode has proper contrast (use WebAIM contrast checker)

## Week 4: Additional Themes

### Goal
Add remaining Bulmaswatch themes

### Tasks

#### 1. Extract Theme Colors
For each theme in `/desktop/public/assets/bulmaswatch/`:

1. Open theme CSS file
2. Find primary colors (usually in first 100 lines)
3. Create theme file with overrides

#### 2. Create Theme Files
Files: `/components/styles/themes/{themeName}.css`

Themes to create:
- cyborg.css
- darkly.css
- flatly.css
- superhero.css
- slate.css
- solar.css
- cerulean.css
- cosmo.css
- journal.css
- litera.css
- lumen.css
- lux.css
- materia.css
- minty.css
- nuclear.css
- pulse.css
- sandstone.css
- simplex.css
- united.css
- yeti.css

#### 3. Update Theme Switcher
Add all themes to default list:
```javascript
static get properties() {
  return {
    themes: {
      type: Array,
      default: () => [
        'spacelab', 'light', 'dark', 'cyborg', 'darkly',
        'flatly', 'superhero', 'slate', 'solar', 'nuclear',
        // ... all themes ...
      ]
    },
    // ...
  };
}
```

#### 4. Create Preview Gallery
Demo page showing all themes (optional but recommended)

### Validation

- [ ] All 22+ themes available in switcher
- [ ] Each theme renders correctly
- [ ] Theme preview colors accurate
- [ ] No console errors for any theme

## Week 5: Migration & Polish

### Goal
Replace old system, optimize, and finalize

### Tasks

#### 1. Remove Old Code
- [ ] Delete or deprecate `ThemeSwitcher.svelte`
- [ ] Remove theme link manipulation logic
- [ ] Clean up Svelte theme store (if not used elsewhere)
- [ ] Update imports in `App.svelte`

#### 2. State Integration
Ensure TerraphimState tracks theme:
```javascript
import { createGlobalState } from './components/base/terraphim-state.js';

const state = createGlobalState({
  theme: window.__TERRAPHIM_THEME__?.get() || 'spacelab',
  // ... other state ...
}, {
  persist: true,
  storagePrefix: 'terraphim'
});

// Sync theme changes
window.addEventListener('theme-changed', (e) => {
  state.set('theme', e.detail.theme);
});
```

#### 3. Performance Optimization
- [ ] Minify CSS files
- [ ] Test theme switch latency (target: <16ms)
- [ ] Verify no memory leaks (DevTools Memory profiler)
- [ ] Check bundle size increase (<20KB)

#### 4. Accessibility Audit
Run Lighthouse and fix issues:
- [ ] Contrast ratios meet WCAG AA (4.5:1 text, 3:1 UI)
- [ ] Keyboard navigation complete
- [ ] Screen reader announcements work
- [ ] Focus indicators visible

#### 5. Browser Testing
Test in:
- [ ] Chrome (latest)
- [ ] Firefox (latest)
- [ ] Safari (latest)
- [ ] Edge (latest)
- [ ] Mobile browsers (iOS Safari, Chrome Android)

#### 6. User Acceptance Testing
- [ ] Theme switching works smoothly
- [ ] Preferences persist
- [ ] No FOUC
- [ ] All components themed correctly

### Validation

- [ ] All old code removed
- [ ] Performance targets met
- [ ] Lighthouse score: 100
- [ ] Zero regressions
- [ ] UAT approved

## Common Pitfalls

### 1. FOUC Still Occurring
**Problem**: External script loading
**Solution**: Ensure theme-loader.js is **inlined** in HTML head, not `<script src="...">`

### 2. Variables Not Working in Shadow DOM
**Problem**: CSS variables don't inherit into shadow roots automatically
**Solution**: Import variables.css or use `:host { color: var(--text-primary); }`

### 3. Theme Not Persisting
**Problem**: localStorage blocked or STORAGE_KEY mismatch
**Solution**: Check browser privacy settings, verify key is consistent

### 4. Poor Dark Mode Contrast
**Problem**: Colors too similar
**Solution**: Use contrast checker, adjust variables in dark theme file

### 5. Laggy Theme Switch
**Problem**: Too many transitions, layout thrashing
**Solution**: Use `transition: var(--transition-colors)`, batch DOM updates

## Debug Commands

```javascript
// Get current theme
window.__TERRAPHIM_THEME__.get()

// Set theme
window.__TERRAPHIM_THEME__.set('dark')

// Inspect a variable
getComputedStyle(document.documentElement).getPropertyValue('--color-primary')

// List available themes
document.querySelector('terraphim-theme-switcher').themes

// Force re-render
document.querySelector('terraphim-theme-switcher').requestUpdate()
```

## Success Criteria

### Must Have
- ✅ Zero FOUC on any page load
- ✅ Theme persists across sessions
- ✅ Keyboard navigation works
- ✅ WCAG 2.1 AA compliant
- ✅ Works in all major browsers

### Should Have
- ✅ <16ms theme switch (60fps)
- ✅ <50ms to first themed paint
- ✅ All 22+ themes available
- ✅ Preview colors accurate
- ✅ Smooth transitions

### Nice to Have
- ✅ Auto-detect system preference
- ✅ Theme preview on hover
- ✅ Custom color overrides
- ✅ Theme gallery demo page

## Getting Help

1. **Check Blueprint**: Full details in `phase-2.4-theme-system-blueprint.md`
2. **Review Examples**: All code examples in blueprint
3. **Test Locally**: Use debug commands above
4. **Ask Questions**: Unclear requirements → update blueprint

## Quick Reference Links

- **Full Blueprint**: `phase-2.4-theme-system-blueprint.md`
- **TerraphimElement API**: `/components/base/terraphim-element.js`
- **TerraphimState API**: `/components/base/terraphim-state.js`
- **Current Implementation**: `/desktop/src/lib/ThemeSwitcher.svelte`

---

**Last Updated**: 2025-10-25
**Version**: 1.0
**Status**: Ready for Implementation
