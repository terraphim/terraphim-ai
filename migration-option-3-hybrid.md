# Migration Option 3: Hybrid Electron/Tauri Migration Plan

## Overview

This migration plan creates a hybrid architecture that combines the strengths of both Electron and Tauri, allowing for gradual migration and maximum flexibility. The approach uses Electron for cross-platform compatibility while leveraging Tauri's performance benefits where possible.

## Migration Strategy

### Phase 1: Hybrid Architecture Design (2-3 weeks)

#### 1.1 Architecture Planning
**Timeline**: 1 week
**Effort**: High

**Tasks**:
- Design hybrid architecture with shared components
- Define component boundaries between Electron and Tauri
- Create migration roadmap with incremental steps
- Establish communication protocols between architectures
- Design feature flag system for gradual rollout

**Hybrid Architecture Overview**:
```
hybrid-app/
├── shared/                    # Shared codebase
│   ├── frontend/              # Svelte components (shared)
│   │   ├── src/
│   │   │   ├── lib/          # Reusable components
│   │   │   ├── stores/       # State management
│   │   │   └── types/        # TypeScript definitions
│   │   └── package.json
│   ├── backend/              # Rust services (shared)
│   │   ├── src/
│   │   │   ├── services/     # Business logic
│   │   │   ├── models/       # Data models
│   │   │   └── utils/        # Utilities
│   │   └── Cargo.toml
│   └── types/                # Shared type definitions
├── electron-app/             # Electron implementation
│   ├── src/
│   │   ├── main/             # Electron main process
│   │   ├── renderer/         # Electron renderer process
│   │   └── preload/          # Preload scripts
│   ├── package.json
│   └── webpack.config.js
├── tauri-app/                # Tauri implementation
│   ├── src-tauri/            # Tauri backend
│   ├── src/                  # Frontend (symlink to shared)
│   └── Cargo.toml
└── build-tools/              # Shared build scripts
    ├── webpack.common.js
    ├── build-electron.js
    └── build-tauri.js
```

#### 1.2 Shared Component Library
**Timeline**: 1-2 weeks
**Effort**: High

**Tasks**:
- Extract shared Svelte components
- Create abstraction layer for platform-specific APIs
- Implement platform detection and conditional logic
- Create shared state management system
- Establish build system for shared components

**Platform Abstraction Layer**:
```typescript
// shared/frontend/src/lib/platform.ts
export interface PlatformAPI {
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  showNotification(title: string, body: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  getAppVersion(): Promise<string>;
}

class ElectronPlatform implements PlatformAPI {
  async readFile(path: string): Promise<string> {
    return window.electronAPI.readFile(path);
  }

  async writeFile(path: string, content: string): Promise<void> {
    return window.electronAPI.writeFile(path, content);
  }

  async showNotification(title: string, body: string): Promise<void> {
    return window.electronAPI.showNotification(title, body);
  }

  async openExternal(url: string): Promise<void> {
    return window.electronAPI.openExternal(url);
  }

  async getAppVersion(): Promise<string> {
    return window.electronAPI.getAppVersion();
  }
}

class TauriPlatform implements PlatformAPI {
  async readFile(path: string): Promise<string> {
    const { readTextFile } = await import('@tauri-apps/api/fs');
    return readTextFile(path);
  }

  async writeFile(path: string, content: string): Promise<void> {
    const { writeTextFile } = await import('@tauri-apps/api/fs');
    return writeTextFile(path, content);
  }

  async showNotification(title: string, body: string): Promise<void> {
    const { isPermissionGranted, requestPermission, sendNotification } =
      await import('@tauri-apps/api/notification');

    if (await isPermissionGranted()) {
      sendNotification({ title, body });
    } else {
      await requestPermission();
      sendNotification({ title, body });
    }
  }

  async openExternal(url: string): Promise<void> {
    const { open } = await import('@tauri-apps/api/shell');
    return open(url);
  }

  async getAppVersion(): Promise<string> {
    const { getVersion } = await import('@tauri-apps/api/app');
    return getVersion();
  }
}

export const platformAPI: PlatformAPI =
  typeof window !== 'undefined' && 'electronAPI' in window
    ? new ElectronPlatform()
    : new TauriPlatform();
```

### Phase 2: Electron Implementation (4-5 weeks)

#### 2.1 Electron Main Process Setup
**Timeline**: 2 weeks
**Effort**: High

**Tasks**:
- Set up Electron main process with security best practices
- Implement secure IPC communication
- Create preload scripts for API exposure
- Set up auto-updater functionality
- Configure application signing and distribution

