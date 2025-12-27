/**
 * Search Modal Component
 * Provides a modal interface for search functionality
 */

class SearchModal extends HTMLElement {
    constructor() {
        super();
        this.isOpen = false;
        this.search = null;
        this.currentResults = [];
        this.selectedIndex = -1;

        // Bind methods
        this.handleKeydown = this.handleKeydown.bind(this);
        this.handleClickOutside = this.handleClickOutside.bind(this);
        this.handleSearchResults = this.handleSearchResults.bind(this);
    }

    connectedCallback() {
        this.render();
        this.setupEventListeners();
        this.initializeSearch();
    }

    disconnectedCallback() {
        this.removeEventListeners();
        if (this.search) {
            this.search.destroy();
        }
    }

    render() {
        this.innerHTML = `
            <div class="search-modal-overlay" style="display: none;">
                <div class="search-modal">
                    <div class="search-modal-header">
                        <div class="search-input-container">
                            <sl-input
                                class="search-input"
                                placeholder="Search documentation..."
                                size="large"
                                clearable
                                autofocus>
                                <sl-icon name="search" slot="prefix"></sl-icon>
                            </sl-input>
                        </div>
                        <sl-button class="search-close-btn" variant="text" size="small">
                            <sl-icon name="x-lg"></sl-icon>
                        </sl-button>
                    </div>

                    <div class="search-results-container">
                        <div class="search-results" role="listbox"></div>
                        <div class="search-footer">
                            <div class="search-shortcuts">
                                <span><kbd>↑</kbd><kbd>↓</kbd> Navigate</span>
                                <span><kbd>Enter</kbd> Select</span>
                                <span><kbd>Esc</kbd> Close</span>
                            </div>
                        </div>
                    </div>

                    <div class="search-loading" style="display: none;">
                        <sl-spinner></sl-spinner>
                        <span>Searching...</span>
                    </div>

                    <div class="search-empty" style="display: none;">
                        <sl-icon name="search" class="search-empty-icon"></sl-icon>
                        <p>No results found</p>
                        <p class="search-empty-subtitle">Try adjusting your search terms</p>
                    </div>
                </div>
            </div>
        `;

        this.setupModalElements();
    }

    setupModalElements() {
        this.overlay = this.querySelector('.search-modal-overlay');
        this.modal = this.querySelector('.search-modal');
        this.input = this.querySelector('.search-input');
        this.closeBtn = this.querySelector('.search-close-btn');
        this.resultsContainer = this.querySelector('.search-results');
        this.loadingElement = this.querySelector('.search-loading');
        this.emptyElement = this.querySelector('.search-empty');
    }

    async initializeSearch() {
        try {
            // Import PagefindSearch if not already available
            if (typeof PagefindSearch === 'undefined') {
                const module = await import('/js/pagefind-search.js');
                window.PagefindSearch = module.default || module.PagefindSearch;
            }

            this.search = new window.PagefindSearch({
                debounceDelay: 200,
                minQueryLength: 1,
                maxResults: 10
            });

            // Handle URL parameters
            const initialQuery = this.search.handleUrlParams();
            if (initialQuery) {
                this.input.value = initialQuery;
                this.performSearch(initialQuery);
            }
        } catch (error) {
            console.error('Failed to initialize search:', error);
        }
    }

