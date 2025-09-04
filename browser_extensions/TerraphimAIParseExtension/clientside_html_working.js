// Content script for Terraphim AI Parse Extension (Working Version)
// This version demonstrates the proper integration with the background script

(async () => {
    try {
        console.log('Terraphim: Starting page parsing (working version)...');

        // Get the page HTML content
        const tab_html = document.body.innerHTML;

        // Send HTML to background script for processing
        const response = await chrome.runtime.sendMessage({
            type: 'parse',
            tab_html: tab_html
        });

        if (response && response.data && response.data.return_text) {
            // Replace the body content with processed HTML
            document.body.innerHTML = response.data.return_text;
            console.log('Terraphim: Page parsing completed successfully');
        } else if (response && response.error) {
            console.error('Terraphim: Parse error:', response.error);
        } else {
            console.warn('Terraphim: No response from background script');
        }
    } catch (error) {
        console.error('Terraphim: Client-side error:', error);
    }
}
)();