**Electron Main Process**:
```typescript
// electron-app/src/main/main.ts
import { app, BrowserWindow, ipcMain, Menu, shell } from 'electron';
import { join } from 'path';
import { autoUpdater } from 'electron-updater';

class ElectronApp {
  private mainWindow: BrowserWindow | null = null;
  private isDev = process.env.NODE_ENV === 'development';

  constructor() {
    this.initializeApp();
  }

  private initializeApp() {
    app.whenReady().then(() => {
      this.createMainWindow();
      this.setupMenu();
      this.setupIPC();
      this.setupAutoUpdater();
    });

    app.on('window-all-closed', () => {
      if (process.platform !== 'darwin') {
        app.quit();
      }
    });

    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        this.createMainWindow();
      }
    });
  }

  private createMainWindow() {
    this.mainWindow = new BrowserWindow({
      width: 1200,
      height: 800,
      minWidth: 800,
      minHeight: 600,
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        enableRemoteModule: false,
        preload: join(__dirname, '../preload/preload.js'),
        webSecurity: !this.isDev,
      },
      icon: join(__dirname, '../../../assets/icon.png'),
      show: false,
    });

    // Load the app
    if (this.isDev) {
      this.mainWindow.loadURL('http://localhost:5173');
      this.mainWindow.webContents.openDevTools();
    } else {
      this.mainWindow.loadFile(join(__dirname, '../renderer/index.html'));
    }

    this.mainWindow.once('ready-to-show', () => {
      this.mainWindow?.show();
    });

    this.mainWindow.on('closed', () => {
      this.mainWindow = null;
    });
  }

  private setupMenu() {
    const template: Electron.MenuItemConstructorOptions[] = [
      {
        label: 'File',
        submenu: [
          {
            label: 'New Conversation',
            accelerator: 'CmdOrCtrl+N',
            click: () => {
              this.mainWindow?.webContents.send('new-conversation');
            },
          },
          { type: 'separator' },
          {
            label: 'Quit',
            accelerator: process.platform === 'darwin' ? 'Cmd+Q' : 'Ctrl+Q',
            click: () => {
              app.quit();
            },
          },
        ],
      },
      {
        label: 'Edit',
        submenu: [
          { role: 'undo' },
          { role: 'redo' },
          { type: 'separator' },
          { role: 'cut' },
          { role: 'copy' },
          { role: 'paste' },
        ],
      },
    ];

    const menu = Menu.buildFromTemplate(template);
    Menu.setApplicationMenu(menu);
  }

  private setupIPC() {
    // File operations
    ipcMain.handle('file:read', async (_, path: string) => {
      try {
        const fs = await import('fs/promises');
        return await fs.readFile(path, 'utf-8');
      } catch (error) {
        throw new Error(`Failed to read file: ${error}`);
      }
    });

    ipcMain.handle('file:write', async (_, path: string, content: string) => {
      try {
        const fs = await import('fs/promises');
        await fs.writeFile(path, content, 'utf-8');
      } catch (error) {
        throw new Error(`Failed to write file: ${error}`);
      }
    });

    // Notifications
    ipcMain.handle('notification:show', (_, title: string, body: string) => {
      const { Notification } = require('electron');
      new Notification({ title, body }).show();
    });

    // External links
    ipcMain.handle('shell:openExternal', (_, url: string) => {
      shell.openExternal(url);
    });

    // App version
    ipcMain.handle('app:getVersion', () => {
      return app.getVersion();
    });
  }

  private setupAutoUpdater() {
    if (this.isDev) return;

    autoUpdater.checkForUpdatesAndNotify();

    autoUpdater.on('update-available', () => {
      this.mainWindow?.webContents.send('update-available');
    });

    autoUpdater.on('update-downloaded', () => {
      this.mainWindow?.webContents.send('update-downloaded');
    });
  }
}

new ElectronApp();
```

#### 2.2 Electron Preload Scripts
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Create secure preload scripts
- Expose safe APIs to renderer process
- Implement context isolation
- Add type definitions for preload APIs

**Preload Script**:
```typescript
// electron-app/src/preload/preload.ts
import { contextBridge, ipcRenderer } from 'electron';

export interface ElectronAPI {
  // File operations
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;

  // Notifications
  showNotification(title: string, body: string): Promise<void>;

  // Shell operations
  openExternal(url: string): Promise<void>;

  // App info
  getAppVersion(): Promise<string>;

  // Events
  onUpdateAvailable(callback: () => void): void;
  onUpdateDownloaded(callback: () => void): void;
  onNewConversation(callback: () => void): void;
}

const electronAPI: ElectronAPI = {
  readFile: (path: string) => ipcRenderer.invoke('file:read', path),
  writeFile: (path: string, content: string) =>
    ipcRenderer.invoke('file:write', path, content),
  showNotification: (title: string, body: string) =>
    ipcRenderer.invoke('notification:show', title, body),
  openExternal: (url: string) => ipcRenderer.invoke('shell:openExternal', url),
  getAppVersion: () => ipcRenderer.invoke('app:getVersion'),

  onUpdateAvailable: (callback: () => void) =>
    ipcRenderer.on('update-available', callback),
  onUpdateDownloaded: (callback: () => void) =>
    ipcRenderer.on('update-downloaded', callback),
  onNewConversation: (callback: () => void) =>
    ipcRenderer.on('new-conversation', callback),
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);

// Type augmentation for TypeScript
declare global {
  interface Window {
    electronAPI: ElectronAPI;
  }
}
```

#### 2.3 Electron Renderer Process
**Timeline**: 1-2 weeks
**Effort**: Medium

**Tasks**:
- Set up webpack configuration for renderer
- Configure development and production builds
- Implement hot module replacement
- Set up source maps and debugging

### Phase 3: Tauri Integration (3-4 weeks)

#### 3.1 Tauri Backend Services
**Timeline**: 2 weeks
**Effort**: High

**Tasks**:
- Migrate existing Tauri backend to shared architecture
- Implement Tauri commands for shared services
- Set up Tauri configuration
- Create Tauri-specific build scripts

