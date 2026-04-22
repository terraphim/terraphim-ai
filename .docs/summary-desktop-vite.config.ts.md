# Summary: desktop/vite.config.ts

**Purpose:** Vite build configuration for the Svelte frontend.

**Key Details:**
- Svelte plugin with SCSS preprocessing
- **Important:** Extensive warning suppressions for CSS and a11y issues (Svelma-related)
- Path aliases: `$lib` -> `./src/lib`, `$workers` -> `./src/workers`
- **Critical:** Svelte internal path mappings to prevent ENOTDIR errors from shim
  - Maps `svelte/internal`, `svelte/store`, `svelte/transition`, etc. to actual source files
  - `svelte-original` alias for real runtime entry
- Rollup manual chunks for vendor code splitting:
  - `vendor-ui`: bulma, svelma, fontawesome
  - `vendor-editor`: tiptap
  - `vendor-charts`: d3
  - `vendor-atomic`: @tomic/lib
  - `vendor-utils`: routing, comlink-fetch
- Chunk size warning limit: 1MB
- Server fs.allow: `..` (serves files from project root)
- OptimizeDeps: includes @tomic/lib, @tomic/svelte
