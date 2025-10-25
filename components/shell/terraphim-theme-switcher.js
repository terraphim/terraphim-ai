/**
 * @fileoverview TerraphimThemeSwitcher - Theme switcher component with Bulmaswatch support
 * Provides dropdown interface for switching between 22 Bulmaswatch themes
 * Includes localStorage persistence and system preference detection
 */

import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * Theme metadata for all 22 Bulmaswatch themes
 * Organized by type (light/dark) with descriptions
 */
const THEMES = [
  // Light themes
  { name: 'cerulean', type: 'light', label: 'Cerulean', description: 'Calm blue theme' },
  { name: 'cosmo', type: 'light', label: 'Cosmo', description: 'Modern flat design' },
  { name: 'default', type: 'light', label: 'Default', description: 'Bulma default theme' },
  { name: 'flatly', type: 'light', label: 'Flatly', description: 'Flat and modern' },
  { name: 'journal', type: 'light', label: 'Journal', description: 'Newspaper style' },
  { name: 'litera', type: 'light', label: 'Litera', description: 'Clean serif theme' },
  { name: 'lumen', type: 'light', label: 'Lumen', description: 'Light and airy' },
  { name: 'lux', type: 'light', label: 'Lux', description: 'Luxurious gold accents' },
  { name: 'materia', type: 'light', label: 'Materia', description: 'Material design inspired' },
  { name: 'minty', type: 'light', label: 'Minty', description: 'Fresh green theme' },
  { name: 'pulse', type: 'light', label: 'Pulse', description: 'Vibrant purple accents' },
  { name: 'sandstone', type: 'light', label: 'Sandstone', description: 'Warm earthy tones' },
  { name: 'simplex', type: 'light', label: 'Simplex', description: 'Simple and clean' },
  { name: 'spacelab', type: 'light', label: 'Spacelab', description: 'Default theme' },
  { name: 'united', type: 'light', label: 'United', description: 'Bold and unified' },
  { name: 'yeti', type: 'light', label: 'Yeti', description: 'Cool blue-gray theme' },

  // Dark themes
  { name: 'cyborg', type: 'dark', label: 'Cyborg', description: 'Dark blue tech theme' },
  { name: 'darkly', type: 'dark', label: 'Darkly', description: 'Popular dark theme' },
  { name: 'nuclear', type: 'dark', label: 'Nuclear', description: 'Dark green accents' },
  { name: 'slate', type: 'dark', label: 'Slate', description: 'Dark purple theme' },
  { name: 'solar', type: 'dark', label: 'Solar', description: 'Solarized dark' },
  { name: 'superhero', type: 'dark', label: 'Superhero', description: 'Dark with red accents' }
];

/**
 * ThemeManager - Handles theme loading and persistence
 * Embedded in the component for zero external dependencies
 */
class ThemeManager {
  /**
   * Create a new ThemeManager instance
   * @param {Object} options - Configuration options
   * @param {string} options.storageKey - localStorage key for persistence
   * @param {string} options.assetPath - Base path for Bulmaswatch CSS files
   */
  constructor(options = {}) {
    this.storageKey = options.storageKey || 'terraphim-theme';
    this.assetPath = options.assetPath || '/assets/bulmaswatch';
    this.currentLink = null;
    this.loadCallbacks = new Set();
    this.errorCallbacks = new Set();
  }

  /**
   * Apply a theme by name
   * Sets data-theme attribute and loads Bulmaswatch CSS
   * @param {string} themeName - Name of theme to apply
   * @returns {Promise<void>}
   */
  async applyTheme(themeName) {
    const startTime = performance.now();

    // Set data-theme attribute for CSS custom properties
    document.documentElement.setAttribute('data-theme', themeName);

    // Load Bulmaswatch CSS
    return new Promise((resolve, reject) => {
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = `${this.assetPath}/${themeName}/bulmaswatch.min.css`;

      link.onload = () => {
        // Remove previous theme stylesheet
        if (this.currentLink && this.currentLink !== link) {
          this.currentLink.remove();
        }
        this.currentLink = link;

        const loadTime = performance.now() - startTime;

        // Notify load callbacks
        this.loadCallbacks.forEach(callback => {
          try {
            callback({ theme: themeName, loadTime });
          } catch (error) {
            console.error('Error in theme load callback:', error);
          }
        });

        resolve();
      };

      link.onerror = () => {
        const error = new Error(`Failed to load theme: ${themeName}`);

        // Notify error callbacks
        this.errorCallbacks.forEach(callback => {
          try {
            callback({ theme: themeName, error });
          } catch (err) {
            console.error('Error in theme error callback:', err);
          }
        });

        reject(error);
      };

      document.head.appendChild(link);
    });
  }

