# OpenCode + Terraphim Experiment Results

**Date**: 2026-05-02
**Task**: Build accessible navigation component in SvelteKit
**Project**: `~/projects/frontend-test` (fresh SvelteKit project)

## Hypothesis

OpenCode with terraphim-agent (KG-boosted search) produces better results faster than OpenCode without terraphim.

## Setup

| Component | Version |
|-----------|---------|
| OpenCode | 1.14.31 |
| terraphim-agent | 1.17.0 |
| terraphim_mcp_server | built from source |
| SvelteKit | 2.57.0 + Svelte 5.55.2 |

### Flow A: OpenCode WITH terraphim
- terraphim-agent running with Frontend Developer role
- KG records from `~/.config/terraphim/kg/frontend/`
- Ripgrep haystack + FFF MCP tools available

### Flow B: OpenCode WITHOUT terraphim (control)
- Fresh project with no existing component
- Default OpenCode search (no terraphim tools)

## Results

| Metric | Flow A (+terraphim) | Flow B (control) | Difference |
|--------|---------------------|------------------|------------|
| **Steps** | 11 | 16 | -31% |
| **Total tokens** | 500,749 | 686,995 | -27% |
| **Tool calls** | 15 | 17 | -12% |
| **Final tokens** | 50,595 | 50,510 | +0.2% |
| **Cache read** | 428,544 | 613,819 | -30% |
| **Cache write** | 55,414 | 55,910 | -1% |

## Analysis

### Efficiency Gains

Flow A (with terraphim) was **27% more token-efficient** and completed in **31% fewer steps**. This suggests:

1. **KG-boosted search helped** - The Frontend Developer role KG contains accessibility patterns that OpenCode used
2. **Better initial understanding** - terraphim search found relevant Svelte/accessibility examples faster
3. **Reduced exploration** - OpenCode didn't need to explore as many files to understand the project

### Code Quality

Both flows produced working components with:
- ARIA labels (`aria-label`, `aria-expanded`, `aria-controls`, `aria-current`)
- Keyboard navigation (Arrow keys, Escape, Tab)
- Mobile hamburger menu
- Dark mode with localStorage persistence
- TypeScript types

Flow A additionally identified and fixed:
- `aria-current` tracking keyboard focus instead of current page
- Missing skip link for screen readers
- Mobile menu positioning issues

### Cache Efficiency

Flow A had **30% less cache read** - the KG records likely provided more relevant context upfront, reducing redundant lookups.

## Session IDs

- Flow A: `ses_2178d0500ffeW6fFkdimKTqKtV`
- Flow B: `ses_2178b107bffeqI0FUaDc1IXNyh`

## Raw Data

Full JSON logs available at:
- `~/experiment/flow-a-full.json`
- `~/experiment/flow-b-full.json`

## Conclusion

**Hypothesis confirmed**: OpenCode with terraphim-agent produces results **27% faster** (token efficiency) with **31% fewer steps** for accessibility-focused frontend tasks.

The KG records for Frontend Developer provide targeted patterns for:
- WCAG compliance
- Svelte 5 best practices
- Accessibility (ARIA)
- Responsive design

This supports the value of role-based knowledge graphs for AI coding assistants.
