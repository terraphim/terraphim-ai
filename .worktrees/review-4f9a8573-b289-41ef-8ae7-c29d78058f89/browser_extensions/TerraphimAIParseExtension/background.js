// background script
/* eslint-disable no-undef */

// Import API utility
importScripts('api.js');
// KG domain used to form links to published logseq knowledge graph, currently only works for project manager role
// TERRAPHIM_POST URL is used to add to Terraphim instance index
// toWikiLinksConcepts 1: will find and turn to wikilinks in format: matched concept [[normalized concept]], i.e. synonyms will link to root concept, if toWikiLinksConcepts=2 will create links to concepts directly [[matched concept]]
// test pages for System Operator:
// https://sebokwiki.org/wiki/System_Operation
// https://sebokwiki.org/wiki/Procurement_and_Acquisition
// https://sebokwiki.org/wiki/System_Maintenance
// https://sebokwiki.org/wiki/Maintainability_(glossary)
importScripts('./pkg/terrraphim_automata_wasm_worker.js');

// Use the singleton API instance from api.js
let api = null;

chrome.runtime.onInstalled.addListener(async () => {
    await initializeExtension();
});

chrome.runtime.onStartup.addListener(async () => {
    await initializeExtension();
});

async function initializeExtension() {
    try {
        // Use the singleton instance instead of creating a new one
        api = terraphimAPI;
        await api.initialize();

        // Load WASM
        await loadWasm();

        // Setup context menu
        setupContextMenu();

        console.log('TerraphimAI Parse Extension initialized');
    } catch (error) {
        console.error('Failed to initialize extension:', error);
    }
}

async function loadWasm() {
    // Initialize the WASM module
    await wasm_bindgen('./pkg/terrraphim_automata_wasm_bg.wasm');
    // Call the exported functions from the WASM module
    wasm_bindgen.print_with_value('Wasm Works!');
}

function setupContextMenu() {
    chrome.contextMenus.create({
      id: 'define-word',
      title: 'Define',
      contexts: ['selection']
    });
  }



