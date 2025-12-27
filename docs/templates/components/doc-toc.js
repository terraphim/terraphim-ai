class DocToc extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.render();
    this.generateToc();
  }

  generateToc() {
    const article = document.querySelector('.main-article');
    if (!article) return;

    const headers = Array.from(article.querySelectorAll('h1, h2, h3, h4, h5, h6'));
    const tocList = document.createElement('ul');
    tocList.className = 'toc-list';

    headers.forEach(header => {
      // Skip the main title
      if (header.tagName === 'H1' && header === article.querySelector('h1')) {
        return;
      }

      const level = parseInt(header.tagName.charAt(1));
      const title = header.textContent;
      const id = this.slugify(title);

      // Add id to the header if it doesn't have one
      if (!header.id) {
        header.id = id;
      }

      const listItem = document.createElement('li');
      listItem.className = `toc-item level-${level}`;

      const link = document.createElement('a');
      link.href = `#${id}`;
      link.textContent = title;

      listItem.appendChild(link);
      tocList.appendChild(listItem);
    });

    const tocContent = this.shadowRoot.querySelector('.toc-content');
    tocContent.innerHTML = '';
    tocContent.appendChild(tocList);
  }

  slugify(text) {
    return text.toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/(^-|-$)/g, '');
  }

  render() {
    this.shadowRoot.innerHTML = `
      <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.12.0/cdn/themes/light.css" />
      <link rel="stylesheet" href="/css/styles.css">
      <style>
        :host {
          display: block;
          padding: var(--sl-spacing-medium);
          background: var(--sl-panel-background-color);
          border-left: solid var(--sl-panel-border-width) var(--sl-panel-border-color);
        }

        .toc-header {
          font-family: var(--sl-font-sans);
          font-size: var(--sl-font-size-small);
          font-weight: var(--sl-font-weight-semibold);
          text-transform: uppercase;
          color: var(--sl-color-neutral-500);
          margin-bottom: var(--sl-spacing-medium);
        }

        .toc-list {
          list-style: none;
          padding: 0;
          margin: 0;
          font-family: var(--sl-font-sans);
        }

        .toc-item {
          margin: var(--sl-spacing-2x-small) 0;
        }

        .toc-item.level-1 { padding-left: 0; }
        .toc-item.level-2 { padding-left: var(--sl-spacing-large); }
        .toc-item.level-3 { padding-left: calc(var(--sl-spacing-large) * 2); }
        .toc-item.level-4 { padding-left: calc(var(--sl-spacing-large) * 3); }
        .toc-item.level-5 { padding-left: calc(var(--sl-spacing-large) * 4); }
        .toc-item.level-6 { padding-left: calc(var(--sl-spacing-large) * 5); }

        a {
          color: var(--sl-color-neutral-700);
          text-decoration: none;
          font-size: var(--sl-font-size-small);
          line-height: var(--sl-line-height-normal);
          transition: var(--sl-transition-fast) color;
        }

        a:hover {
          color: var(--sl-color-primary-600);
        }

        @media (max-width: 1200px) {
          :host {
            display: none;
          }
        }
      </style>
      <div class="toc-header">On this page</div>
      <div class="toc-content"></div>
    `;
  }
}

customElements.define('doc-toc', DocToc);