    setupEventListeners() {
        // Keyboard shortcuts
        document.addEventListener('keydown', this.handleKeydown);

        // Modal events
        this.closeBtn?.addEventListener('click', () => this.close());
        this.overlay?.addEventListener('click', this.handleClickOutside);

        // Search input events
        this.input?.addEventListener('input', (e) => {
            const query = e.target.value.trim();
            this.performSearch(query);
        });

        this.input?.addEventListener('keydown', (e) => {
            if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                e.preventDefault();
                this.navigateResults(e.key === 'ArrowDown' ? 1 : -1);
            } else if (e.key === 'Enter') {
                e.preventDefault();
                this.selectCurrentResult();
            }
        });
    }

    removeEventListeners() {
        document.removeEventListener('keydown', this.handleKeydown);
    }

    handleKeydown(e) {
        // Open search modal with '/' or 'Cmd+K'
        if (e.key === '/' || (e.key === 'k' && (e.metaKey || e.ctrlKey))) {
            e.preventDefault();
            this.open();
            return;
        }

        // Close modal with Escape
        if (e.key === 'Escape' && this.isOpen) {
            e.preventDefault();
            this.close();
            return;
        }
    }

    handleClickOutside(e) {
        if (e.target === this.overlay) {
            this.close();
        }
    }

    async performSearch(query) {
        if (!this.search) return;

        // Update URL
        this.search.updateUrl(query);

        if (!query || query.length < 1) {
            this.showEmpty();
            return;
        }

        this.showLoading();

        try {
            await this.search.search(query, this.handleSearchResults);
        } catch (error) {
            console.error('Search error:', error);
            this.showEmpty();
        }
    }

    handleSearchResults(searchData, error) {
        this.hideLoading();

        if (error) {
            this.showEmpty();
            return;
        }

        this.currentResults = searchData.results || [];

        if (this.currentResults.length === 0) {
            this.showEmpty();
            return;
        }

        this.renderResults(searchData);
    }

    renderResults(searchData) {
        const { query, results, totalResults } = searchData;

        this.resultsContainer.innerHTML = '';
        this.selectedIndex = -1;

        results.forEach((result, index) => {
            const resultElement = this.createResultElement(result, query, index);
            this.resultsContainer.appendChild(resultElement);
        });

        this.emptyElement.style.display = 'none';
        this.resultsContainer.parentElement.style.display = 'block';
    }

    createResultElement(result, query, index) {
        const element = document.createElement('div');
        element.className = 'search-result-item';
        element.setAttribute('data-index', index);
        element.setAttribute('role', 'option');

        const highlightedTitle = this.search ?
            this.search.highlightTerms(result.title, query) :
            result.title;

        const highlightedExcerpt = this.search ?
            this.search.highlightTerms(result.excerpt, query) :
            result.excerpt;

        element.innerHTML = `
            <div class="search-result-content">
                <h3 class="search-result-title">${highlightedTitle}</h3>
                <p class="search-result-excerpt">${highlightedExcerpt}</p>
                <span class="search-result-url">${result.url}</span>
            </div>
            <div class="search-result-action">
                <sl-icon name="arrow-right"></sl-icon>
            </div>
        `;

        element.addEventListener('click', () => {
            this.selectResult(result);
        });

        element.addEventListener('mouseenter', () => {
            this.setSelectedIndex(index);
        });

        return element;
    }

    navigateResults(direction) {
        if (this.currentResults.length === 0) return;

        const newIndex = this.selectedIndex + direction;

        if (newIndex >= 0 && newIndex < this.currentResults.length) {
            this.setSelectedIndex(newIndex);
        } else if (direction > 0 && this.selectedIndex === this.currentResults.length - 1) {
            this.setSelectedIndex(0);
        } else if (direction < 0 && this.selectedIndex === 0) {
            this.setSelectedIndex(this.currentResults.length - 1);
        }
    }

    setSelectedIndex(index) {
        // Remove previous selection
        const previousSelected = this.resultsContainer.querySelector('.selected');
        if (previousSelected) {
            previousSelected.classList.remove('selected');
        }

        this.selectedIndex = index;

        // Add selection to current item
        const currentItem = this.resultsContainer.querySelector(`[data-index="${index}"]`);
        if (currentItem) {
            currentItem.classList.add('selected');
            currentItem.scrollIntoView({ block: 'nearest' });
        }
    }

    selectCurrentResult() {
        if (this.selectedIndex >= 0 && this.currentResults[this.selectedIndex]) {
            this.selectResult(this.currentResults[this.selectedIndex]);
        }
    }

    selectResult(result) {
        // Navigate to the result
        window.location.href = result.url;
    }

    showLoading() {
        this.loadingElement.style.display = 'flex';
        this.emptyElement.style.display = 'none';
        this.resultsContainer.parentElement.style.display = 'none';
    }

    hideLoading() {
        this.loadingElement.style.display = 'none';
    }

    showEmpty() {
        this.hideLoading();
        this.emptyElement.style.display = 'flex';
        this.resultsContainer.parentElement.style.display = 'none';
        this.currentResults = [];
        this.selectedIndex = -1;
    }

    open() {
        this.isOpen = true;
        this.overlay.style.display = 'flex';

        // Focus input after modal opens
        requestAnimationFrame(() => {
            this.input?.focus();
        });

        // Prevent body scroll
        document.body.style.overflow = 'hidden';
    }

    close() {
        this.isOpen = false;
        this.overlay.style.display = 'none';

        // Restore body scroll
        document.body.style.overflow = '';

        // Clear selection
        this.selectedIndex = -1;
        this.currentResults = [];
    }

    // Public API
    triggerSearch(query) {
        this.input.value = query;
        this.performSearch(query);
        this.open();
    }
}

// Define the custom element
customElements.define('search-modal', SearchModal);