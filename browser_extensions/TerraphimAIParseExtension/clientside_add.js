// // content script
console.log("Running clientside_add.js");
(async () => {
    var tab_html = document.body.innerText;
    console.log("Tab html", tab_html);
    const response = await chrome.runtime.sendMessage({ type: 'add', title: document.title, url: document.URL, body: tab_html });
    console.log("Response from background ", response);
    alert("Add worked");
    console.log('parsing complete')
}
)();
