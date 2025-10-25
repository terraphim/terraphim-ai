# Terraphim App Shell & Routing System

Complete vanilla JavaScript application shell with dual-mode routing (History API / Hash mode) built using Web Components.

## Overview

This implementation provides a production-ready application shell with:

- **Pure vanilla JavaScript** - No frameworks, no build tools
- **Web Components** - Encapsulated, reusable components
- **Dual-mode routing** - Automatic detection for web/Tauri
- **Navigation system** - Tab-based navigation with keyboard shortcuts
- **Lazy loading** - Components loaded on demand
- **Navigation guards** - Route-level access control
- **Mobile responsive** - Optimized for mobile and desktop

## Architecture

```
┌─────────────────────────────────────────┐
│         TerraphimApp (Shell)            │
│  ┌───────────────────────────────────┐  │
│  │        Header                      │  │
│  │  ┌──────────┐  ┌──────────────┐   │  │
│  │  │   Logo   │  │ TerraphimNav │   │  │
│  │  └──────────┘  └──────────────┘   │  │
│  └───────────────────────────────────┘  │
│                                          │
│  ┌───────────────────────────────────┐  │
│  │     Main Content Area             │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │    RouterOutlet             │  │  │
│  │  │  (Dynamic Components)       │  │  │
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
│                                          │
│  ┌───────────────────────────────────┐  │
│  │        Footer                      │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Components

### 1. TerraphimRouter (`terraphim-router.js`) - ~8KB

Core routing engine handling all navigation logic.

**Features:**
- Dual-mode routing (history/hash)
- Route pattern matching (`:param`, `:param?`)
- Query string parsing
- Navigation guards (beforeEach, afterEach, beforeEnter, afterEnter)
- Lazy loading support
- Browser back/forward support
- Programmatic navigation API

**API:**

```javascript
// Create router
const router = new TerraphimRouter({
  mode: 'history', // 'history', 'hash', or 'auto'
  routes: [...],
  beforeEach: (to, from) => { /* guard logic */ },
  afterEach: (to, from) => { /* post-navigation */ }
});

// Initialize
router.init();

// Navigate
router.navigate('/path');
router.push('route-name', { id: 123 }, { query: 'param' });
router.replace('route-name');
router.back();
router.forward();

// Check active
router.isActive('/path', true); // exact match
router.isActive('/path'); // prefix match

// Build paths
router.buildPath('user', { id: 123 }, { tab: 'profile' });
// Returns: /user/123?tab=profile
```

**Events:**
- `route-change-start` - Before route changes
- `route-change` - After route changes
- `route-change-error` - On error loading component

### 2. RouterOutlet (`router-outlet.js`) - ~3KB

Dynamic component container that renders route components.

**Features:**
- Dynamic component loading
- Loading states with spinner
- Error states with retry
- Component cleanup
- Route params/query passing
- Optional keep-alive caching

**Usage:**

```html
<router-outlet></router-outlet>
<router-outlet keep-alive></router-outlet>
```

**Component Props:**
- `routeParams` - Route parameters object
- `queryParams` - Query parameters object
- `routeMeta` - Route metadata

### 3. TerraphimNav (`terraphim-nav.js`) - ~4KB

Navigation component with active link highlighting.

**Features:**
- Horizontal tab navigation
- Active link detection
- Keyboard shortcuts (Ctrl+1-9)
- Mobile responsive (icon-only)
- Badge notifications
- ARIA accessibility

**Auto-generated from routes:**
```javascript
// Routes with meta.title appear in navigation
{
  name: 'search',
  path: '/',
  meta: {
    title: 'Search',
    icon: 'fas fa-search',
    badge: 'New'
  }
}
```

**Keyboard Shortcuts:**
- `Ctrl+1` - First nav item
- `Ctrl+2` - Second nav item
- `Ctrl+3` - Third nav item
- etc.

### 4. TerraphimApp (`terraphim-app.js`) - ~6KB

Main application shell container.

**Features:**
- Application layout (header/main/footer)
- Router initialization
- Route change handling
- Mobile menu management
- Loading states
- Page title updates

**Usage:**

```html
<terraphim-app router-mode="auto"></terraphim-app>
```

```javascript
// Set routes programmatically
const app = document.querySelector('terraphim-app');
app.setRoutes([...]);

