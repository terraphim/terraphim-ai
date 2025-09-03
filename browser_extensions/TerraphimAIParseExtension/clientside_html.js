// // content script
(async () => {
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
        let newHtml=`<a href='https://alexmikhalev.terraphim.cloud?search=${encodedString}' target='_top'>${origText}</a>`;
        // let newHtml=origText.replace(/\binference\b/g,'<a href="http://www.cnn.com">asdf</a>');
        //Only change the DOM if we actually made a replacement in the text.
        //Compare the strings, as it should be faster than a second RegExp operation and
        //  lets us use the RegExp in only one place for maintainability.
        if( newHtml !== origText) {
            let newSpan = document.createElement('span');
            newSpan.innerHTML = newHtml;
            textNode.parentNode.replaceChild(newSpan,textNode);
        }
    }

    //Testing: Walk the DOM of the <body> handling all non-empty text nodes
    function processDocument() {
        //Create the TreeWalker
        let treeWalker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT,{
            acceptNode: function(node) {
                if(node.textContent.length === 0) {
                    //Alternately, could filter out the <script> and <style> text nodes here.
                    return NodeFilter.FILTER_SKIP; //Skip empty text nodes
                } //else
                return NodeFilter.FILTER_ACCEPT;
            }
        }, false );
        //Make a list of the text nodes prior to modifying the DOM. Once the DOM is
        //  modified the TreeWalker will become invalid (i.e. the TreeWalker will stop
        //  traversing the DOM after the first modification).
        let nodeList=[];
        while(treeWalker.nextNode()){
            nodeList.push(treeWalker.currentNode);
        }
        //Iterate over all text nodes, calling handleTextNode on each node in the list.
        nodeList.forEach(function(el){
            handleTextNode(el);
        });
    }
    processDocument();
    console.log('parsing complete');
}
)();

