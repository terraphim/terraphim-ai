/**
 * Terraphim Gallery Component
 * Root application component that orchestrates the entire gallery
 */

import { router } from '../../docs/gallery/scripts/router.js';
import { search } from '../../docs/gallery/scripts/search.js';

class TerraphimGallery extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.mobileMenuOpen = false;
  }

  connectedCallback() {
    this.render();
    this.setupRouter();
    this.setupSearch();
    this.setupEventListeners();
    this.loadComponents();
  }

  /**
   * Setup router
   */
  setupRouter() {
    // Listen for route changes
    router.addListener((currentPath) => {
      this.handleRouteChange(currentPath);
    });

    // Handle initial route
    const currentPath = router.getCurrentPath();
    this.handleRouteChange(currentPath);
  }

  /**
   * Setup search
   */
  async setupSearch() {
    try {
      const response = await fetch('./data/components.json');
      const data = await response.json();
      search.initialize(data.components || []);
    } catch (error) {
      console.error('Failed to load component data for search:', error);
    }
  }

  /**
   * Load component data
   */
  async loadComponents() {
    try {
      const response = await fetch('./data/components.json');
      const data = await response.json();
      this.dispatchEvent(new CustomEvent('components-loaded', {
        detail: { components: data.components, categories: data.categories },
        bubbles: true,
        composed: true
      }));
    } catch (error) {
      console.error('Failed to load components:', error);
    }
  }

  /**
   * Setup event listeners
   */
  setupEventListeners() {
    // Navigation events
    this.addEventListener('navigate', (e) => {
      const path = e.detail.path;
      router.navigate(path);
      this.closeMobileMenu();
    });

    // Menu toggle (mobile)
    this.addEventListener('menu-toggle', () => {
      this.toggleMobileMenu();
    });

    // Search events
    this.addEventListener('search', (e) => {
      this.handleSearch(e.detail.query);
    });

    // Close mobile menu when clicking overlay
    const overlay = this.shadowRoot.querySelector('.sidebar-overlay');
    if (overlay) {
      overlay.addEventListener('click', () => {
        this.closeMobileMenu();
      });
    }

    // Close mobile menu on escape key
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.mobileMenuOpen) {
        this.closeMobileMenu();
      }
    });
  }

  /**
   * Handle route change
   * @param {string} path - New route path
   */
  handleRouteChange(path) {
    // Update sidebar active state
    const sidebar = this.shadowRoot.querySelector('gallery-sidebar');
    if (sidebar) {
      sidebar.updateActivePath(path);
    }

    // Update main content
    const main = this.shadowRoot.querySelector('gallery-main');
    if (main) {
      main.updateContent(path);
    }

    // Update page title
    this.updatePageTitle(path);

    // Scroll to top
    const mainElement = this.shadowRoot.querySelector('gallery-main');
    if (mainElement) {
      mainElement.scrollTop = 0;
    }
  }

  /**
   * Update page title based on route
   * @param {string} path - Current route path
   */
  updatePageTitle(path) {
    const segments = path.split('/').filter(s => s);
    if (segments.length === 0) {
      document.title = 'Terraphim Gallery';
    } else {
      const title = segments[segments.length - 1]
        .split('-')
        .map(word => word.charAt(0).toUpperCase() + word.slice(1))
        .join(' ');
      document.title = `${title} - Terraphim Gallery`;
    }
  }

  /**
   * Handle search
   * @param {string} query - Search query
   */
  handleSearch(query) {
    const results = search.search(query);
    console.log('Search results:', results);

    // TODO Phase 2: Display search results in UI
    // For now, just log to console
  }

  /**
   * Toggle mobile menu
   */
  toggleMobileMenu() {
    this.mobileMenuOpen = !this.mobileMenuOpen;
    this.updateMobileMenuState();
  }

  /**
   * Open mobile menu
   */
  openMobileMenu() {
    this.mobileMenuOpen = true;
    this.updateMobileMenuState();
  }

  /**
   * Close mobile menu
   */
  closeMobileMenu() {
    this.mobileMenuOpen = false;
    this.updateMobileMenuState();
  }

  /**
   * Update mobile menu state in DOM
   */
  updateMobileMenuState() {
    const sidebar = this.shadowRoot.querySelector('gallery-sidebar');
    const overlay = this.shadowRoot.querySelector('.sidebar-overlay');
    const menuToggle = this.shadowRoot.querySelector('gallery-header')?.shadowRoot.querySelector('.menu-toggle');

    if (sidebar) {
      if (this.mobileMenuOpen) {
        sidebar.setAttribute('data-open', 'true');
      } else {
        sidebar.removeAttribute('data-open');
      }
    }

    if (overlay) {
      if (this.mobileMenuOpen) {
        overlay.setAttribute('data-visible', 'true');
      } else {
        overlay.removeAttribute('data-visible');
      }
    }

    if (menuToggle) {
      menuToggle.setAttribute('aria-expanded', this.mobileMenuOpen.toString());
    }

    // Prevent body scroll when menu is open on mobile
    if (window.innerWidth < 768) {
      document.body.style.overflow = this.mobileMenuOpen ? 'hidden' : '';
    }
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          position: relative;
          width: 100%;
          height: 100vh;
          overflow: hidden;
        }

        .gallery-container {
          display: grid;
          grid-template-columns: 1fr;
          grid-template-rows: var(--header-height, 64px) 1fr;
          width: 100%;
          height: 100%;
        }

        @media (min-width: 768px) {
          .gallery-container {
            grid-template-columns: var(--sidebar-width, 280px) 1fr;
          }
        }

        gallery-header {
          grid-column: 1 / -1;
          grid-row: 1;
        }

        gallery-sidebar {
          grid-column: 1;
          grid-row: 2;
        }

        @media (min-width: 768px) {
          gallery-sidebar {
            grid-column: 1;
            grid-row: 2;
          }
        }

        gallery-main {
          grid-column: 1 / -1;
          grid-row: 2;
        }

        @media (min-width: 768px) {
          gallery-main {
            grid-column: 2;
            grid-row: 2;
          }
        }

        .sidebar-overlay {
          display: none;
          position: fixed;
          top: var(--header-height, 64px);
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(0, 0, 0, 0.5);
          z-index: calc(var(--z-fixed, 300) - 1);
          opacity: 0;
          transition: opacity 0.25s ease-in-out;
        }

        .sidebar-overlay[data-visible="true"] {
          display: block;
          opacity: 1;
        }

        @media (min-width: 768px) {
          .sidebar-overlay {
            display: none !important;
          }
        }
      </style>

      <div class="gallery-container">
        <gallery-header></gallery-header>
        <gallery-sidebar></gallery-sidebar>
        <gallery-main></gallery-main>
        <div class="sidebar-overlay" aria-hidden="true"></div>
      </div>
    `;
  }
}

customElements.define('terraphim-gallery', TerraphimGallery);
