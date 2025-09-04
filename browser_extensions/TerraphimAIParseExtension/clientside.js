// // content script
console.log("Running clientside.js");
(async () => {
    var tab_html = document.body.innerHTML;
    console.log("Tab html", tab_html);
    const response = await chrome.runtime.sendMessage({ type: 'parse', url: 'https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json', tab_html: tab_html });
    console.log("Response from background ", response);
    document.body.innerHTML = response.data.return_text;
    console.log('parsing complete')
}
)();

