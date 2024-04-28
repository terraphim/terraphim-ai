# Front end for Terraphim AI assistant

This code is shared between Tauri desktop and Web.
It uses Svelte for the front end and Bulma for the CSS.
See `../Earthfile` for pre-requisites

## Development

To run in development mode

```sh
yarn  # install dependencies
yarn run dev # run the Svelte dev server 
yarn run tauri dev # run the Tauri dev server 
```

## Production

To build for production

```sh
yarn install # install dependencies
yarn run build # build the Svelte app
yarn run tauri build # build the Tauri app
```