**Tauri Commands**:
```rust
// tauri-app/src-tauri/src/commands.rs
use serde_json::Value;
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import shared services
use shared_backend::services::{SearchService, ChatService, ConfigService};

#[tauri::command]
async fn search(
    query: String,
    limit: Option<usize>,
    search_service: State<'_, Arc<Mutex<SearchService>>>,
) -> Result<Vec<Value>, String> {
    let service = search_service.lock().await;
    service
        .search(&query, limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_chat_message(
    message: String,
    conversation_id: Option<String>,
    chat_service: State<'_, Arc<Mutex<ChatService>>>,
) -> Result<Value, String> {
    let service = chat_service.lock().await;
    service
        .send_message(&message, conversation_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config(
    config_service: State<'_, Arc<Mutex<ConfigService>>>,
) -> Result<Value, String> {
    let service = config_service.lock().await;
    service
        .get_config()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_config(
    config: Value,
    config_service: State<'_, Arc<Mutex<ConfigService>>>,
) -> Result<Value, String> {
    let service = config_service.lock().await;
    service
        .update_config(config)
        .await
        .map_err(|e| e.to_string())
}

pub fn get_handlers() -> Vec<tauri::InvokeHandler> {
    vec![
        tauri::generate_handler![
            search,
            send_chat_message,
            get_config,
            update_config
        ]
    ]
}
```

#### 3.2 Tauri Configuration
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Configure Tauri build settings
- Set up security policies
- Configure application metadata
- Set up development and production builds

**Tauri Configuration**:
```json
{
  "build": {
    "beforeBuildCommand": "npm run build:shared",
    "beforeDevCommand": "npm run dev:shared",
    "devPath": "http://localhost:5173",
    "distDir": "../shared/frontend/dist"
  },
  "package": {
    "productName": "Terraphim AI",
    "version": "0.3.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "fs": {
        "all": false,
        "readFile": true,
        "writeFile": true,
        "scope": ["$APPDATA/*", "$DOCUMENT/*", "$DOWNLOAD/*"]
      },
      "notification": {
        "all": true
      },
      "shell": {
        "all": false,
        "open": true
      }
    },
    "bundle": {
      "active": true,
      "targets": "all",
      "identifier": "ai.terraphim.desktop",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    },
    "security": {
      "csp": null
    },
    "updater": {
      "active": false
    },
    "windows": [
      {
        "fullscreen": false,
        "resizable": true,
        "title": "Terraphim AI",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600
      }
    ]
  }
}
```

### Phase 4: Shared Services Integration (3-4 weeks)

#### 4.1 Cross-Platform System Tray & Role Management
**Timeline**: 2 weeks
**Effort**: High

**Tasks**:
- Create unified system tray interface for both platforms
- Implement role switching across Electron and Tauri
- Create shared role management service
- Implement platform-specific optimizations
- Add role persistence and synchronization

