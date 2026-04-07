// background script
const KG_DOMAIN = "https://terraphim.github.io/terraphim-project/#/page/";
const toWikiLinksConcepts = true;

importScripts('./wasm/pkg/terrraphim_automata_wasm.js');
chrome.runtime.onInstalled.addListener(() => {
    runDemo();
});

async function runDemo() {
    // Initialize the WASM module
    await wasm_bindgen('/wasm/pkg/terrraphim_automata_wasm_bg.wasm');

    // Call the exported functions from the WASM module
    wasm_bindgen.print_with_value('Wasm Works!');
}

chrome.runtime.onMessage.addListener(function (message, sender, senderResponse) {
    var replacer_config = { patterns: [], replace_with: [], rdr: String };
    if (message === true) {
        console.log("Recieved message from clientside.js");
        // chrome.runtime.sendMessage({type: 'message', message: 'Test Message'});
    }
    console.log("Getting url", message.url);
    if (message.type === "dict") {
        fetch(message.url, {
            method: 'GET',
            headers: {
                'Content-Type': 'text/json',
                'Accept': 'application/json'
            }
        }).then(response => {
            return response.json();
        }).then(json => {
            console.log("JSON", json);
            // iterate over json
            json.forEach((item) => {
            console.log(`{$item.term} ${item.id} ${item.nterm}`);
            replacer_config.patterns.push(item.term);
            if (toWikiLinksConcepts===true){
                console.log("toWikiLinksConcepts", toWikiLinksConcepts);
                var link_string = `${item.term}[[${item.nterm}]]`;
            }else{
                var link_string = `[[${item.term}]]`;
            }
                replacer_config.replace_with.push(link_string);
            });
                replacer_config.rdr = message.tab_text;
                console.log("replacer config", replacer_config);
                wasm_bindgen('/wasm/pkg/terrraphim_automata_wasm_bg.wasm');
                var return_text = wasm_bindgen.replace_all_stream(replacer_config);
                console.log("Return text", return_text);
                console.log("Dict", replacer_config);
                senderResponse({ data: { "replacer_config": replacer_config, return_text: return_text } });
                return true;
            })
    } else {
        senderResponse({ data: "No data" });
        return true;
    }
    return true;
});

init = (tab) => {
    const { id, url } = tab;
    chrome.scripting.executeScript(
        {
            target: { tabId: id, allFrames: true },
            files: ['clientside.js']
        }
    )
    console.log(`Loading: ${url}`);
}

chrome.action.onClicked.addListener(tab => {
    init(tab)
});