// Navigate
app.navigate('/path');

// Get current route
const route = app.getCurrentRoute();
```

**Events:**
- `app-ready` - Application initialized
- `route-changed` - Route changed

## Route Configuration

### Route Definition

```javascript
{
  name: 'route-name',           // Unique route name
  path: '/path/:param?',         // URL pattern
  component: 'element-name',     // Web Component tag
  lazy: () => import('./comp.js'), // Lazy loader
  meta: {                        // Metadata
    title: 'Page Title',
    icon: 'fas fa-icon',
    exact: true,                 // Exact path matching
    requiresAuth: true           // Custom meta
  },
  beforeEnter: (to, from) => { /* guard */ },
  afterEnter: (to, from) => { /* hook */ }
}
```

### Path Patterns

- `/path` - Exact path
- `/path/:id` - Required parameter
- `/path/:id?` - Optional parameter
- `*` - Wildcard (404 catch-all)

### Example Routes

```javascript
const routes = [
  {
    name: 'search',
    path: '/',
    component: 'terraphim-search',
    lazy: () => import('./search/terraphim-search.js'),
    meta: { title: 'Search', icon: 'fas fa-search', exact: true }
  },
  {
    name: 'chat',
    path: '/chat/:id?',
    component: 'terraphim-chat',
    lazy: () => import('./features/terraphim-chat.js'),
    meta: { title: 'Chat', icon: 'fas fa-comments' }
  },
  {
    name: '404',
    path: '*',
    component: 'terraphim-not-found',
    lazy: () => import('./features/terraphim-not-found.js'),
    meta: { title: 'Page Not Found' }
  }
];
```

## Navigation Guards

### Global Guards

```javascript
const router = new TerraphimRouter({
  routes,
  beforeEach: async (to, from) => {
    // Check authentication
    if (to.meta.requiresAuth && !isAuthenticated()) {
      router.navigate('/login');
      return false; // Cancel navigation
    }
    return true; // Allow navigation
  },
  afterEach: async (to, from) => {
    // Track analytics
    trackPageView(to.path);
  }
});
```

### Route-specific Guards

```javascript
{
  name: 'admin',
  path: '/admin',
  component: 'admin-panel',
  beforeEnter: async (to, from) => {
    if (!isAdmin()) {
      return false;
    }
  },
  afterEnter: async (to, from) => {
    console.log('Entered admin panel');
  }
}
```

## Routing Modes

### History API Mode (Web)

Uses `pushState` and `popstate` for clean URLs:
- URL: `https://example.com/chat/123`
- Requires server-side routing configuration
- Best for web deployments

### Hash Mode (Tauri)

Uses URL hash for client-side routing:
- URL: `https://example.com/#/chat/123`
- Works without server configuration
- Best for Tauri desktop apps

### Auto-detection

```javascript
// Automatically detects Tauri environment
const router = new TerraphimRouter({
  mode: 'auto' // Uses hash for Tauri, history for web
});
```

## Integration Guide

### Basic Setup

1. **Import components:**

```html
<script type="module">
  import './components/shell/terraphim-app.js';
</script>
```

2. **Define routes:**

```javascript
const routes = [
  {
    name: 'home',
    path: '/',
    component: 'home-page',
    lazy: () => import('./pages/home-page.js'),
    meta: { title: 'Home', icon: 'fas fa-home' }
  }
];
```

3. **Initialize app:**

```html
<terraphim-app id="app"></terraphim-app>

<script type="module">
  const app = document.getElementById('app');
  app.setRoutes(routes);
</script>
```

### Creating Route Components

Route components receive route data automatically:

```javascript
class MyPage extends TerraphimElement {
  static get observedAttributes() {
    return ['route-params', 'query-params'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'route-params') {
      const params = JSON.parse(newValue);
      console.log('Route params:', params);
    }
  }
}
```

### Programmatic Navigation

```javascript
// Get router instance
const router = window.terraphimRouter;

// Navigate by path
router.navigate('/chat/123?tab=messages');

// Navigate by name
router.push('chat', { id: 123 }, { tab: 'messages' });

// Replace current entry
router.replace('home');

// Browser history
router.back();
router.forward();
```

