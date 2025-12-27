/**
 * Search Initialization Script
 * Connects the header search input with the search modal
 */

document.addEventListener('DOMContentLoaded', () => {
    const headerSearchInput = document.getElementById('header-search-input');
    const searchModal = document.querySelector('search-modal');

    if (!headerSearchInput || !searchModal) {
        console.warn('Search components not found');
        return;
    }

    // Handle header search input
    headerSearchInput.addEventListener('focus', () => {
        // Open search modal when header input is focused
        searchModal.open();
    });

    headerSearchInput.addEventListener('click', () => {
        // Also open on click
        searchModal.open();
    });

    // Handle header search input value
    headerSearchInput.addEventListener('input', (e) => {
        const query = e.target.value;
        if (query.trim()) {
            searchModal.triggerSearch(query);
        }
    });

    // Prevent header input from actually being used for typing
    // since we're using the modal input instead
    headerSearchInput.addEventListener('keydown', (e) => {
        if (e.key !== 'Tab' && e.key !== 'Escape') {
            e.preventDefault();
            searchModal.open();

            // If it's a printable character, pass it to the modal
            if (e.key.length === 1) {
                setTimeout(() => {
                    const modalInput = searchModal.querySelector('.search-input');
                    if (modalInput) {
                        modalInput.value = e.key;
                        modalInput.focus();
                        // Trigger search
                        modalInput.dispatchEvent(new Event('input', { bubbles: true }));
                    }
                }, 0);
            }
        }
    });

    // Clear header input when modal closes
    searchModal.addEventListener('close', () => {
        headerSearchInput.value = '';
    });

    // Handle URL parameters on page load
    const urlParams = new URLSearchParams(window.location.search);
    const searchQuery = urlParams.get('q');

    if (searchQuery) {
        // Show the search query in header input (for display only)
        headerSearchInput.value = decodeURIComponent(searchQuery);

        // Open search modal with the query
        setTimeout(() => {
            searchModal.triggerSearch(searchQuery);
        }, 100);
    }
});

// Global keyboard shortcut handling
document.addEventListener('keydown', (e) => {
    // Don't interfere if user is typing in an input
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable) {
        return;
    }

    // Open search with '/' key
    if (e.key === '/') {
        e.preventDefault();
        const searchModal = document.querySelector('search-modal');
        if (searchModal) {
            searchModal.open();
        }
    }

    // Open search with Cmd/Ctrl + K
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        const searchModal = document.querySelector('search-modal');
        if (searchModal) {
            searchModal.open();
        }
    }
});