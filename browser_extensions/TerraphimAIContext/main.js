// Import API utility
importScripts('api.js');

// Global API instance
let api = null;

// Initialize the extension
async function initializeExtension() {
    try {
        api = new TerraphimContextAPI();
        await api.initialize();
        console.log('TerraphimAI Context Extension initialized');
    } catch (error) {
        console.error('Failed to initialize extension:', error);
    }
}

// Initialize when extension starts
initializeExtension();

// Helper function to show notifications
function showNotification(title, message) {
  if (chrome.notifications) {
    chrome.notifications.create({
      type: 'basic',
      iconUrl: 'assets/search.png',
      title: title,
      message: message
    });
  } else {
    console.warn('Notifications not supported, logging:', title, '-', message);
  }
}

chrome.contextMenus.removeAll(function () {
  chrome.contextMenus.create({
    id: "1",
    title: "Search in Terraphim AI",
    contexts: ["selection"],  // ContextType
  });
  chrome.contextMenus.create({
    id: "2",
    title: "Add Selection to LogSeq",
    contexts: ["selection"],  // ContextType
  });
  chrome.contextMenus.create({
    id: "3",
    title: "Add to Atomic Server",
    contexts: ["selection"],  // ContextType
  });
  chrome.contextMenus.create({
    id: "4",
    title: "Concept Refactoring in Terraphim AI",
    contexts: ["selection"],  // ContextType
  });
  chrome.contextMenus.onClicked.addListener(async (info, tab) => {
    const { menuItemId } = info;
    const query = info.selectionText;

    // Ensure API is initialized
    if (!api) {
      api = new TerraphimContextAPI();
      await api.initialize();
    }

    console.log(`Context menu action ${menuItemId} clicked with text: "${query}"`);

    try {
      switch (menuItemId) {
        case "1":
          // Search in Terraphim AI using dynamic server URL
          if (api.isConfigured() && api.serverUrl) {
            const searchUrl = api.getSearchUrl(query);
            if (searchUrl) {
              chrome.tabs.create({ url: searchUrl });
            } else {
              console.error('No search URL available');
              showNotification('Error', 'Server not configured. Please configure in extension options.');
            }
          } else {
            console.error('API not configured');
            showNotification('Configuration Required', 'Please configure the Terraphim server in extension options.');
            chrome.runtime.openOptionsPage();
          }
          break;

        case "2":
          // Add to LogSeq using dynamic URL generation
          const logseqUrl = api.getLogseqUrl(query, info.pageUrl);
          console.log("Logseq URL", logseqUrl);
          chrome.tabs.create({ url: logseqUrl });
          break;

        case "3":
          // Add to Atomic Server using configurable URL
          const atomicUrl = api.getAtomicServerUrl(query);
          chrome.tabs.create({ url: atomicUrl });
          break;

        case "4":
          // Concept refactoring - add selected text to knowledge base
          if (api.isConfigured()) {
            try {
              const title = `Selection from ${tab.title || tab.url}`;
              const body = `Selected text: "${query}"\n\nFrom: ${info.pageUrl}`;

              await api.addDocument(title, body, info.pageUrl);
              showNotification('Success', 'Text added to knowledge base');
              console.log("Concept added to knowledge base");
            } catch (error) {
              console.error("Failed to add concept:", error);
              showNotification('Error', 'Failed to add to knowledge base: ' + error.message);
            }
          } else {
            console.error('API not configured for concept refactoring');
            showNotification('Configuration Required', 'Please configure the Terraphim server in extension options.');
            chrome.runtime.openOptionsPage();
          }
          break;

        default:
          console.log("Unknown menu item ID");
      }
    } catch (error) {
      console.error('Context menu action failed:', error);
      showNotification('Error', error.message);
    }

    console.log("Context menu action completed");
  })
})

