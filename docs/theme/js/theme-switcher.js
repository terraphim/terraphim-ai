document.addEventListener('DOMContentLoaded', () => {
    const themeToggle = document.getElementById('theme-toggle');
    if (!themeToggle) {
        return;
    }

    // Initialize theme based on stored preference or system preference
    const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const savedTheme = localStorage.getItem('mdbook-theme');
    const initialTheme = savedTheme || (systemPrefersDark ? 'dark' : 'light');

    // Apply the initial theme
    applyTheme(initialTheme);
    updateThemeIcon(initialTheme);

    // Theme toggle click handler
    themeToggle.addEventListener('click', () => {
        const currentTheme = document.documentElement.classList.contains('sl-theme-dark') ? 'dark' : 'light';
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';
        
        applyTheme(newTheme);
        updateThemeIcon(newTheme);
    });

    // Listen for system theme changes
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        const newTheme = e.matches ? 'dark' : 'light';
        applyTheme(newTheme);
        updateThemeIcon(newTheme);
    });

    // Add TOC toggle functionality
    const tocToggle = document.getElementById('toc-toggle');
    const tocWrapper = document.getElementById('toc-wrapper');
    
    if (tocToggle && tocWrapper) {
        tocToggle.addEventListener('click', () => {
            tocWrapper.classList.toggle('hidden');
            tocToggle.setAttribute('aria-expanded', 
                !tocWrapper.classList.contains('hidden'));
        });
    }
});

function applyTheme(theme) {
    const html = document.documentElement;
    
    // Remove existing theme classes
    html.classList.remove('sl-theme-light', 'sl-theme-dark');
    
    // Add new theme class
    html.classList.add(`sl-theme-${theme}`);
    
    // Store theme preference
    localStorage.setItem('mdbook-theme', theme);
}

function updateThemeIcon(theme) {
    const themeToggle = document.getElementById('theme-toggle');
    if (!themeToggle) {
        return;
    }

    const moonIcon = '<i class="fa fa-moon-o"></i>';
    const sunIcon = '<i class="fa fa-sun-o"></i>';
    
    themeToggle.innerHTML = theme === 'light' ? moonIcon : sunIcon;
    themeToggle.setAttribute('title', `Switch to ${theme === 'light' ? 'dark' : 'light'} theme`);
    themeToggle.setAttribute('aria-label', `Switch to ${theme === 'light' ? 'dark' : 'light'} theme`);
} 