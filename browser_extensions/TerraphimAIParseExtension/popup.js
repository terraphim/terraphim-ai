// Global elements
const elements = {
    terraphimIt: document.getElementById('terraphim-it'),
    terraphimAdd: document.getElementById('terraphim-add'),
    terraphimConcept: document.getElementById('terraphim-concept'),
    status: document.getElementById('status'),
    serverInfo: document.getElementById('server-info'),
    currentRole: document.getElementById('current-role'),
    configureButton: document.getElementById('configure-button')
};

// Initialize popup
document.addEventListener('DOMContentLoaded', async () => {
    await initializePopup();
});

async function initializePopup() {
    try {
        // Load API and configuration status
        const stored = await chrome.storage.sync.get(['serverUrl', 'currentRole']);

        if (stored.serverUrl) {
            elements.serverInfo.textContent = `Server: ${stored.serverUrl}`;
            elements.currentRole.textContent = `Role: ${stored.currentRole || 'None'}`;

            // Test if server is reachable
            try {
                const response = await fetch(`${stored.serverUrl}/health`, {
                    method: 'GET',
                    signal: AbortSignal.timeout(3000)
                });

                if (response.ok) {
                    updateStatus('success', 'Connected');
                    enableButtons(true);
                } else {
                    updateStatus('warning', 'Server not responding');
                    enableButtons(false);
                }
            } catch (error) {
                updateStatus('error', 'Connection failed');
                enableButtons(false);
            }
        } else {
            updateStatus('warning', 'Not configured');
            elements.serverInfo.textContent = 'Server: Not configured';
            elements.currentRole.textContent = 'Role: None';
            enableButtons(false);
        }
    } catch (error) {
        console.error('Failed to initialize popup:', error);
        updateStatus('error', 'Initialization failed');
        enableButtons(false);
    }
}

async function getCurrentTab() {
    const queryOptions = { active: true, currentWindow: true };
    const [tab] = await chrome.tabs.query(queryOptions);
    return tab;
}

function updateStatus(type, message) {
    elements.status.textContent = message;
    elements.status.className = `status ${type}`;
}

function enableButtons(enabled) {
    elements.terraphimIt.disabled = !enabled;
    elements.terraphimAdd.disabled = !enabled;
    elements.terraphimConcept.disabled = !enabled;
}

// Event listeners
elements.terraphimConcept.addEventListener('click', async () => {
    if (elements.terraphimConcept.disabled) return;

    updateStatus('info', 'Processing...');
    const tab = await getCurrentTab();

    chrome.scripting.executeScript({
        target: { tabId: tab.id },
        files: ['clientside_html.js']
    });
    console.log(`Processing tab: ${tab.url}`);
});

elements.terraphimAdd.addEventListener('click', async () => {
    if (elements.terraphimAdd.disabled) return;

    updateStatus('info', 'Adding to knowledge base...');
    const tab = await getCurrentTab();

    chrome.scripting.executeScript({
        target: { tabId: tab.id },
        files: ['clientside_add.js']
    });
    console.log(`Adding document: ${tab.url}`);
});

elements.configureButton.addEventListener('click', () => {
    chrome.runtime.openOptionsPage();
});

// Listen for messages from content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.type === 'popup_status') {
        updateStatus(message.status, message.message);
    }
});
