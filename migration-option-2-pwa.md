# Migration Option 2: Web-based PWA Migration Plan

## Overview

This migration plan transforms the desktop application into a Progressive Web Application (PWA) that can run in any modern browser while maintaining desktop-like capabilities through web APIs. The backend services are migrated to a web server architecture with REST APIs.

## Migration Strategy

### Phase 1: Backend Web Service Migration (3-4 weeks)

#### 1.1 Create Web Server Backend
**Timeline**: 2 weeks
**Effort**: High

**Tasks**:
- Extract Tauri commands into HTTP API endpoints
- Implement RESTful API design with OpenAPI specification
- Create authentication and authorization layer
- Set up CORS for cross-origin requests
- Implement WebSocket support for real-time features

**Architecture**:
```
web-backend/
├── src/
│   ├── main.rs              # Web server entry point
│   ├── handlers/            # API endpoint handlers
│   │   ├── search.rs       # Search endpoints
│   │   ├── chat.rs         # Chat endpoints
│   │   ├── config.rs       # Configuration endpoints
│   │   └── kg.rs           # Knowledge graph endpoints
│   ├── middleware/         # HTTP middleware
│   │   ├── auth.rs         # Authentication
│   │   ├── cors.rs         # CORS handling
│   │   └── logging.rs      # Request logging
│   ├── models/             # Data models
│   └── utils/              # Utility functions
├── Cargo.toml              # Dependencies
└── openapi.yaml            # API specification
```

**Implementation Example**:
```rust
// web-backend/src/handlers/search.rs
use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Serialize)]
struct SearchResult {
    id: String,
    title: String,
    content: String,
    score: f64,
}

pub async fn search(
    Query(query): Query<SearchQuery>,
    app_state: State<AppState>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let results = app_state
        .search_service
        .search(&query.q, query.limit.unwrap_or(10))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}
```

#### 1.2 Authentication & Security
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Implement JWT-based authentication
- Create session management
- Add rate limiting
- Implement API key authentication for external access
- Add security headers and CSP

**Security Features**:
```rust
// JWT Authentication middleware
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

pub fn create_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("your-secret-key".as_ref()),
    )
}
```

#### 1.3 Database & Persistence Layer
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Migrate persistence to web-compatible backends
- Implement database connection pooling
- Create data access layer
- Add data synchronization capabilities
- Implement backup and restore

### Phase 2: Frontend PWA Conversion (4-5 weeks)

#### 2.1 PWA Foundation Setup
**Timeline**: 2 weeks
**Effort**: High

**Tasks**:
- Set up SvelteKit with PWA plugin
- Create service worker for offline functionality
- Implement web app manifest
- Add push notification support
- Create responsive design for mobile/tablet/desktop

**PWA Configuration**:
```typescript
// svelte.config.js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/kit/vite';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: null,
      precompress: false,
      strict: true
    }),
    serviceWorker: {
      register: true,
      inline: true,
    }
  }
};

export default config;
```

**Web App Manifest**:
```json
{
  "name": "Terraphim AI",
  "short_name": "Terraphim",
  "description": "Privacy-first AI assistant",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#3273dc",
  "icons": [
    {
      "src": "/icons/icon-192x192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512x512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

#### 2.2 Component Migration & Enhancement
**Timeline**: 2-3 weeks
**Effort**: High

**Tasks**:
- Migrate existing Svelte components to web standards
- Remove Tauri-specific APIs and replace with web alternatives
- Implement file system access via File System Access API
- Add camera/microphone access where needed
- Create web-compatible notification system

**File System Access Example**:
```typescript
// Replace Tauri file operations with Web File System Access API
import { open, save } from '@tauri-apps/api/dialog';

// Web alternative
async function openFile(): Promise<File | null> {
  try {
    const [fileHandle] = await window.showOpenFilePicker({
      types: [{
        description: 'Text files',
        accept: { 'text/plain': ['.txt', '.md'] }
      }]
    });
    return await fileHandle.getFile();
  } catch (err) {
    console.error('File open cancelled or failed:', err);
    return null;
  }
}

