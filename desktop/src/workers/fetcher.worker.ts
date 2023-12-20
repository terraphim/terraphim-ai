import type { PostMessage, PostMessageDataRequest } from './postmessage';
let result;
async function postJSON(url, data) {
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        "Content-Type": "application/json",
        // 'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: JSON.stringify(data)
    })
    const json = await response.json()
    console.log('posted');
    console.log(json);
  } catch (e) {
    console.log(e);
    result = e;
  }

}
async function fetcher(url, postUrl, isWiki=false) {
  let fetched;
  if (isWiki) {
    url = url + '?action=raw';
  }
 
  try {
    const response = await fetch(url)
    fetched = await response.json();
    console.log(fetched);
    console.log('fetched now post');
    if (isWiki) {
      console.log('is wiki');
      console.log("Processing to be written");
    }else{
      let obj = fetched;
    // loop over list of article in fetched object
    for (let i = 0; i < obj.length; i++) {
      let article = obj[i];
      console.log(article);
      console.log('posting');
      postJSON(postUrl, article);
    }
  }

  } catch (e) {
    console.log(e);
    result = e;
  }
}
onmessage = ({ data: { data, msg } }: MessageEvent<PostMessage<PostMessageDataRequest>>) => {
  console.log(msg, data);
  
  fetcher(data['url'], data['postUrl'], data['isWiki']);

  const message: PostMessage<PostMessageDataRequest> = {
    msg: 'response1',
    data: { text: result }
  };
  postMessage(message);
};

export { };