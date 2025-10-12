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

        if (response && response.data && response.data.replacementMap) {
            console.log('Terraphim: Applying replacements to page (working version)...');

            // Apply replacements directly to DOM
            const replacementMap = response.data.replacementMap;
            applyReplacements(replacementMap);

            console.log('Terraphim: Page parsing completed successfully');
        } else if (response && response.error) {
            console.error('Terraphim: Parse error:', response.error);
        } else {
            console.warn('Terraphim: No response from background script');
        }
    } catch (error) {
        console.error('Terraphim: Client-side error:', error);
    }

    function applyReplacements(replacementMap) {
        // Create a TreeWalker to process text nodes
        const walker = document.createTreeWalker(
            document.body,
            NodeFilter.SHOW_TEXT,
            {
                acceptNode: function(node) {
                    // Skip script and style elements
                    if (node.parentNode.tagName === 'SCRIPT' ||
                        node.parentNode.tagName === 'STYLE') {
                        return NodeFilter.FILTER_REJECT;
                    }
                    return NodeFilter.FILTER_ACCEPT;
                }
            }
        );

        const textNodes = [];
        let node;
        while (node = walker.nextNode()) {
            textNodes.push(node);
        }

        // Process each text node
        textNodes.forEach(textNode => {
            let content = textNode.textContent;
            let modified = false;

            // Apply each replacement
            for (const [pattern, replacement] of Object.entries(replacementMap)) {
                const regex = new RegExp(escapeRegExp(pattern), 'g');
                if (regex.test(content)) {
                    content = content.replace(regex, replacement);
                    modified = true;
                }
            }

            // If content was modified, replace the text node with HTML
            if (modified) {
                const wrapper = document.createElement('span');
                wrapper.innerHTML = content;
                textNode.parentNode.replaceChild(wrapper, textNode);
            }
        });
    }

    function escapeRegExp(string) {
        return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    }
}
)();
