// Content script for Terraphim AI Parse Extension
(async () => {
    try {
        console.log('Terraphim: Starting page parsing...');

        // Get the page HTML content
        const tab_html = document.body.innerHTML;

        // Send HTML to background script for processing
        const response = await chrome.runtime.sendMessage({
            type: 'parse',
            tab_html: tab_html
        });

        if (response && response.data) {
            if (response.data.processedContent) {
                console.log('Terraphim: Applying WASM-processed content to page...');

                // Replace the entire body content with WASM-processed HTML
                document.body.innerHTML = response.data.processedContent;

                console.log('Terraphim: Page parsing completed successfully with WASM processing');
            } else if (response.data.replacementMap) {
                console.log('Terraphim: WASM processing failed, using fallback replacement method...');

                // Apply replacements using the fallback JavaScript method
                applyReplacements(response.data.replacementMap);

                console.log('Terraphim: Page parsing completed with fallback method');
            } else {
                console.warn('Terraphim: No valid data in response');
            }
        } else if (response && response.error) {
            console.error('Terraphim: Parse error:', response.error);
        } else {
            console.warn('Terraphim: No response from background script');
        }
    } catch (error) {
        console.error('Terraphim: Client-side error:', error);
    }

    function applyReplacements(replacementMap) {
        console.log('Applying replacements with', Object.keys(replacementMap).length, 'patterns');
        console.log('Sample patterns:', Object.keys(replacementMap).slice(0, 5));

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

        // Process each text node individually to avoid recursive replacements
        textNodes.forEach(textNode => {
            // Skip already processed nodes (marked with terraphim-processed class)
            if (textNode.parentNode.classList && textNode.parentNode.classList.contains('terraphim-processed')) {
                return;
            }

            const originalContent = textNode.textContent;
            let content = originalContent;
            let replacements = [];

            // First, find all matches without applying replacements yet
            for (const [pattern, replacement] of Object.entries(replacementMap)) {
                try {
                    const regexPattern = `\\b${escapeRegExp(pattern)}\\b`;
                    const regex = new RegExp(regexPattern, 'gi');
                    let match;

                    while ((match = regex.exec(originalContent)) !== null) {
                        replacements.push({
                            start: match.index,
                            end: match.index + match[0].length,
                            original: match[0],
                            replacement: replacement,
                            pattern: pattern
                        });
                    }
                } catch (regexError) {
                    console.warn('Regex failed for pattern:', pattern, regexError);
                }
            }

            // Sort replacements by position (earliest first) and remove overlaps
            replacements.sort((a, b) => a.start - b.start);
            const filteredReplacements = [];
            let lastEnd = -1;

            for (const replacement of replacements) {
                if (replacement.start >= lastEnd) {
                    filteredReplacements.push(replacement);
                    lastEnd = replacement.end;
                }
            }

            // Apply replacements if any were found
            if (filteredReplacements.length > 0) {
                console.log('Applying', filteredReplacements.length, 'replacements to text node');

                // Build new content with replacements
                let newContent = '';
                let lastPos = 0;

                for (const repl of filteredReplacements) {
                    // Add text before replacement
                    newContent += originalContent.substring(lastPos, repl.start);
                    // Add replacement
                    newContent += repl.replacement;
                    lastPos = repl.end;
                }
                // Add remaining text
                newContent += originalContent.substring(lastPos);

                // Create wrapper and mark as processed
                const wrapper = document.createElement('span');
                wrapper.className = 'terraphim-processed';
                wrapper.innerHTML = newContent;
                textNode.parentNode.replaceChild(wrapper, textNode);
            }
        });
    }

    function escapeRegExp(string) {
        return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    }
}
)();

