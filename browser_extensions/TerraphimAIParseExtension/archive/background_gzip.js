// background script
/* eslint-disable no-undef */
const KG_DOMAIN = "https://terraphim.github.io/terraphim-project/#/page/"
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

importScripts('./pako.min.js');
importScripts('./papaparse.min.js');
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
                'Content-Type': 'text/csv'
            }
        }).then(response => {
            return response.arrayBuffer();
        }).then(buffer => pako.inflate(new Uint8Array(buffer)))
            .then(decompressed => {
                // convert binary data to string
                const decoder = new TextDecoder('utf-8');
                const csv = decoder.decode(decompressed);

                // parse csv with Papa Parse
                Papa.parse(csv, {
                    delimiter: ',',
                    dynamicTyping: true,
                    skipEmptyLines: true,
                    header: false,
                    complete: function (results) {

                        for (var i = 0; i < results.data.length; i++) {
                            console.log("Items ",results.data[i][0], results.data[i][1], results.data[i][2]);
                            replacer_config.patterns.push(results.data[i][0]);
                            // FIXME: link #value shall be reverse id match to term
                            var link_string = `<a id="${results.data[i][1]}" href="${KG_DOMAIN}${results.data[i][2]}" target="_blank">${results.data[i][0]}</a>`
                            replacer_config.replace_with.push(link_string);

                        }
                    }
                });
                replacer_config.rdr = message.tab_html;
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