  /**
   * Save theme name to localStorage
   * @param {string} themeName - Theme name to save
   */
  saveTheme(themeName) {
    try {
      localStorage.setItem(this.storageKey, themeName);
    } catch (error) {
      console.error('Failed to save theme to localStorage:', error);
    }
  }

  /**
   * Load saved theme from localStorage
   * @returns {string|null} Saved theme name or null
   */
  loadTheme() {
    try {
      return localStorage.getItem(this.storageKey);
    } catch (error) {
      console.error('Failed to load theme from localStorage:', error);
      return null;
    }
  }

  /**
   * Detect system color scheme preference
   * @returns {string} 'dark' or 'light'
   */
  detectSystemPreference() {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'dark';
    }
    return 'light';
  }

  /**
   * Register a callback for successful theme loads
   * @param {Function} callback - Callback function
   * @returns {Function} Unregister function
   */
  onLoad(callback) {
    this.loadCallbacks.add(callback);
    return () => this.loadCallbacks.delete(callback);
  }

  /**
   * Register a callback for theme load errors
   * @param {Function} callback - Callback function
   * @returns {Function} Unregister function
   */
  onError(callback) {
    this.errorCallbacks.add(callback);
    return () => this.errorCallbacks.delete(callback);
  }
}

/**
 * TerraphimThemeSwitcher - Theme selector component
 *
 * @fires theme-changed - Emitted when theme changes: { oldTheme, newTheme, isDark }
 * @fires theme-loaded - Emitted when CSS loads: { theme, loadTime }
 * @fires theme-error - Emitted on load failure: { theme, error }
 *
 * @example
 * <terraphim-theme-switcher
 *   current-theme="spacelab"
 *   show-label="true"
 *   storage-key="my-theme">
 * </terraphim-theme-switcher>
 *
 * @example
 * const switcher = document.querySelector('terraphim-theme-switcher');
 * switcher.addEventListener('theme-changed', (e) => {
 *   console.log('Theme changed:', e.detail.newTheme);
 * });
 */
export class TerraphimThemeSwitcher extends TerraphimElement {
  static get observedAttributes() {
    return ['current-theme', 'show-label', 'storage-key'];
  }

