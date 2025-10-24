/**
 * Theme Toggle Component
 * Switches between light and dark themes
 */

class ThemeToggle extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.currentTheme = this.getStoredTheme() || this.getPreferredTheme();
  }

  connectedCallback() {
    this.render();
    this.applyTheme(this.currentTheme);
    this.setupEventListeners();
  }

  /**
   * Get theme from localStorage
   * @returns {string|null} Stored theme or null
   */
  getStoredTheme() {
    return localStorage.getItem('terraphim-theme');
  }

  /**
   * Get user's preferred theme from system
   * @returns {string} 'dark' or 'light'
   */
  getPreferredTheme() {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'dark';
    }
    return 'light';
  }

  /**
   * Apply theme to document
   * @param {string} theme - Theme name ('light' or 'dark')
   */
  applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('terraphim-theme', theme);
    this.currentTheme = theme;
    this.updateButton();
    this.dispatchThemeChange(theme);
  }

  /**
   * Toggle between light and dark themes
   */
  toggleTheme() {
    const newTheme = this.currentTheme === 'light' ? 'dark' : 'light';
    this.applyTheme(newTheme);
  }

  /**
   * Update button appearance based on current theme
   */
  updateButton() {
    const button = this.shadowRoot.querySelector('button');
    if (!button) return;

    const icon = this.currentTheme === 'light' ? '🌙' : '☀️';
    const label = this.currentTheme === 'light' ? 'Dark mode' : 'Light mode';

    button.setAttribute('aria-label', label);
    button.querySelector('.icon').textContent = icon;
  }

  /**
   * Dispatch theme change event
   * @param {string} theme - New theme name
   */
  dispatchThemeChange(theme) {
    this.dispatchEvent(new CustomEvent('theme-changed', {
      detail: { theme },
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Setup event listeners
   */
  setupEventListeners() {
    const button = this.shadowRoot.querySelector('button');
    button.addEventListener('click', () => this.toggleTheme());

    // Listen for system theme changes
    if (window.matchMedia) {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!this.getStoredTheme()) {
          const theme = e.matches ? 'dark' : 'light';
          this.applyTheme(theme);
        }
      });
    }
  }

  /**
   * Render component template
   */
  render() {
    const icon = this.currentTheme === 'light' ? '🌙' : '☀️';
    const label = this.currentTheme === 'light' ? 'Dark mode' : 'Light mode';

    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: inline-block;
        }

        button {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 0.75rem;
          background: transparent;
          border: 1px solid var(--color-border, #e1e4e8);
          border-radius: 8px;
          color: var(--color-text-primary, #2c3e50);
          font-family: var(--font-family-base, system-ui, sans-serif);
          font-size: 0.875rem;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.15s ease-in-out;
        }

        button:hover {
          background: var(--color-surface, #f8f9fa);
          border-color: var(--color-accent, #3498db);
        }

        button:focus-visible {
          outline: 2px solid var(--color-border-focus, #3498db);
          outline-offset: 2px;
        }

        button:active {
          transform: scale(0.98);
        }

        .icon {
          font-size: 1.125rem;
          line-height: 1;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .label {
          display: none;
        }

        @media (min-width: 640px) {
          .label {
            display: inline;
          }
        }
      </style>

      <button type="button" aria-label="${label}">
        <span class="icon">${icon}</span>
        <span class="label">${label}</span>
      </button>
    `;
  }
}

customElements.define('theme-toggle', ThemeToggle);
