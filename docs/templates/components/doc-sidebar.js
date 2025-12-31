class DocSidebar extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.render();
  }

  render() {
    this.shadowRoot.innerHTML = `
      <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.12.0/cdn/themes/light.css" />
      <script type="module" src="https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.12.0/cdn/components/icon/icon.js"></script>
      <style>
        :host {
          display: block;
          padding: var(--sl-spacing-medium);
          background: var(--sl-panel-background-color);
          border-right: solid var(--sl-panel-border-width) var(--sl-panel-border-color);
        }

        /* Reset all list styles */
        ::slotted(ul),
        ::slotted(li) {
          list-style: none;
          list-style-type: none;
          padding: 0;
          margin: 0;
        }

        .sidebar-nav {
          padding: var(--sl-spacing-medium);
        }

        .sidebar-section {
          margin-bottom: var(--sl-spacing-large);
        }

        .sidebar-section-title {
          font-family: var(--sl-font-sans);
          font-size: var(--sl-font-size-small);
          font-weight: var(--sl-font-weight-semibold);
          text-transform: uppercase;
          color: var(--sl-color-neutral-500);
          margin-bottom: var(--sl-spacing-medium);
        }

        .sidebar-items {
          margin: 0 0 0 var(--sl-spacing-medium);
        }

        .sidebar-item {
          margin: var(--sl-spacing-x-small) 0;
        }

        /* Reset all link styles */
        ::slotted(a),
        ::slotted(a:visited) {
          color: var(--sl-color-neutral-700) !important;
          text-decoration: none !important;
        }

        .sidebar-item a {
          display: flex;
          align-items: center;
          gap: var(--sl-spacing-medium);
          color: var(--sl-color-neutral-700);
          text-decoration: none;
          font-size: var(--sl-font-size-medium);
          line-height: var(--sl-line-height-normal);
          padding: var(--sl-spacing-x-small) var(--sl-spacing-medium);
          border-radius: var(--sl-border-radius-medium);
          transition: var(--sl-transition-medium) background-color,
                    var(--sl-transition-medium) color;
          position: relative;
        }

        .sidebar-item sl-icon {
          font-size: 1em;
          color: var(--sl-color-neutral-400);
          transition: var(--sl-transition-medium) color;
        }

        /* Hover state */
        .sidebar-item a:hover {
          background: var(--sl-color-neutral-100);
          color: var(--sl-color-primary-600);
          text-decoration: none;
        }

        .sidebar-item a:hover sl-icon {
          color: var(--sl-color-primary-600);
        }

        /* Active state */
        .sidebar-item a.active {
          background: var(--sl-color-primary-100);
          color: var(--sl-color-primary-600);
          font-weight: var(--sl-font-weight-semibold);
          text-decoration: none;
        }

        .sidebar-item a.active sl-icon {
          color: var(--sl-color-primary-600);
        }

        .sidebar-item a.active::before {
          content: '';
          position: absolute;
          left: calc(-1 * var(--sl-spacing-x-small));
          top: 0;
          bottom: 0;
          width: 3px;
          background: var(--sl-color-primary-600);
          border-radius: 0 var(--sl-border-radius-medium) var(--sl-border-radius-medium) 0;
        }

        @media (max-width: 768px) {
          :host {
            display: none;
          }
        }
      </style>
      <nav class="sidebar-nav">
        <slot></slot>
      </nav>
    `;

    // Only keep page icons code
    this.shadowRoot.host.querySelectorAll('.sidebar-item a').forEach(link => {
      const icon = document.createElement('sl-icon');
      if (link.matches('.active')) {
        icon.name = 'bookmark-fill';
      } else {
        icon.name = 'chevron-right';
      }
      link.prepend(icon);
    });
  }
}

customElements.define('doc-sidebar', DocSidebar);