**Unified System Tray Interface**:
```typescript
// shared/frontend/src/lib/system-tray.ts
export interface SystemTrayAPI {
  initialize(): Promise<void>;
  showRoleMenu(): Promise<void>;
  switchRole(roleName: string): Promise<void>;
  showNotification(title: string, body: string): Promise<void>;
  updateRoleIndicator(roleName: string): Promise<void>;
  setQuickActions(actions: QuickAction[]): Promise<void>;
}

export interface QuickAction {
  id: string;
  label: string;
  icon: string;
  accelerator?: string;
  action: () => void | Promise<void>;
}

class UnifiedSystemTrayManager {
  private api: SystemTrayAPI;
  private currentRole = 'default';
  private roleChangeCallbacks: Array<(roleName: string) => void> = [];

  constructor() {
    this.api = this.createPlatformAPI();
  }

  private createPlatformAPI(): SystemTrayAPI {
    if (typeof window !== 'undefined' && 'electronAPI' in window) {
      return new ElectronSystemTrayAPI();
    } else {
      return new TauriSystemTrayAPI();
    }
  }

  async initialize() {
    await this.api.initialize();
    await this.loadCurrentRole();
    await this.setupRoleChangeListeners();
  }

  private async loadCurrentRole() {
    try {
      const config = await this.invoke('get_config');
      this.currentRole = config.selected_role || 'default';
      await this.api.updateRoleIndicator(this.currentRole);
    } catch (error) {
      console.error('Failed to load current role:', error);
    }
  }

  private async setupRoleChangeListeners() {
    // Listen for role changes from other parts of the app
    window.addEventListener('role-changed', (event: any) => {
      this.handleRoleChange(event.detail.roleName);
    });
  }

  async switchRole(roleName: string) {
    try {
      await this.api.switchRole(roleName);
      this.currentRole = roleName;

      // Notify listeners
      this.roleChangeCallbacks.forEach(callback => callback(roleName));

      // Emit global event
      window.dispatchEvent(new CustomEvent('role-changed', {
        detail: { roleName }
      }));

      // Update autocomplete for new role
      await this.updateAutocompleteForRole(roleName);

    } catch (error) {
      console.error('Failed to switch role:', error);
      throw error;
    }
  }

  private async updateAutocompleteForRole(roleName: string) {
    // Update WASM autocomplete with new role's thesaurus
    if (window.wasmAutocomplete) {
      try {
        await window.wasmAutocomplete.loadThesaurus(roleName);
      } catch (error) {
        console.error('Failed to update autocomplete for role:', error);
      }
    }
  }

  async showRoleMenu() {
    await this.api.showRoleMenu();
  }

  async showNotification(title: string, body: string) {
    await this.api.showNotification(title, body);
  }

  onRoleChange(callback: (roleName: string) => void) {
    this.roleChangeCallbacks.push(callback);
  }

  getCurrentRole(): string {
    return this.currentRole;
  }

  private async invoke(command: string, args?: any): Promise<any> {
    if (typeof window !== 'undefined' && 'electronAPI' in window) {
      return window.electronAPI[command](args);
    } else {
      const { invoke } = await import('@tauri-apps/api/tauri');
      return invoke(command, args);
    }
  }
}

// Electron implementation
class ElectronSystemTrayAPI implements SystemTrayAPI {
  async initialize(): Promise<void> {
    // Electron system tray is initialized in main process
    console.log('Electron system tray initialized');
  }

  async showRoleMenu(): Promise<void> {
    // Send message to main process to show role menu
    await window.electronAPI.showRoleMenu();
  }

  async switchRole(roleName: string): Promise<void> {
    await window.electronAPI.switchRole(roleName);
  }

  async showNotification(title: string, body: string): Promise<void> {
    await window.electronAPI.showNotification(title, body);
  }

  async updateRoleIndicator(roleName: string): Promise<void> {
    await window.electronAPI.updateTrayRole(roleName);
  }

  async setQuickActions(actions: QuickAction[]): Promise<void> {
    await window.electronAPI.setTrayQuickActions(actions);
  }
}

// Tauri implementation
class TauriSystemTrayAPI implements SystemTrayAPI {
  async initialize(): Promise<void> {
    // Tauri system tray is initialized in Rust
    console.log('Tauri system tray initialized');
  }

  async showRoleMenu(): Promise<void> {
    const { emit } = await import('@tauri-apps/api/event');
    await emit('show-tray-menu');
  }

  async switchRole(roleName: string): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/tauri');
    await invoke('switch_tray_role', { roleName });
  }

  async showNotification(title: string, body: string): Promise<void> {
    const { isPermissionGranted, requestPermission, sendNotification } =
      await import('@tauri-apps/api/notification');

    if (await isPermissionGranted()) {
      sendNotification({ title, body });
    } else {
      await requestPermission();
      sendNotification({ title, body });
    }
  }

  async updateRoleIndicator(roleName: string): Promise<void> {
    const { emit } = await import('@tauri-apps/api/event');
    await emit('update-tray-role', { roleName });
  }

  async setQuickActions(actions: QuickAction[]): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/tauri');
    await invoke('set_tray_quick_actions', { actions });
  }
}

export const systemTrayManager = new UnifiedSystemTrayManager();
```

#### 4.2 Unified Auto-Updater Service
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Create cross-platform update manager
- Implement unified update checking
- Create platform-specific update mechanisms
- Add rollback capabilities
- Implement update scheduling

