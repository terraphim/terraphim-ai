# Extension publication output receipts

These are results, not input fixtures. They are bound to the source and CI run named in each receipt.

- `native-vscode-final.json`: final repaired VSIX at source `4c45aa221ad573266a73cbacdbad2620a42a26ca`, clean tracked source, same package on all three hosts. All 18 saved files were independently hashed; each exported graph was searched with released grep and returned three Risk decisions. Includes exact Markdown link output from the packaged legacy helper. [Final CI run](https://github.com/terraphim/terraphim-ai/actions/runs/38218712077).
- `native-vscode-ci.json`: downloaded Linux/macOS/Windows files independently matched every recorded hash. Native undo differs from edited source; redo equals it. All three hosts used one VSIX.
- `released-grep-ci-graphs.json`: released macOS terraphim-grep 1.21.19 actually read each host-produced graph and retrieved three Risk decisions for Concern, with no synthesis answer.
- `continue-ci.json`: actual Linux MCP client/server transport and released grep; literal Concern baseline returned no chunks, the exported graph retrieved the Risk excerpt. This CI corpus contains one risk line; it is separate from the three-decision corpus above.

Raw host files and receipts are retained as artifacts of https://github.com/terraphim/terraphim-ai/actions/runs/38216701669. Continue output comes from https://github.com/terraphim/terraphim-ai/actions/runs/38215897542.

These checks do not certify webview input, native save dialogs, the complete original 41 requirements, Continue's own UI, a Zed installation or marketplace publication. See ../../plans/design-extension-marketplaces.md for the release gates.