chrome.runtime.onMessage.addListener(function (message, sender, senderResponse) {
    (async () => {
        try {
            // Check if API is initialized and configured
            if (!api) {
                api = terraphimAPI; // Fallback to singleton if not set
            }

            if (!api.isConfigured()) {
                // Try to re-initialize if not configured
                try {
                    await api.initialize();
                } catch (initError) {
                    console.error('Failed to initialize API:', initError);
                }

                if (!api.isConfigured()) {
                    senderResponse({
                        error: "Extension not configured. Please configure server settings in options page (right-click extension icon → Options)."
                    });
                    return;
                }
            }

            await wasm_bindgen('./pkg/terrraphim_automata_wasm_bg.wasm');
            var replacer_config = { patterns: [], replace_with: [], rdr: String };

        if (message === true) {
            console.log("Received message from clientside.js");
        }

        if (message.type === "parse") {
            try {
                // Get thesaurus from API instead of hardcoded URL
                const thesaurus = await api.getThesaurus();

                // Check if thesaurus is empty - this might indicate missing dependencies
                if (!thesaurus || Object.keys(thesaurus).length === 0) {
                    const stored = await chrome.storage.sync.get(['cloudflareAccountId', 'cloudflareApiToken']);
                    if (!stored.cloudflareAccountId || !stored.cloudflareApiToken) {
                        senderResponse({
                            error: "Concept mapping requires Cloudflare API credentials. Please configure Cloudflare Account ID and API Token in the extension options."
                        });
                        return;
                    } else {
                        senderResponse({
                            error: "No thesaurus data available for concept mapping. Check your role configuration and server connection."
                        });
                        return;
                    }
                }

                // Get wiki link mode from storage
                const stored = await chrome.storage.sync.get(['wikiLinkMode']);
                const wikiLinkMode = parseInt(stored.wikiLinkMode || '0');

                // Get KG domain from current role
                const kgDomain = api.getKnowledgeGraphDomain() || "logseq://graph/default?page=";

                console.log("Thesaurus loaded:", Object.keys(thesaurus).length, "terms");

                // Prepare WASM replacer config with input content
                const patterns = Object.keys(thesaurus);
                const replace_with = patterns.map(key => {
                    const nterm = thesaurus[key];
                    let link_string;
                    if (wikiLinkMode === 1) {
                        link_string = `${key} [[${nterm}]]`;
                    } else if (wikiLinkMode === 2) {
                        link_string = `[[${key}]]`;
                    } else {
                        // Generate unique ID for the term
                        const termId = btoa(nterm).replace(/[+/=]/g, '').substring(0, 8);
                        link_string = `<a id="${termId}" href="${kgDomain}${encodeURIComponent(nterm)}" target="_blank">${key}</a>`;
                    }
                    return link_string;
                });

                // Get the HTML content to process
                const htmlContent = message.tab_html;
                console.log("Processing content with WASM using", patterns.length, "patterns");

                try {
                    // Prepare config for WASM processing
                    const wasmConfig = {
                        patterns: patterns,
                        replace_with: replace_with,
                        content: htmlContent
                    };

                    // Use WASM to process the HTML content
                    const processedContent = wasm_bindgen.replace_all_stream(wasmConfig);

                    console.log("WASM processing completed successfully");

                    // Send processed content to client
                    senderResponse({
                        data: {
                            processedContent: processedContent,
                            wikiLinkMode: wikiLinkMode
                        }
                    });
                } catch (wasmError) {
                    console.error("WASM processing failed:", wasmError);
                    console.log("Falling back to replacement map method");

                    // Fallback to replacement map if WASM fails
                    const replacementMap = {};
                    patterns.forEach((pattern, index) => {
                        replacementMap[pattern] = replace_with[index];
                    });

                    senderResponse({
                        data: {
                            replacementMap: replacementMap,
                            wikiLinkMode: wikiLinkMode
                        }
                    });
                }

            } catch (error) {
                console.error("Parse error:", error);
                senderResponse({ error: "Failed to parse: " + error.message });
            }

        } else if (message.type === "add") {
            try {
                // Use API to add document
                const result = await api.addDocument(message.title, message.body, message.url);
                console.log("Document added successfully:", result);
                senderResponse({ data: result });

            } catch (error) {
                console.error("Add document error:", error);
                senderResponse({ error: "Failed to add document: " + error.message });
            }

        } else if (message.type === "concept") {
            let data = message.data;
            console.log("Concept", data);
            senderResponse({ data: data });

        } else {
            senderResponse({ data: "No data" });
        }
    } catch (globalError) {
        console.error("Global message handler error:", globalError);
        senderResponse({ error: "Message handler failed: " + globalError.message });
    }
    })();
    return true;
});

init = (tab) => {
    const { id, url } = tab;
    chrome.scripting.executeScript(
        {
            target: { tabId: id, allFrames: true },
            files: ['clientside_html.js']
        }
    )
    console.log(`Loading: ${url}`);
}

chrome.action.onClicked.addListener(tab => {
    init(tab)
});

// chrome.contextMenus.onClicked.addListener((data) => {
//     chrome.runtime.sendMessage({
//       name: 'define-word',
//       data: { value: data.selectionText }
//     });
//   });

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
    const { id, url } = tab;
    const { menuItemId } = info
    var query = info.selectionText;
    switch (menuItemId) {
      case "define-word":
        await chrome.sidePanel.open({ tabId: id });
        await chrome.sidePanel.setOptions({
            tabId: id,
            path: 'sidepanel.html',
            enabled: true
        });
        await chrome.scripting.executeScript(
            {
                target: { tabId: id, allFrames: true },
                files: ['clientside_concept.js']
            }
        )
    }
});