async function saveFile(content: string, filename: string): Promise<void> {
  try {
    const fileHandle = await window.showSaveFilePicker({
      suggestedName: filename,
      types: [{
        description: 'Text files',
        accept: { 'text/plain': ['.txt', '.md'] }
      }]
    });
    const writable = await fileHandle.createWritable();
    await writable.write(content);
    await writable.close();
  } catch (err) {
    console.error('File save cancelled or failed:', err);
  }
}
```

#### 2.3 Offline Capabilities
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Implement service worker for caching
- Create offline-first data synchronization
- Add background sync for queued operations
- Implement cache management strategies
- Create offline indicators and controls

**Service Worker Implementation**:
```typescript
// src/service-worker.ts
import { build, files, version } from '$service-worker';

const CACHE_NAME = `terraphim-cache-${version}`;
const STATIC_ASSETS = 'static-assets';
const API_CACHE = 'api-cache';

// Install event - cache static assets
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(STATIC_ASSETS).then((cache) => {
      return cache.addAll(build.concat(files));
    })
  );
});

// Fetch event - implement caching strategies
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // API requests - network first, cache fallback
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(networkFirst(request));
    return;
  }

  // Static assets - cache first
  event.respondWith(cacheFirst(request));
});

async function networkFirst(request: Request): Promise<Response> {
  try {
    const response = await fetch(request);

    // Cache successful responses
    if (response.ok) {
      const cache = await caches.open(API_CACHE);
      cache.put(request, response.clone());
    }

    return response;
  } catch (error) {
    // Fallback to cache
    const cachedResponse = await caches.match(request);
    if (cachedResponse) {
      return cachedResponse;
    }

    // Return offline page for navigation requests
    if (request.mode === 'navigate') {
      return caches.match('/offline') || new Response('Offline');
    }

    throw error;
  }
}

async function cacheFirst(request: Request): Promise<Response> {
  const cachedResponse = await caches.match(request);
  if (cachedResponse) {
    return cachedResponse;
  }

  return fetch(request);
}
```

### Phase 3: Advanced Web Features (3-4 weeks)

#### 3.1 Web-Based System Tray Simulation
**Timeline**: 1-2 weeks
**Effort**: High

**Tasks**:
- Implement PWA install prompt for desktop-like experience
- Create floating action button for role switching
- Implement browser action/badge for quick access
- Add role switching via browser extension (optional)
- Create persistent role state across sessions

**PWA System Tray Alternative**:
```typescript
// src/lib/pwa-system-tray.ts
class PWASystemTrayManager {
    private isInstalled = false;
    private currentRole = 'default';
    private floatingButton: HTMLElement | null = null;

    async initialize() {
        // Check if PWA is installed
        this.isInstalled = this.checkPWAInstallation();

        // Create floating action button for role switching
        this.createFloatingRoleSwitcher();

        // Set up browser action if supported
        this.setupBrowserAction();

        // Listen for role changes
        this.setupRoleChangeListeners();
    }

    private checkPWAInstallation(): boolean {
        return window.matchMedia('(display-mode: standalone)').matches ||
               (window.navigator as any).standalone === true;
    }

    private createFloatingRoleSwitcher() {
        // Create floating action button
        this.floatingButton = document.createElement('div');
        this.floatingButton.className = 'pwa-floating-tray';
        this.floatingButton.innerHTML = `
            <div class="tray-button">
                <i class="fas fa-user-circle"></i>
                <span class="role-indicator">${this.currentRole}</span>
            </div>
            <div class="tray-menu" style="display: none;">
                <div class="tray-header">Switch Role</div>
                <div class="role-list" id="role-list">
                    <!-- Roles will be populated here -->
                </div>
                <div class="tray-actions">
                    <button class="action-btn" onclick="pwaTray.openSettings()">
                        <i class="fas fa-cog"></i> Settings
                    </button>
                    <button class="action-btn" onclick="pwaTray.showAbout()">
                        <i class="fas fa-info-circle"></i> About
                    </button>
                </div>
            </div>
        `;

        document.body.appendChild(this.floatingButton);

        // Add event listeners
        const trayButton = this.floatingButton.querySelector('.tray-button');
        trayButton?.addEventListener('click', () => this.toggleTrayMenu());

        // Load available roles
        this.loadRoles();
    }

