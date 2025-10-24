/**
 * @fileoverview TerraphimThemeSwitcher - Light/dark theme toggle
 * Persists theme preference to localStorage
 */

import { TerraphimElement } from '../base/terraphim-element.js';
import { galleryState } from './terraphim-gallery.js';

/**
 * TerraphimThemeSwitcher Component
 * Toggle between light and dark themes
 */
export class TerraphimThemeSwitcher extends TerraphimElement {
  static get properties() {
    return {
      theme: { type: String, default: 'light' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    this.bindState(galleryState, 'theme', 'theme', { immediate: true });

    // Apply theme to document
    this.bindState(galleryState, 'theme', (theme) => {
      document.documentElement.setAttribute('data-theme', theme);
    }, { immediate: true });
  }

  toggleTheme() {
    const newTheme = this.theme === 'light' ? 'dark' : 'light';
    galleryState.set('theme', newTheme);
  }

  render() {
    const isDark = this.theme === 'dark';

    this.setHTML(this.shadowRoot, `
      <style>
        :host {
          display: inline-block;
        }

        .theme-switcher {
          position: relative;
          display: inline-flex;
          align-items: center;
          gap: 8px;
        }

        .toggle-button {
          position: relative;
          width: 48px;
          height: 24px;
          background: ${isDark ? '#3498db' : '#bdc3c7'};
          border: none;
          border-radius: 12px;
          cursor: pointer;
          transition: background 0.3s;
          padding: 0;
        }

        .toggle-button:hover {
          background: ${isDark ? '#2980b9' : '#95a5a6'};
        }

        .toggle-button:focus {
          outline: 2px solid var(--color-primary, #3498db);
          outline-offset: 2px;
        }

        .toggle-slider {
          position: absolute;
          top: 2px;
          left: ${isDark ? '26px' : '2px'};
          width: 20px;
          height: 20px;
          background: white;
          border-radius: 50%;
          transition: left 0.3s;
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 12px;
        }

        .theme-label {
          font-size: 14px;
          color: var(--color-text, #333);
          user-select: none;
        }

        .theme-icon {
          font-size: 16px;
        }
      </style>

      <div class="theme-switcher">
        <span class="theme-icon">${isDark ? '🌙' : '☀️'}</span>
        <button
          class="toggle-button"
          id="toggleBtn"
          role="switch"
          aria-checked="${isDark}"
          aria-label="Toggle theme"
          title="${isDark ? 'Switch to light mode' : 'Switch to dark mode'}"
        >
          <span class="toggle-slider">
            ${isDark ? '🌙' : '☀️'}
          </span>
        </button>
        <span class="theme-label">${isDark ? 'Dark' : 'Light'}</span>
      </div>
    `);

    // Add event listener
    const toggleBtn = this.$('#toggleBtn');
    if (toggleBtn) {
      toggleBtn.addEventListener('click', () => this.toggleTheme());
    }
  }
}

customElements.define('terraphim-theme-switcher', TerraphimThemeSwitcher);
