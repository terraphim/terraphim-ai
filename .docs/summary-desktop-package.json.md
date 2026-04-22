# Summary: desktop/package.json

**Purpose:** Frontend application manifest for the Svelte-based web UI.

**Key Details:**
- Package name: `terraphim-search-ui`, version 1.5.0
- **Package manager:** Yarn (has yarn.lock)
- Build tool: Vite 5.3.4
- Framework: Svelte 5.55.1 + TypeScript 5.0.4
- CSS: Bulma 1.0.4 + Bulmaswatch 0.8.1 (NOT Tailwind)
- Test runner: Vitest 1.6.0
- E2E: Playwright 1.57.0
- Editor: TipTap 2.11.5 (rich text editor)
- Charts: D3 7.9.0
- Atomic Data: @tomic/lib 0.40.0
- Icons: FontAwesome 7.0.1
- Routing: svelte-routing 2.13.0, tinro 0.6.12
- Postinstall: `patch-package` for patching dependencies
- Many peer dependencies marked as optional