**Unified Update Manager**:
```typescript
// shared/frontend/src/lib/update-manager.ts
export interface UpdateAPI {
  checkForUpdates(silent?: boolean): Promise<UpdateInfo | null>;
  downloadAndInstall(updateInfo: UpdateInfo): Promise<void>;
  getCurrentVersion(): string;
  isUpdateAvailable(): Promise<boolean>;
}

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string;
  pubDate: string;
  signature?: string;
  downloadUrl?: string;
}

class UnifiedUpdateManager {
  private api: UpdateAPI;
  private updateCallbacks: Array<(updateInfo: UpdateInfo) => void> = [];

  constructor() {
    this.api = this.createPlatformAPI();
  }

  private createPlatformAPI(): UpdateAPI {
    if (typeof window !== 'undefined' && 'electronAPI' in window) {
      return new ElectronUpdateAPI();
    } else {
      return new TauriUpdateAPI();
    }
  }

  async checkForUpdates(silent = false): Promise<UpdateInfo | null> {
    try {
      const updateInfo = await this.api.checkForUpdates(silent);

      if (updateInfo) {
        this.notifyUpdateAvailable(updateInfo);
      }

      return updateInfo;
    } catch (error) {
      console.error('Failed to check for updates:', error);
      if (!silent) {
        this.showUpdateError(error);
      }
      return null;
    }
  }

  async downloadAndInstall(updateInfo: UpdateInfo): Promise<void> {
    try {
      await this.api.downloadAndInstall(updateInfo);
    } catch (error) {
      console.error('Failed to install update:', error);
      this.showUpdateError(error);
    }
  }

  async getCurrentVersion(): Promise<string> {
    return this.api.getCurrentVersion();
  }

  async isUpdateAvailable(): Promise<boolean> {
    return this.api.isUpdateAvailable();
  }

  onUpdateAvailable(callback: (updateInfo: UpdateInfo) => void) {
    this.updateCallbacks.push(callback);
  }

  private notifyUpdateAvailable(updateInfo: UpdateInfo) {
    this.updateCallbacks.forEach(callback => callback(updateInfo));

    // Show notification
    this.showUpdateNotification(updateInfo);
  }

  private showUpdateNotification(updateInfo: UpdateInfo) {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification('Terraphim Update Available', {
        body: `Version ${updateInfo.version} is ready to install`,
        icon: '/icons/icon-192x192.png',
        tag: 'terraphim-update',
        requireInteraction: true
      });
    }
  }

  private showUpdateError(error: any) {
    console.error('Update error:', error);

    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification('Terraphim Update Failed', {
        body: 'Failed to check for updates. Please try again later.',
        icon: '/icons/icon-192x192.png',
        tag: 'terraphim-update-error'
      });
    }
  }
}

// Electron update API
class ElectronUpdateAPI implements UpdateAPI {
  async checkForUpdates(silent = false): Promise<UpdateInfo | null> {
    const updateInfo = await window.electronAPI.checkForUpdates(silent);
    return updateInfo;
  }

  async downloadAndInstall(updateInfo: UpdateInfo): Promise<void> {
    await window.electronAPI.downloadAndInstall(updateInfo);
  }

  async getCurrentVersion(): Promise<string> {
    return window.electronAPI.getAppVersion();
  }

  async isUpdateAvailable(): Promise<boolean> {
    return window.electronAPI.isUpdateAvailable();
  }
}

// Tauri update API
class TauriUpdateAPI implements UpdateAPI {
  async checkForUpdates(silent = false): Promise<UpdateInfo | null> {
    const { check } = await import('@tauri-apps/api/updater');

    try {
      const { available, manifest } = await check();

      if (available && manifest) {
        return {
          version: manifest.version,
          currentVersion: await this.getCurrentVersion(),
          notes: manifest.body || '',
          pubDate: manifest.date || '',
          signature: manifest.signature
        };
      }

      return null;
    } catch (error) {
      console.error('Tauri update check failed:', error);
      return null;
    }
  }

  async downloadAndInstall(updateInfo: UpdateInfo): Promise<void> {
    const { downloadAndInstall, install } = await import('@tauri-apps/api/updater');

    try {
      await downloadAndInstall((event) => {
        // Handle download progress
        console.log(`Download progress: ${event.event}`);
      });

      // Install and restart
      await install();
    } catch (error) {
      console.error('Tauri update install failed:', error);
      throw error;
    }
  }

  async getCurrentVersion(): Promise<string> {
    const { getVersion } = await import('@tauri-apps/api/app');
    return getVersion();
  }

  async isUpdateAvailable(): Promise<boolean> {
    const updateInfo = await this.checkForUpdates(true);
    return updateInfo !== null;
  }
}

export const updateManager = new UnifiedUpdateManager();
```

#### 4.3 WASM-Enhanced Autocomplete Service
**Timeline**: 1 week
**Effort**: High

**Tasks**:
- Create shared WASM autocomplete service
- Implement platform-agnostic WASM loading
- Create unified autocomplete interface
- Add cross-platform thesaurus management
- Implement streaming autocomplete

