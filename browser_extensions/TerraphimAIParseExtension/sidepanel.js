// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

const words = {
  extensions:
    'Extensions are software programs, built on web technologies (such as HTML, CSS, and JavaScript) that enable users to customize the Chrome browsing experience.',
  popup:
    "A UI surface which appears when an extension's action icon is clicked."
};

async function getCloudflareCredentials() {
  const stored = await chrome.storage.sync.get(['cloudflareAccountId', 'cloudflareApiToken']);

  if (!stored.cloudflareAccountId || !stored.cloudflareApiToken) {
    throw new Error('Cloudflare credentials not configured. Please set them in the extension options.');
  }

  return {
    accountId: stored.cloudflareAccountId,
    apiToken: stored.cloudflareApiToken
  };
}

async function run(model, input) {
  try {
    const credentials = await getCloudflareCredentials();
    const url = `https://api.cloudflare.com/client/v4/accounts/${credentials.accountId}/ai/run/${model}`;
    console.log("URL", url);

    const response = await fetch(url, {
      headers: { Authorization: `Bearer ${credentials.apiToken}` },
      method: "POST",
      body: JSON.stringify(input),
    });

    const result = await response.json();
    return result;
  } catch (error) {
    console.error('Cloudflare API error:', error);
    throw error;
  }
}


chrome.runtime.onMessage.addListener(async ({ type, data }) => {
  console.log('Message received', type, data);
  if (type === 'concept') {
    // Hide instructions.
    document.body.querySelector('#select-a-word').style.display = 'none';

    // Show word and definition.
    document.body.querySelector('#definition-word').innerText = data;
    run('@cf/meta/m2m100-1.2b', {
      text: data,
      source_lang: "english", // defaults to english
      target_lang: "spanish"
      }).then((response) => {
          console.log(JSON.stringify(response));
          if (!response.success) {
              console.log("Error", response);
          }else{
              console.log("Response", response);
              // let translated_text=response.result.translated_text
              document.body.querySelector('#definition-text').innerText = response.result.translated_text;
          }
      });


    // document.body.querySelector('#definition-text').innerText =
    //   words[data.toLowerCase()];
  }
});
