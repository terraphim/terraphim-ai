// // content script
console.log("Running clientside_concepts.js");
(async () => {
    let selection = document.getSelection().toString();
    console.log("Selection", selection);
    const response = await chrome.runtime.sendMessage({ type: 'concept', data:selection });
    console.log("Response from background ", response);
    console.log('concept complete')
}
)();