**Shared WASM Autocomplete**:
```typescript
// shared/frontend/src/lib/wasm-autocomplete-service.ts
export interface AutocompleteService {
  initialize(): Promise<void>;
  loadThesaurus(roleName: string): Promise<void>;
  getSuggestions(query: string, limit?: number): Promise<string[]>;
  getContextualSuggestions(query: string, context: string, limit?: number): Promise<AutocompleteSuggestion[]>;
  addCustomTerm(term: string, weight?: number): Promise<void>;
  isInitialized(): boolean;
}

export interface AutocompleteSuggestion {
  term: string;
  score: number;
  context?: string;
  source: 'thesaurus' | 'custom' | 'contextual';
}

class SharedWASMAutocompleteService implements AutocompleteService {
  private wasmModule: any = null;
  private autocompleteEngine: any = null;
  private initialized = false;
  private thesaurusCache = new Map<string, any>();
  private platformAPI: any;

  constructor() {
    this.platformAPI = this.createPlatformAPI();
  }

  private createPlatformAPI() {
    if (typeof window !== 'undefined' && 'electronAPI' in window) {
      return {
        loadThesaurus: (roleName: string) => window.electronAPI.loadThesaurus(roleName),
        storeData: (key: string, data: any) => window.electronAPI.storeData(key, data),
        loadData: (key: string) => window.electronAPI.loadData(key)
      };
    } else {
      return {
        loadThesaurus: async (roleName: string) => {
          const { invoke } = await import('@tauri-apps/api/tauri');
          return invoke('load_thesaurus', { roleName });
        },
        storeData: async (key: string, data: any) => {
          const { invoke } = await import('@tauri-apps/api/tauri');
          return invoke('store_data', { key, data });
        },
        loadData: async (key: string) => {
          const { invoke } = await import('@tauri-apps/api/tauri');
          return invoke('load_data', { key });
        }
      };
    }
  }

  async initialize(): Promise<void> {
    if (this.initialized) return;

    try {
      // Load WASM module
      this.wasmModule = await import('../wasm/pkg/terraphim_wasm.js');
      await this.wasmModule.default();

      this.autocompleteEngine = new this.wasmModule.AutocompleteEngine();
      this.initialized = true;

      console.log('Shared WASM Autocomplete service initialized');
    } catch (error) {
      console.error('Failed to initialize shared WASM autocomplete:', error);
      throw error;
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
      // Load thesaurus data via platform API
      const thesaurusData = await this.platformAPI.loadThesaurus(roleName);

      // Cache the data
      this.thesaurusCache.set(roleName, thesaurusData);

      // Load into WASM engine
      this.autocompleteEngine.load_thesaurus(JSON.stringify(thesaurusData));

      // Persist for offline use
      await this.platformAPI.storeData(`thesaurus_${roleName}`, thesaurusData);

      console.log(`Thesaurus loaded for role: ${roleName}`);
    } catch (error) {
      console.error('Failed to load thesaurus:', error);

      // Try to load from offline cache
      await this.loadThesaurusOffline(roleName);
    }
  }

  private async loadThesaurusOffline(roleName: string): Promise<void> {
    try {
      const cachedData = await this.platformAPI.loadData(`thesaurus_${roleName}`);

      if (cachedData) {
        this.autocompleteEngine.load_thesaurus(JSON.stringify(cachedData));
        console.log(`Loaded thesaurus from offline cache: ${roleName}`);
      }
    } catch (error) {
      console.error('Failed to load thesaurus from offline cache:', error);
    }
  }

  async getSuggestions(query: string, limit = 10): Promise<string[]> {
    if (!this.autocompleteEngine) {
      return [];
    }

    try {
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
    limit = 10
  ): Promise<AutocompleteSuggestion[]> {
    if (!this.autocompleteEngine) {
      return [];
    }

    try {
      const results = this.autocompleteEngine.get_contextual_suggestions(
        query,
        context,
        limit
      );

      const suggestions = JSON.parse(results);
      return suggestions.map((suggestion: any) => ({
        term: suggestion.term,
        score: suggestion.score,
        context: suggestion.context,
        source: suggestion.source || 'contextual'
      }));
    } catch (error) {
      console.error('Failed to get contextual suggestions:', error);
      return [];
    }
  }

  async addCustomTerm(term: string, weight = 1.0): Promise<void> {
    if (!this.autocompleteEngine) {
      return;
    }

    try {
      this.autocompleteEngine.add_custom_term(term, weight);

      // Persist custom terms
      const customTerms = await this.platformAPI.loadData('custom_terms') || [];
      customTerms.push({ term, weight, timestamp: Date.now() });
      await this.platformAPI.storeData('custom_terms', customTerms);
    } catch (error) {
      console.error('Failed to add custom term:', error);
    }
  }

  isInitialized(): boolean {
    return this.initialized && this.autocompleteEngine !== null;
  }

  // Streaming autocomplete for real-time suggestions
  async *streamSuggestions(query: string, limit = 10): AsyncGenerator<string[]> {
    if (!this.autocompleteEngine) {
      return;
    }

    try {
      const stream = this.autocompleteEngine.stream_suggestions(query, limit);

      for (const chunk of stream) {
        yield JSON.parse(chunk);
      }
    } catch (error) {
      console.error('Failed to stream suggestions:', error);
    }
  }

  // Clear cache and reload
  async clearCache(): Promise<void> {
    this.thesaurusCache.clear();

    if (this.autocompleteEngine) {
      this.autocompleteEngine.clear_cache();
    }
  }

  // Get statistics
  getStats(): AutocompleteStats {
    if (!this.autocompleteEngine) {
      return { totalTerms: 0, cacheSize: 0, customTerms: 0 };
    }

    try {
      const stats = this.autocompleteEngine.get_stats();
      return JSON.parse(stats);
    } catch (error) {
      console.error('Failed to get autocomplete stats:', error);
      return { totalTerms: 0, cacheSize: 0, customTerms: 0 };
    }
  }
}

export interface AutocompleteStats {
  totalTerms: number;
  cacheSize: number;
  customTerms: number;
}

export const sharedAutocompleteService = new SharedWASMAutocompleteService();
```

#### 4.4 Backend Service Abstraction
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Create shared backend services
- Implement platform-agnostic interfaces
- Create service registry and dependency injection
- Implement data access layer abstraction

**Shared Service Interface**:
```rust
// shared/backend/src/services/mod.rs
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;

#[async_trait]
pub trait SearchService: Send + Sync {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Value>, Box<dyn Error>>;
    async fn get_suggestions(&self, query: &str) -> Result<Vec<String>, Box<dyn Error>>;
}

#[async_trait]
pub trait ChatService: Send + Sync {
    async fn send_message(&self, message: &str, conversation_id: Option<&str>) -> Result<Value, Box<dyn Error>>;
    async fn get_conversation(&self, id: &str) -> Result<Value, Box<dyn Error>>;
    async fn list_conversations(&self) -> Result<Vec<Value>, Box<dyn Error>>;
}

#[async_trait]
pub trait ConfigService: Send + Sync {
    async fn get_config(&self) -> Result<Value, Box<dyn Error>>;
    async fn update_config(&self, config: Value) -> Result<Value, Box<dyn Error>>;
    async fn reset_config(&self) -> Result<Value, Box<dyn Error>>;
}

// Service registry
pub struct ServiceRegistry {
    pub search_service: Arc<dyn SearchService>,
    pub chat_service: Arc<dyn ChatService>,
    pub config_service: Arc<dyn ConfigService>,
}

impl ServiceRegistry {
    pub fn new(
        search_service: Arc<dyn SearchService>,
        chat_service: Arc<dyn ChatService>,
        config_service: Arc<dyn ConfigService>,
    ) -> Self {
        Self {
            search_service,
            chat_service,
            config_service,
        }
    }
}
```