    private async loadRoles() {
        try {
            const config = await this.invoke('get_config');
            const roleList = this.floatingButton?.querySelector('#role-list');

            if (roleList && config.roles) {
                roleList.innerHTML = '';

                Object.entries(config.roles).forEach(([roleName, role]) => {
                    const roleItem = document.createElement('div');
                    roleItem.className = `role-item ${roleName === this.currentRole ? 'active' : ''}`;
                    roleItem.innerHTML = `
                        <i class="fas fa-${roleName === this.currentRole ? 'check' : 'circle'}"></i>
                        <span>${roleName}</span>
                        ${role.terraphim_it ? '<i class="fas fa-robot role-badge" title="Terraphim IT"></i>' : ''}
                    `;

                    roleItem.addEventListener('click', () => this.switchRole(roleName));
                    roleList.appendChild(roleItem);
                });
            }
        } catch (error) {
            console.error('Failed to load roles:', error);
        }
    }

    private toggleTrayMenu() {
        const trayMenu = this.floatingButton?.querySelector('.tray-menu');
        if (trayMenu) {
            const isVisible = trayMenu.style.display !== 'none';
            trayMenu.style.display = isVisible ? 'none' : 'block';

            if (!isVisible) {
                this.loadRoles(); // Refresh roles when opening
            }
        }
    }

    private async switchRole(roleName: string) {
        try {
            await this.invoke('select_role', { roleName });
            this.currentRole = roleName;

            // Update UI
            const roleIndicator = this.floatingButton?.querySelector('.role-indicator');
            if (roleIndicator) {
                roleIndicator.textContent = roleName;
            }

            // Show notification
            this.showNotification(`Switched to role: ${roleName}`);

            // Close menu
            this.toggleTrayMenu();

            // Reload roles to update active state
            this.loadRoles();
        } catch (error) {
            console.error('Failed to switch role:', error);
            this.showNotification('Failed to switch role', 'error');
        }
    }

    private setupBrowserAction() {
        // Set up browser action/badge if supported
        if ('serviceWorker' in navigator && 'PushManager' in window) {
            // Register for push notifications
            this.registerPushNotifications();
        }
    }

    private async registerPushNotifications() {
        try {
            const registration = await navigator.serviceWorker.ready;
            const subscription = await registration.pushManager.subscribe({
                userVisibleOnly: true,
                applicationServerKey: this.urlBase64ToUint8Array('YOUR_VAPID_PUBLIC_KEY')
            });

            // Send subscription to server
            await this.invoke('register_push_subscription', { subscription });
        } catch (error) {
            console.error('Failed to register push notifications:', error);
        }
    }

    private urlBase64ToUint8Array(base64String: string): Uint8Array {
        const padding = '='.repeat((4 - base64String.length % 4) % 4);
        const base64 = (base64String + padding)
            .replace(/-/g, '+')
            .replace(/_/g, '/');

        const rawData = window.atob(base64);
        const outputArray = new Uint8Array(rawData.length);

        for (let i = 0; i < rawData.length; ++i) {
            outputArray[i] = rawData.charCodeAt(i);
        }
        return outputArray;
    }

    private showNotification(message: string, type: 'info' | 'error' = 'info') {
        if ('Notification' in window && Notification.permission === 'granted') {
            new Notification('Terraphim', {
                body: message,
                icon: '/icons/icon-192x192.png',
                badge: '/icons/badge-72x72.png',
                tag: 'terraphim-role-change'
            });
        }
    }