### Link Navigation

```html
<!-- Links are automatically intercepted -->
<a href="/chat">Go to Chat</a>

<!-- Hash mode -->
<a href="#/chat">Go to Chat</a>

<!-- External links work normally -->
<a href="https://example.com">External</a>
```

## Demo

Open `shell-integration-demo.html` in a browser to test:

1. **Route navigation** - Click nav tabs or use dropdown
2. **Keyboard shortcuts** - Ctrl+1 through Ctrl+5
3. **Browser buttons** - Back/forward navigation
4. **URL parameters** - Test `/chat/123`
5. **404 handling** - Navigate to invalid path
6. **Mode switching** - Toggle history/hash modes
7. **Mobile menu** - Resize window to test mobile view

## Testing

### Manual Testing

```bash
# Serve files locally
python3 -m http.server 8000

# Open in browser
open http://localhost:8000/components/shell/shell-integration-demo.html
```

### Test Cases

- ✅ All routes render correctly
- ✅ Navigation between routes works
- ✅ Browser back/forward buttons work
- ✅ Active link highlighting updates
- ✅ Keyboard shortcuts (Ctrl+1-5) work
- ✅ Hash mode works for Tauri
- ✅ History mode works for web
- ✅ Lazy loading shows loading state
- ✅ 404 page displays for invalid routes
- ✅ Mobile menu toggles on small screens
- ✅ Route params are passed to components
- ✅ Query params are passed to components

## File Structure

```
components/shell/
├── terraphim-router.js          # Core router (~8KB)
├── terraphim-app.js             # App shell (~6KB)
├── terraphim-nav.js             # Navigation (~4KB)
├── router-outlet.js             # Route outlet (~3KB)
├── shell-integration-demo.html  # Integration demo
└── README.md                    # This file

components/features/
├── terraphim-chat.js            # Chat stub
├── terraphim-graph.js           # Graph stub
├── terraphim-config-wizard.js   # Wizard stub
├── terraphim-config-json.js     # JSON editor stub
└── terraphim-not-found.js       # 404 page
```

## Browser Support

- Chrome/Edge 88+
- Firefox 85+
- Safari 14+
- Mobile browsers (iOS Safari, Chrome Mobile)

**Required APIs:**
- Custom Elements v1
- Shadow DOM v1
- ES6 Modules
- History API (for history mode)
- URL hash (for hash mode)

## Performance

- **Total bundle size**: ~21KB (uncompressed)
- **Gzipped**: ~6KB
- **Initial load**: Instant (no build step)
- **Route change**: <50ms (lazy loaded)
- **Memory**: Minimal (efficient cleanup)

## Migration from Svelte

### Before (Svelte + Tinro)

```svelte
<script>
  import { Route } from 'tinro';
  import SearchPage from './SearchPage.svelte';
</script>

<Route path="/">
  <SearchPage />
</Route>
```

### After (Vanilla Components)

```javascript
const routes = [
  {
    name: 'search',
    path: '/',
    component: 'terraphim-search',
    lazy: () => import('./components/search/terraphim-search.js'),
    meta: { title: 'Search', icon: 'fas fa-search' }
  }
];
```

## Troubleshooting

### Routes not loading

- Check console for import errors
- Verify component files exist
- Ensure components are defined before router init

### Active links not highlighting

- Verify route paths match exactly
- Check `exact` meta option for home route
- Ensure router is initialized

### Hash mode not working in Tauri

- Verify `router-mode="hash"` or `"auto"`
- Check for conflicting history API calls

### Back button not working

- Ensure router is initialized with `router.init()`
- Check for event listener conflicts

## Future Enhancements

- [ ] Route transitions/animations
- [ ] Nested routes
- [ ] Route redirects
- [ ] Query param reactivity
- [ ] Route-based code splitting
- [ ] Preloading strategies
- [ ] Navigation progress bar

## License

Part of the Terraphim AI project - see main LICENSE file.

## Support

For issues and questions:
- GitHub Issues: https://github.com/terraphim/terraphim-ai/issues
- Documentation: See main project README
