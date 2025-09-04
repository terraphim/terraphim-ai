// // content script
(async () => {
    // var tab_html = document.body.innerHTML;
    // console.log("Tab html", tab_html);
    // const response = await chrome.runtime.sendMessage({ type: 'dict', url: 'https://project-manager-terraphim-kg-ci.s3.eu-west-2.amazonaws.com/term_to_id.json', tab_html: tab_html });
    // console.log("Response from background ", response);
    // document.body.innerHTML = response.data.return_text;
    // console.log('parsing complete')
    function handleTextNode(textNode) {
        if(textNode.nodeName !== '#text'
            || textNode.parentNode.nodeName === 'SCRIPT'
            || textNode.parentNode.nodeName === 'STYLE'
        ) {
            //Don't do anything except on text nodes, which are not children
            //  of <script> or <style>.
            return;
        }
        let origText = textNode.textContent;
        let encodedString=new URLSearchParams(origText).toString();
        // let newHtml=origText.replace(/(^|\s)(\S+)(?=\s|$)/mg, '$1<button>$2</button>');
        let newHtml=`<a href='https://alexmikhalev.terraphim.cloud?search=${encodedString}' target='_top'>${origText}</a>`;
        //Only change the DOM if we actually made a replacement in the text.
        //Compare the strings, as it should be faster than a second RegExp operation and
        //  lets us use the RegExp in only one place for maintainability.
        if( newHtml !== origText) {
            let newSpan = document.createElement('span');
            newSpan.innerHTML = newHtml;
            textNode.parentNode.replaceChild(newSpan,textNode);
        }
    }

    //Find all text node descendants of <p> elements:
    let allP = document.querySelectorAll('p');  // Get all <p>
    console.log("All P", allP);
    let textNodes = [];
    for (let p of allP) {
        //Create a NodeIterator to get the text nodes descendants of each <p>
        let nodeIter = document.createNodeIterator(p,NodeFilter.SHOW_TEXT);
        let currentNode;
        //Add text nodes found to list of text nodes to process below.
        while(currentNode = nodeIter.nextNode()) {
            textNodes.push(currentNode);
        }
    }
    //Process each text node
    textNodes.forEach(function(el){
        handleTextNode(el);
    });
}
)();