#### 4.2 Frontend State Management
**Timeline**: 1-2 weeks
**Effort**: Medium

**Tasks**:
- Create shared state management system
- Implement platform-agnostic stores
- Create reactive data binding
- Implement error handling and loading states

**Shared State Management**:
```typescript
// shared/frontend/src/stores/app.ts
import { writable, derived, type Writable } from 'svelte/store';
import { platformAPI } from '$lib/platform';

interface AppState {
  user: User | null;
  config: Config | null;
  isLoading: boolean;
  error: string | null;
}

interface SearchState {
  query: string;
  results: SearchResult[];
  suggestions: string[];
  isSearching: boolean;
}

interface ChatState {
  conversations: Conversation[];
  currentConversation: Conversation | null;
  isLoading: boolean;
}

// App state
export const appStore: Writable<AppState> = writable({
  user: null,
  config: null,
  isLoading: false,
  error: null,
});

// Search state
export const searchStore: Writable<SearchState> = writable({
  query: '',
  results: [],
  suggestions: [],
  isSearching: false,
});

// Chat state
export const chatStore: Writable<ChatState> = writable({
  conversations: [],
  currentConversation: null,
  isLoading: false,
});

// Derived stores
export const isAppReady = derived(
  appStore,
  ($appStore) => $appStore.config !== null && !$appStore.isLoading
);

export const hasActiveConversation = derived(
  chatStore,
  ($chatStore) => $chatStore.currentConversation !== null
);

// Actions
export const appActions = {
  async loadConfig() {
    appStore.update(state => ({ ...state, isLoading: true, error: null }));

    try {
      const config = await platformAPI.getConfig();
      appStore.update(state => ({
        ...state,
        config,
        isLoading: false
      }));
    } catch (error) {
      appStore.update(state => ({
        ...state,
        error: error.message,
        isLoading: false
      }));
    }
  },

  async updateConfig(config: Config) {
    appStore.update(state => ({ ...state, isLoading: true, error: null }));

    try {
      const updatedConfig = await platformAPI.updateConfig(config);
      appStore.update(state => ({
        ...state,
        config: updatedConfig,
        isLoading: false
      }));
    } catch (error) {
      appStore.update(state => ({
        ...state,
        error: error.message,
        isLoading: false
      }));
    }
  },
};

export const searchActions = {
  async search(query: string) {
    searchStore.update(state => ({
      ...state,
      query,
      isSearching: true
    }));

    try {
      const results = await platformAPI.search(query, 20);
      searchStore.update(state => ({
        ...state,
        results,
        isSearching: false
      }));
    } catch (error) {
      searchStore.update(state => ({
        ...state,
        error: error.message,
        isSearching: false
      }));
    }
  },

  async getSuggestions(query: string) {
    try {
      const suggestions = await platformAPI.getSearchSuggestions(query);
      searchStore.update(state => ({ ...state, suggestions }));
    } catch (error) {
      console.error('Failed to get suggestions:', error);
    }
  },
};
```

### Phase 5: Build System & Deployment (2-3 weeks)

#### 5.1 Unified Build System
**Timeline**: 1-2 weeks
**Effort**: High

**Tasks**:
- Create unified build scripts
- Set up monorepo structure
- Configure shared dependency management
- Implement automated testing pipeline

**Build Configuration**:
```javascript
// build-tools/webpack.common.js
const path = require('path');
const { DefinePlugin } = require('webpack');

module.exports = {
  entry: './shared/frontend/src/main.ts',
  output: {
    path: path.resolve(__dirname, '../shared/frontend/dist'),
    filename: 'bundle.js',
    clean: true,
  },
  resolve: {
    extensions: ['.ts', '.js', '.svelte'],
    alias: {
      '@': path.resolve(__dirname, '../shared/frontend/src'),
      '@shared': path.resolve(__dirname, '../shared'),
    },
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.svelte$/,
        use: 'svelte-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.css$/,
        use: ['style-loader', 'css-loader'],
      },
    ],
  },
  plugins: [
    new DefinePlugin({
      'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV),
      'process.env.PLATFORM': JSON.stringify(process.env.PLATFORM),
    }),
  ],
};
```

#### 5.2 CI/CD Pipeline
**Timeline**: 1 week
**Effort**: Medium

**Tasks**:
- Set up GitHub Actions for both platforms
- Configure automated testing
- Set up artifact management
- Configure deployment pipelines

**GitHub Actions Workflow**:
```yaml
# .github/workflows/build-hybrid.yml
name: Build Hybrid Applications

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test-shared:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install dependencies
        run: |
          cd shared/frontend && npm ci
          cd ../backend && cargo fetch

      - name: Run frontend tests
        run: cd shared/frontend && npm test

      - name: Run backend tests
        run: cd shared/backend && cargo test

  build-electron:
    needs: test-shared
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: |
          cd shared/frontend && npm ci
          cd ../../electron-app && npm ci

      - name: Build Electron app
        run: cd electron-app && npm run build

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: electron-${{ matrix.os }}
          path: electron-app/dist/

  build-tauri:
    needs: test-shared
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install dependencies
        run: |
          cd shared/frontend && npm ci
          cd ../../tauri-app && npm ci

      - name: Build Tauri app
        run: cd tauri-app && npm run tauri:build

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: tauri-${{ matrix.os }}
          path: tauri-app/src-tauri/target/release/bundle/
```

