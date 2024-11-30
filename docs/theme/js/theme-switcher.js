document.addEventListener('DOMContentLoaded', () => {
    const themeToggle = document.getElementById('theme-toggle');
    const tocToggle = document.getElementById('toc-toggle');
    const tocWrapper = document.querySelector('.toc-wrapper');
    
    if (!themeToggle) return;

    function setTheme(theme) {
        document.documentElement.classList.remove('sl-theme-light', 'sl-theme-dark');
        document.documentElement.classList.add(`sl-theme-${theme}`);
        localStorage.setItem('mdbook-theme', theme);
        
        // Update icon
        const icon = themeToggle.querySelector('i');
        if (icon) {
            icon.className = theme === 'light' ? 'fa fa-sun-o' : 'fa fa-moon-o';
        }
    }

    // Initialize theme
    const savedTheme = localStorage.getItem('mdbook-theme');
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const initialTheme = savedTheme || (prefersDark ? 'dark' : 'light');
    setTheme(initialTheme);

    // Handle theme toggle
    themeToggle.addEventListener('click', () => {
        const currentTheme = document.documentElement.classList.contains('sl-theme-dark') ? 'dark' : 'light';
        setTheme(currentTheme === 'dark' ? 'light' : 'dark');
    });

    // Handle TOC toggle
    if (tocToggle && tocWrapper) {
        tocToggle.addEventListener('click', () => {
            tocWrapper.classList.toggle('show');
            tocToggle.setAttribute('aria-expanded', tocWrapper.classList.contains('show'));
        });
    }
}); 