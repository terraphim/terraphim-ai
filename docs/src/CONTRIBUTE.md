# Contributing
## Guide for Terraphim AI contributors
## Earthly-Based Development

You can run the full stack using Earthly.
From the project root, execute the following command:

```sh
earthly ls
```

This will list all the available targets. You can then run the full stack using the following command:

```sh
earthly +pipeline
```

This will build the full stack using Earthly.

# Local Development

If you want to develop without using Earthly, you need a local node.js, Yarn,
and Rust environment.

Then you can run the following commands:

## Install the sample data for `system_operator`

```sh
git clone https://github.com/terraphim/INCOSE-Systems-Engineering-Handbook.git /tmp/system_operator/
```

## Run the backend

```sh
cargo run 
```

## Run the frontend in development mode:

```sh
yarn run dev
```
from ./desktop folder

to compile tauri in dev mode run:

```
yarn run tauri dev 
```