## Technical Specifications

### Hybrid Architecture Benefits

**Code Sharing**:
- 80%+ shared frontend code
- 90%+ shared backend logic
- Unified state management
- Consistent user experience

**Platform Optimization**:
- Electron: Maximum compatibility, extensive ecosystem
- Tauri: Performance, security, smaller bundle size
- Gradual migration path
- Risk mitigation

**Development Efficiency**:
- Single team, multiple platforms
- Shared testing infrastructure
- Unified build pipeline
- Consistent deployment process

### Performance Comparison

| Metric | Electron | Tauri | Hybrid |
|--------|----------|-------|--------|
| Startup Time | 3-5s | 1-2s | 1-3s |
| Memory Usage | 200-400MB | 100-200MB | 150-300MB |
| Bundle Size | 100-200MB | 25-50MB | 50-100MB |
| CPU Usage | Medium | Low | Low-Medium |

### Feature Matrix

| Feature | Electron | Tauri | Hybrid |
|---------|----------|-------|--------|
| Cross-platform | ✅ | ✅ | ✅ |
| Native APIs | ✅ | ✅ | ✅ |
| Web Technologies | ✅ | ✅ | ✅ |
| Auto-updater | ✅ | ✅ | ✅ |
| System Tray | ✅ | ✅ | ✅ |
| File System | ✅ | ✅ | ✅ |
| Notifications | ✅ | ✅ | ✅ |
| Native Menus | ✅ | ✅ | ✅ |

## Benefits of Hybrid Approach

### 1. Risk Mitigation
- Gradual migration reduces risk
- Platform fallback options
- Parallel development tracks
- Feature parity validation

### 2. Performance Optimization
- Use Tauri for performance-critical features
- Electron for compatibility requirements
- Optimized bundle sizes
- Platform-specific optimizations

### 3. Development Flexibility
- Team can work on both platforms
- Shared knowledge and expertise
- Unified development practices
- Cross-platform skill development

### 4. User Choice
- Users can choose preferred platform
- A/B testing capabilities
- Gradual user migration
- Platform-specific features

### 5. Future-Proofing
- Easy to add new platforms
- Technology stack flexibility
- Migration path to web/mobile
- Long-term maintainability

## Challenges & Considerations

### Technical Challenges

**Build Complexity**:
- Multiple build configurations
- Platform-specific dependencies
- Shared code management
- Testing complexity

**Code Synchronization**:
- Keeping platforms in sync
- API consistency
- Feature parity maintenance
- Version management

**Performance Overhead**:
- Abstraction layer complexity
- Platform detection overhead
- Bundle size considerations
- Runtime performance

### Resource Challenges

**Development Overhead**:
- Maintaining two platforms
- Increased testing requirements
- Complex build pipeline
- Documentation maintenance

**Team Coordination**:
- Cross-platform expertise required
- Communication overhead
- Knowledge sharing
- Skill development

## Migration Strategy

### Phase 1: Foundation (Weeks 1-3)
- Set up hybrid architecture
- Create shared component library
- Implement platform abstraction

### Phase 2: Electron Implementation (Weeks 4-8)
- Build Electron version
- Implement platform-specific features
- Create testing infrastructure

### Phase 3: Tauri Integration (Weeks 9-12)
- Migrate to Tauri architecture
- Implement shared services
- Optimize performance

### Phase 4: Integration & Testing (Weeks 13-16)
- Unify build systems
- Comprehensive testing
- Performance optimization

### Phase 5: Deployment & Rollout (Weeks 17-20)
- Set up CI/CD pipeline
- Gradual user rollout
- Monitor and optimize

## Success Metrics

### Technical Metrics
- 80%+ code sharing between platforms
- <3s startup time for both platforms
- <200MB memory usage
- 95%+ test coverage

### User Metrics
- 4.5+ star rating for both platforms
- <5% crash rate
- 85%+ user retention
- Positive platform preference feedback

### Development Metrics
- 30% reduction in development time (vs separate apps)
- 90%+ feature parity
- <24h bug fix turnaround
- Comprehensive documentation coverage

## Resource Requirements

### Development Team
- **Frontend Developer**: 1 (full-time for 20 weeks)
- **Rust Developer**: 1 (full-time for 16 weeks)
- **Electron Specialist**: 1 (part-time for 8 weeks)
- **DevOps Engineer**: 1 (part-time for 6 weeks)
- **QA Engineer**: 1 (part-time for 12 weeks)

### Infrastructure Costs
- Build servers: $100-200/month
- CI/CD pipeline: $50-100/month
- Testing infrastructure: $50-100/month
- Distribution platforms: $100-300/month

## Timeline Summary

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Phase 1: Architecture Design | 2-3 weeks | Week 1 | Week 3 |
| Phase 2: Electron Implementation | 4-5 weeks | Week 3 | Week 8 |
| Phase 3: Tauri Integration | 3-4 weeks | Week 8 | Week 12 |
| Phase 4: Shared Services | 3-4 weeks | Week 10 | Week 14 |
| Phase 5: Build & Deployment | 2-3 weeks | Week 14 | Week 17 |

**Total Duration**: 17-20 weeks (4-5 months)

This hybrid approach provides the best of both worlds, allowing for maximum flexibility, risk mitigation, and gradual migration while maintaining high performance and user satisfaction across both platforms.