    private async invoke(command: string, args?: any): Promise<any> {
        // This would call your API endpoints
        const response = await fetch(`/api/${command}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(args || {}),
        });

        if (!response.ok) {
            throw new Error(`API call failed: ${response.statusText}`);
        }

        return response.json();
    }

    openSettings() {
        window.location.href = '/config/wizard';
        this.toggleTrayMenu();
    }

    showAbout() {
        // Show about dialog
        this.toggleTrayMenu();
    }
}

// Global instance
window.pwaTray = new PWASystemTrayManager();

// CSS for floating tray
const style = document.createElement('style');
style.textContent = `
    .pwa-floating-tray {
        position: fixed;
        bottom: 20px;
        right: 20px;
        z-index: 9999;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    }

    .tray-button {
        background: #3273dc;
        color: white;
        border: none;
        border-radius: 50%;
        width: 60px;
        height: 60px;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-direction: column;
        cursor: pointer;
        box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        transition: all 0.3s ease;
    }

    .tray-button:hover {
        transform: translateY(-2px);
        box-shadow: 0 6px 16px rgba(0,0,0,0.2);
    }

    .role-indicator {
        font-size: 10px;
        margin-top: 2px;
        font-weight: bold;
    }

    .tray-menu {
        position: absolute;
        bottom: 70px;
        right: 0;
        background: white;
        border-radius: 8px;
        box-shadow: 0 8px 24px rgba(0,0,0,0.15);
        min-width: 250px;
        overflow: hidden;
    }

    .tray-header {
        padding: 12px 16px;
        background: #f8f9fa;
        border-bottom: 1px solid #e9ecef;
        font-weight: 600;
        font-size: 14px;
    }

    .role-list {
        max-height: 200px;
        overflow-y: auto;
    }

    .role-item {
        padding: 12px 16px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 8px;
        transition: background-color 0.2s ease;
    }

    .role-item:hover {
        background-color: #f8f9fa;
    }

    .role-item.active {
        background-color: #e3f2fd;
        color: #1976d2;
    }

    .role-badge {
        margin-left: auto;
        font-size: 10px;
        color: #666;
    }

    .tray-actions {
        padding: 8px;
        border-top: 1px solid #e9ecef;
        display: flex;
        gap: 4px;
    }

    .action-btn {
        flex: 1;
        padding: 8px 12px;
        border: none;
        background: transparent;
        cursor: pointer;
        border-radius: 4px;
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        color: #666;
        transition: all 0.2s ease;
    }

    .action-btn:hover {
        background-color: #f8f9fa;
        color: #333;
    }
`;
document.head.appendChild(style);
```

#### 3.2 PWA Auto-Updater
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Implement service worker-based update checking
- Create update notification system
- Add silent update capabilities
- Implement version comparison
- Create update scheduling

**PWA Update System**:
```typescript
// src/lib/pwa-updater.ts
class PWAUpdater {
    private registration: ServiceWorkerRegistration | null = null;
    private updateAvailable = false;
    private updateCallbacks: Array<(updateInfo: UpdateInfo) => void> = [];

    async initialize() {
        if ('serviceWorker' in navigator) {
            this.registration = await navigator.serviceWorker.register('/sw.js');
            this.setupUpdateListener();
            this.checkForUpdates();
        }
    }

    private setupUpdateListener() {
        if (!this.registration) return;

        // Listen for updates
        this.registration.addEventListener('updatefound', () => {
            const newWorker = this.registration!.installing;
            if (newWorker) {
                newWorker.addEventListener('statechange', () => {
                    if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
                        this.updateAvailable = true;
                        this.notifyUpdateAvailable();
                    }
                });
            }
        });
    }

    private async checkForUpdates() {
        if (!this.registration) return;

        try {
            await this.registration.update();
        } catch (error) {
            console.error('Failed to check for updates:', error);
        }
    }

    private async notifyUpdateAvailable() {
        const updateInfo = await this.getUpdateInfo();

        // Notify all callbacks
        this.updateCallbacks.forEach(callback => callback(updateInfo));

        // Show notification
        this.showUpdateNotification(updateInfo);
    }

    private async getUpdateInfo(): Promise<UpdateInfo> {
        try {
            const response = await fetch('/api/version');
            const versionInfo = await response.json();

            return {
                version: versionInfo.version,
                currentVersion: this.getCurrentVersion(),
                notes: versionInfo.notes || '',
                availableAt: new Date().toISOString()
            };
        } catch (error) {
            console.error('Failed to get update info:', error);
            return {
                version: 'Unknown',
                currentVersion: this.getCurrentVersion(),
                notes: 'Update available',
                availableAt: new Date().toISOString()
            };
        }
    }

    private getCurrentVersion(): string {
        // Get version from package.json or build info
        return '0.3.0'; // This should be injected at build time
    }

    private showUpdateNotification(updateInfo: UpdateInfo) {
        if ('Notification' in window && Notification.permission === 'granted') {
            const notification = new Notification('Terraphim Update Available', {
                body: `Version ${updateInfo.version} is ready to install`,
                icon: '/icons/icon-192x192.png',
                badge: '/icons/badge-72x72.png',
                tag: 'terraphim-update',
                requireInteraction: true,
                actions: [
                    {
                        action: 'install',
                        title: 'Install Now'
                    },
                    {
                        action: 'later',
                        title: 'Later'
                    }
                ]
            });

            notification.addEventListener('click', () => {
                this.applyUpdate();
            });

            notification.addEventListener('close', () => {
                // User closed notification, schedule reminder
                this.scheduleUpdateReminder();
            });
        }
    }

    private scheduleUpdateReminder() {
        // Schedule reminder in 1 hour
        setTimeout(() => {
            if (this.updateAvailable) {
                this.showUpdateNotification(this.getUpdateInfo() as Promise<UpdateInfo>);
            }
        }, 60 * 60 * 1000);
    }

    async applyUpdate() {
        if (!this.updateAvailable || !this.registration) return;

        try {
            // Tell the new service worker to skip waiting
            const newWorker = this.registration.waiting;
            if (newWorker) {
                newWorker.postMessage({ type: 'SKIP_WAITING' });

                // Wait for the new service worker to become active
                newWorker.addEventListener('statechange', () => {
                    if (newWorker.state === 'activated') {
                        // Reload the page to apply the update
                        window.location.reload();
                    }
                });
            }
        } catch (error) {
            console.error('Failed to apply update:', error);
        }
    }

    onUpdateAvailable(callback: (updateInfo: UpdateInfo) => void) {
        this.updateCallbacks.push(callback);
    }

    isUpdateAvailable(): boolean {
        return this.updateAvailable;
    }
}

interface UpdateInfo {
    version: string;
    currentVersion: string;
    notes: string;
    availableAt: string;
}

export const pwaUpdater = new PWAUpdater();
```

#### 3.3 WASM-Enhanced Autocomplete for Web
**Timeline**: 1-2 weeks
**Effort**: High

**Tasks**:
- Compile existing Rust autocomplete to WASM
- Implement web-compatible autocomplete service
- Create streaming autocomplete API
- Add browser-based thesaurus loading
- Implement offline autocomplete caching

**Web WASM Autocomplete**:
```typescript
// src/lib/web-wasm-autocomplete.ts
class WebWASMAutocompleteManager {
    private wasmModule: any = null;
    private autocompleteEngine: any = null;
    private initialized = false;
    private thesaurusCache = new Map<string, any>();

    async initialize() {
        if (this.initialized) return;

        try {
            // Load WASM module
            this.wasmModule = await import('../wasm/pkg/terraphim_wasm.js');
            await this.wasmModule.default();

            this.autocompleteEngine = new this.wasmModule.AutocompleteEngine();
            this.initialized = true;

            console.log('Web WASM Autocomplete engine initialized');
        } catch (error) {
            console.error('Failed to initialize Web WASM autocomplete:', error);
        }
    }

    async loadThesaurus(roleName: string): Promise<void> {
        if (!this.autocompleteEngine) {
            throw new Error('WASM engine not initialized');
        }

        // Check cache first
        if (this.thesaurusCache.has(roleName)) {
            const cachedData = this.thesaurusCache.get(roleName);
            this.autocompleteEngine.load_thesaurus(JSON.stringify(cachedData));
            return;
        }

        try {
            // Load thesaurus data via API
            const response = await fetch(`/api/thesaurus/${roleName}`);
            if (!response.ok) {
                throw new Error(`Failed to load thesaurus: ${response.statusText}`);
            }

            const thesaurusData = await response.json();

            // Cache the data
            this.thesaurusCache.set(roleName, thesaurusData);

            // Load into WASM engine
            this.autocompleteEngine.load_thesaurus(JSON.stringify(thesaurusData));

            // Store in IndexedDB for offline use
            await this.cacheThesaurusOffline(roleName, thesaurusData);

            console.log(`Thesaurus loaded for role: ${roleName}`);
        } catch (error) {
            console.error('Failed to load thesaurus:', error);

            // Try to load from offline cache
            await this.loadThesaurusOffline(roleName);
        }
    }

    private async cacheThesaurusOffline(roleName: string, data: any): Promise<void> {
        try {
            const db = await this.openIndexedDB();
            const tx = db.transaction(['thesaurus'], 'readwrite');
            const store = tx.objectStore('thesaurus');
            await store.put({ id: roleName, data, timestamp: Date.now() });
            await tx.complete;
        } catch (error) {
            console.error('Failed to cache thesaurus offline:', error);
        }
    }

    private async loadThesaurusOffline(roleName: string): Promise<void> {
        try {
            const db = await this.openIndexedDB();
            const tx = db.transaction(['thesaurus'], 'readonly');
            const store = tx.objectStore('thesaurus');
            const result = await store.get(roleName);

            if (result && result.data) {
                this.autocompleteEngine.load_thesaurus(JSON.stringify(result.data));
                console.log(`Loaded thesaurus from offline cache: ${roleName}`);
            }
        } catch (error) {
            console.error('Failed to load thesaurus from offline cache:', error);
        }
    }

    private async openIndexedDB(): Promise<IDBDatabase> {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open('TerraphimAutocomplete', 1);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(request.result);

            request.onupgradeneeded = (event) => {
                const db = (event.target as IDBOpenDBRequest).result;
                if (!db.objectStoreNames.contains('thesaurus')) {
                    db.createObjectStore('thesaurus');
                }
            };
        });
    }

    async getSuggestions(query: string, limit: number = 10): Promise<string[]> {
        if (!this.autocompleteEngine) {
            return [];
        }

        try {
            // Use WASM for fast autocomplete
            const suggestions = this.autocompleteEngine.get_suggestions(query, limit);
            return JSON.parse(suggestions);
        } catch (error) {
            console.error('Failed to get suggestions:', error);
            return [];
        }
    }

    async getContextualSuggestions(
        query: string,
        context: string,
        limit: number = 10
    ): Promise<Array<{term: string, score: number, context: string}>> {
        if (!this.autocompleteEngine) {
            return [];
        }

        try {
            const results = this.autocompleteEngine.get_contextual_suggestions(
                query,
                context,
                limit
            );
            return JSON.parse(results);
        } catch (error) {
            console.error('Failed to get contextual suggestions:', error);
            return [];
        }
    }

    async addCustomTerm(term: string, weight: number = 1.0): Promise<void> {
        if (!this.autocompleteEngine) {
            return;
        }

        try {
            this.autocompleteEngine.add_custom_term(term, weight);
        } catch (error) {
            console.error('Failed to add custom term:', error);
        }
    }

    isInitialized(): boolean {
        return this.initialized && this.autocompleteEngine !== null;
    }

    // Streaming autocomplete for real-time suggestions
    async *streamSuggestions(query: string, limit: number = 10): AsyncGenerator<string[]> {
        if (!this.autocompleteEngine) {
            return;
        }

        try {
            // Implement streaming suggestions if supported by WASM
            const stream = this.autocompleteEngine.stream_suggestions(query, limit);

            for (const chunk of stream) {
                yield JSON.parse(chunk);
            }
        } catch (error) {
            console.error('Failed to stream suggestions:', error);
        }
    }
}

export const webWASMAutocomplete = new WebWASMAutocompleteManager();
```

#### 3.4 Real-time Communication
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Implement WebSocket connections for real-time updates
- Add live search suggestions
- Create real-time collaboration features
- Implement push notifications
- Add presence indicators

**WebSocket Implementation**:
```typescript
// src/lib/websocket.ts
class WebSocketManager {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  connect(url: string) {
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
    };

    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      this.handleMessage(data);
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      this.attemptReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }

  private attemptReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      setTimeout(() => {
        this.reconnectAttempts++;
        this.connect(this.ws!.url);
      }, this.reconnectDelay * Math.pow(2, this.reconnectAttempts));
    }
  }

  private handleMessage(data: any) {
    // Handle different message types
    switch (data.type) {
      case 'search_update':
        // Update search results
        break;
      case 'chat_message':
        // Handle new chat messages
        break;
      case 'notification':
        // Show notification
        this.showNotification(data.payload);
        break;
    }
  }

  private showNotification(payload: any) {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(payload.title, {
        body: payload.body,
        icon: '/icons/icon-192x192.png'
      });
    }
  }
}

export const wsManager = new WebSocketManager();
```

#### 3.2 Enhanced Mobile Experience
**Timeline**: 1-2 weeks
**Effort**: Medium

**Tasks**:
- Optimize touch interactions
- Implement mobile-specific UI patterns
- Add gesture support
- Create mobile-first navigation
- Optimize performance for mobile devices

#### 3.3 Desktop Integration via Web APIs
**Timeline**: 1 week
**Effort**: Low

**Tasks**:
- Implement Web Share API for sharing content
- Add Web Notifications for system notifications
- Use Screen Wake Lock API for long-running operations
- Implement Clipboard API for copy/paste operations
- Add Badging API for unread counts

### Phase 4: Deployment & Infrastructure (2-3 weeks)

#### 4.1 Cloud Deployment Setup
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Set up containerized deployment (Docker)
- Configure cloud hosting (AWS/GCP/Azure)
- Implement CI/CD pipeline
- Set up monitoring and logging
- Configure backup and disaster recovery

**Docker Configuration**:
```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/terraphim-web /usr/local/bin/

EXPOSE 8080

CMD ["terraphim-web"]
```

#### 4.2 CDN & Static Asset Optimization
**Timeline**: 1 week
**Effort**: Low

**Tasks**:
- Set up CDN for static assets
- Implement asset compression and optimization
- Configure cache headers
- Add image optimization
- Implement bundle splitting

#### 4.3 Monitoring & Analytics
**Timeline**: 1 week
**Effort**: Low

**Tasks**:
- Implement error tracking (Sentry)
- Add performance monitoring
- Create usage analytics
- Set up health checks
- Implement alerting

### Phase 5: Testing & Migration (2 weeks)

#### 5.1 Comprehensive Testing
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Migrate existing tests to web environment
- Add cross-browser testing
- Implement visual regression testing
- Create performance testing suite
- Add security testing

#### 5.2 User Migration Strategy
**Timeline**: 1 week
**Effort**: Low

**Tasks**:
- Create data migration tools
- Implement user onboarding flow
- Create migration documentation
- Set up user support channels
- Plan gradual rollout

## Technical Specifications

### New Architecture

**Frontend (PWA)**:
- SvelteKit with static adapter
- Service Worker for offline functionality
- Web APIs for native-like features
- Responsive design for all devices

**Backend (Web Service)**:
- Axum web framework (Rust)
- RESTful API with OpenAPI spec
- WebSocket for real-time features
- JWT authentication

**Deployment**:
- Docker containers
- Cloud hosting (AWS/GCP/Azure)
- CDN for static assets
- Managed database services

### Performance Targets

**Web Performance**:
- First Contentful Paint: <1.5s
- Largest Contentful Paint: <2.5s
- Time to Interactive: <3.5s
- Cumulative Layout Shift: <0.1

**Mobile Performance**:
- Load time on 3G: <5s
- Bundle size: <1MB compressed
- Memory usage: <100MB on mobile
- Battery impact: Minimal

### Browser Support

**Desktop Browsers**:
- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

**Mobile Browsers**:
- Chrome Mobile 90+
- Safari Mobile 14+
- Samsung Internet 15+
- Firefox Mobile 88+

## Benefits of PWA Approach

### 1. Cross-Platform Compatibility
- Single codebase for all platforms
- No platform-specific development
- Automatic updates
- Reduced maintenance overhead

### 2. Accessibility & Reach
- No installation required
- Search engine discoverable
- Shareable via links
- Lower barrier to entry

### 3. Modern Web Capabilities
- Offline functionality
- Push notifications
- Background sync
- Device API access

### 4. Cost Efficiency
- No app store fees
- Simplified deployment
- Reduced development costs
- Easier maintenance

### 5. Future-Proof
- Standards-based technology
- Continuous browser improvements
- No vendor lock-in
- Easy feature additions

## Challenges & Limitations

### Technical Challenges

**Browser API Limitations**:
- File system access restrictions
- Limited system integration
- Sandboxed execution environment
- Variable API support across browsers

**Performance Considerations**:
- Higher memory usage than native
- Slower startup compared to desktop apps
- Limited background processing
- Battery consumption on mobile

**Security Constraints**:
- CORS restrictions
- Same-origin policy
- Limited local storage
- Secure context requirements

### User Experience Challenges

**Installation Discovery**:
- Users may not realize they can "install"
- Different installation flows per browser
- Less discoverable than app stores
- No centralized update mechanism

**Feature Limitations**:
- No system tray integration
- Limited global shortcuts
- No deep system integration
- Restricted background processing

## Risks & Mitigation Strategies

### High-Risk Items

**Performance Regression**:
- Risk: Slower than native desktop app
- Mitigation: Performance optimization, progressive loading

**Browser Compatibility Issues**:
- Risk: Inconsistent behavior across browsers
- Mitigation: Comprehensive testing, polyfills, feature detection

**Offline Functionality Complexity**:
- Risk: Complex sync logic and data conflicts
- Mitigation: Robust sync algorithms, conflict resolution

### Medium-Risk Items

**Security Concerns**:
- Risk: Web-based security vulnerabilities
- Mitigation: Security headers, CSP, regular audits

**User Adoption**:
- Risk: Users prefer native apps
- Mitigation: User education, seamless experience

## Resource Requirements

### Development Team
- **Frontend Developer**: 1 (full-time for 10-12 weeks)
- **Backend Developer**: 1 (full-time for 8-10 weeks)
- **DevOps Engineer**: 1 (part-time for 4-6 weeks)
- **QA Engineer**: 1 (part-time for 6-8 weeks)

### Infrastructure Costs
- Cloud hosting: $200-500/month
- CDN services: $50-100/month
- Database services: $100-300/month
- Monitoring tools: $50-150/month

## Success Metrics

### Technical Metrics
- <2s page load time
- 95%+ uptime
- 90%+ Lighthouse performance score
- Zero critical security vulnerabilities

### User Metrics
- 70%+ browser install rate
- 80%+ user retention after 30 days
- 4.0+ user satisfaction rating
- <5% support ticket rate

### Business Metrics
- 50% reduction in development costs
- 30% faster feature delivery
- 90%+ cross-platform parity
- 25% increase in user acquisition

## Timeline Summary

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Phase 1: Backend Migration | 3-4 weeks | Week 1 | Week 4 |
| Phase 2: PWA Conversion | 4-5 weeks | Week 4 | Week 9 |
| Phase 3: Advanced Features | 3-4 weeks | Week 8 | Week 12 |
| Phase 4: Deployment Setup | 2-3 weeks | Week 11 | Week 14 |
| Phase 5: Testing & Migration | 2 weeks | Week 13 | Week 15 |

**Total Duration**: 15-17 weeks (4-5 months)

This migration plan transforms the desktop application into a modern PWA while maintaining core functionality and adding cross-platform compatibility, offline capabilities, and improved accessibility.