  static get properties() {
    return {
      currentTheme: { type: String, reflect: true, default: 'spacelab' },
      showLabel: { type: Boolean, reflect: true, default: true },
      storageKey: { type: String, reflect: true, default: 'terraphim-theme' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._themeManager = null;
    this._initializing = false;
  }

  /**
   * Lifecycle: Connected to DOM
   * Initialize theme manager and load theme
   */
  onConnected() {
    // Initialize ThemeManager
    this._themeManager = new ThemeManager({
      storageKey: this.storageKey,
      assetPath: '/assets/bulmaswatch'
    });

    // Register callbacks
    this._themeManager.onLoad((detail) => {
      this.emit('theme-loaded', detail);
    });

    this._themeManager.onError((detail) => {
      this.emit('theme-error', detail);
    });

    // Load initial theme
    this._initializeTheme();
  }

  /**
   * Initialize theme from storage or system preference
   * @private
   */
  async _initializeTheme() {
    if (this._initializing) return;
    this._initializing = true;

    try {
      // 1. Try to load from localStorage
      const savedTheme = this._themeManager.loadTheme();
      if (savedTheme && this._isValidTheme(savedTheme)) {
        await this.switchTheme(savedTheme);
        this._initializing = false;
        return;
      }

      // 2. Try to use current-theme attribute
      if (this.currentTheme && this._isValidTheme(this.currentTheme)) {
        await this.switchTheme(this.currentTheme);
        this._initializing = false;
        return;
      }

      // 3. Detect system preference and pick matching theme
      const systemPreference = this._themeManager.detectSystemPreference();
      const defaultTheme = systemPreference === 'dark' ? 'darkly' : 'spacelab';
      await this.switchTheme(defaultTheme);
    } catch (error) {
      console.error('Failed to initialize theme:', error);
    } finally {
      this._initializing = false;
    }
  }

  /**
   * Check if theme name is valid
   * @private
   * @param {string} themeName - Theme name to validate
   * @returns {boolean}
   */
  _isValidTheme(themeName) {
    return THEMES.some(theme => theme.name === themeName);
  }

  /**
   * Switch to a new theme
   * @param {string} themeName - Name of theme to switch to
   * @returns {Promise<void>}
   */
  async switchTheme(themeName) {
    if (!this._isValidTheme(themeName)) {
      console.warn(`Invalid theme name: ${themeName}`);
      return;
    }

    const oldTheme = this.currentTheme;
    if (oldTheme === themeName) {
      return; // Already on this theme
    }

    try {
      // Apply theme via ThemeManager
      await this._themeManager.applyTheme(themeName);

      // Update component state
      this.currentTheme = themeName;

      // Save to localStorage
      this._themeManager.saveTheme(themeName);

      // Emit theme-changed event
      const themeInfo = THEMES.find(t => t.name === themeName);
      this.emit('theme-changed', {
        oldTheme,
        newTheme: themeName,
        isDark: themeInfo?.type === 'dark'
      });
    } catch (error) {
      console.error('Failed to switch theme:', error);
      throw error;
    }
  }

  /**
   * Get current theme name
   * @returns {string}
   */
  getCurrentTheme() {
    return this.currentTheme;
  }

  /**
   * Get all available themes
   * @returns {Array<Object>} Array of theme metadata
   */
  getThemes() {
    return [...THEMES];
  }

  /**
   * Detect system color scheme preference
   * @returns {string} 'dark' or 'light'
   */
  detectSystemTheme() {
    return this._themeManager.detectSystemPreference();
  }

  /**
   * Handle dropdown selection change
   * @private
   * @param {Event} event - Change event
   */
  _handleThemeChange(event) {
    const themeName = event.target.value;
    this.switchTheme(themeName);
  }

  /**
   * Render the component
   */
  render() {
    const lightThemes = THEMES.filter(t => t.type === 'light');
    const darkThemes = THEMES.filter(t => t.type === 'dark');

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: inline-block;
        }

        .theme-switcher {
          display: flex;
          align-items: center;
          gap: var(--spacing-sm, 0.5rem);
        }

        .theme-label {
          font-size: var(--font-size-sm, 0.875rem);
          color: var(--text-secondary, #4a4a4a);
          font-weight: var(--font-weight-medium, 500);
        }

        .theme-select {
          min-width: 160px;
          padding: var(--spacing-xs, 0.25rem) var(--spacing-sm, 0.5rem);
          font-size: var(--font-size-sm, 0.875rem);
          border: 1px solid var(--border-primary, #dbdbdb);
          border-radius: var(--border-radius-md, 4px);
          background-color: var(--bg-elevated, #ffffff);
          color: var(--text-primary, #363636);
          cursor: pointer;
          transition: var(--transition-base, 200ms ease);
        }

        .theme-select:hover {
          border-color: var(--border-hover, #b5b5b5);
        }

        .theme-select:focus {
          outline: none;
          border-color: var(--border-focus, #3273dc);
          box-shadow: var(--shadow-focus, 0 0 0 3px rgba(50, 115, 220, 0.25));
        }

        .theme-select optgroup {
          font-weight: var(--font-weight-semibold, 600);
        }

        .theme-select option {
          padding: var(--spacing-xs, 0.25rem);
        }

        /* Dark mode support */
        @media (prefers-color-scheme: dark) {
          .theme-select {
            background-color: var(--bg-elevated, #333333);
            color: var(--text-primary, #f5f5f5);
          }
        }
      </style>

      <div class="theme-switcher">
        ${this.showLabel ? '<span class="theme-label">Theme:</span>' : ''}
        <select class="theme-select" aria-label="Select theme">
          <optgroup label="Light Themes">
            ${lightThemes.map(theme => `
              <option
                value="${theme.name}"
                ${theme.name === this.currentTheme ? 'selected' : ''}
                title="${theme.description}">
                ${theme.label}
              </option>
            `).join('')}
          </optgroup>
          <optgroup label="Dark Themes">
            ${darkThemes.map(theme => `
              <option
                value="${theme.name}"
                ${theme.name === this.currentTheme ? 'selected' : ''}
                title="${theme.description}">
                ${theme.label}
              </option>
            `).join('')}
          </optgroup>
        </select>
      </div>
    `);

    // Attach event listener
    const select = this.$('.theme-select');
    if (select) {
      this.listenTo(select, 'change', this._handleThemeChange.bind(this));
    }
  }

  /**
   * Property changed callback - re-render on property changes
   */
  propertyChangedCallback(name, oldValue, newValue) {
    super.propertyChangedCallback(name, oldValue, newValue);

    // If storageKey changed, update ThemeManager
    if (name === 'storageKey' && this._themeManager) {
      this._themeManager.storageKey = newValue;
    }
  }
}

// Register the custom element
customElements.define('terraphim-theme-switcher', TerraphimThemeSwitcher);

// Export THEMES metadata for use in other components
export { THEMES };
