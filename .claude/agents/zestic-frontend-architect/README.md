# Zestic Frontend Architect

**Role**: Strategic frontend architecture and system design for Terraphim AI Web Components migration

## Overview

The Zestic Frontend Architect is responsible for designing scalable, performant, and maintainable web application architectures using vanilla-first, no-build approaches following the Zestic AI Strategy.

## Core Principles

1. **No-Build, Vanilla-First**: HTML, CSS, and vanilla JavaScript (ES6+)
2. **Reusable Web Components**: Native Custom Elements API
3. **Strategic Rust/Wasm**: Only for computationally intensive tasks
4. **Performance First**: Browser-native features, optimal load times
5. **Zero Dependencies**: No frameworks, no build tools for UI layer

## Current Blueprints

### Phase 2.4: CSS Custom Properties Theme System

**Status**: ✅ Complete - Ready for Implementation
**File**: `phase-2.4-theme-system-blueprint.md`
**Date**: 2025-10-25

**Summary**: Comprehensive theming system using pure CSS custom properties to replace Svelte-based theme implementation. Features include:

- 200+ CSS custom properties for complete theme control
- Support for multiple themes (spacelab, light, dark, + 19 Bulmaswatch variants)
- FOUC prevention via inline critical CSS
- localStorage persistence
- Web Component theme switcher with accessibility
- TerraphimState integration
- Bulma CSS compatibility layer

**Key Deliverables**:
- `/components/styles/variables.css` - Root CSS custom properties
- `/components/styles/themes/*.css` - Theme files
- `/components/styles/theme-loader.js` - FOUC prevention script
- `/components/shell/terraphim-theme-switcher.js` - Theme switcher component

**Implementation Timeline**: 5 weeks
**Success Criteria**: Zero FOUC, <16ms theme switches, WCAG 2.1 AA compliance

## Architecture Decisions

### CSS Variables vs CSS-in-JS

**Decision**: Pure CSS Variables
**Rationale**:
- Native browser support (IE11+)
- Zero runtime overhead
- Inspectable in DevTools
- No JavaScript dependency for styling
- Simple mental model

### FOUC Prevention Strategy

**Decision**: Inline critical CSS in HTML head
**Rationale**:
- Synchronous theme loading before first paint
- No external script delays
- 100% FOUC elimination
- Minimal performance impact (~2KB inline)

### State Management Integration

**Decision**: TerraphimState for theme persistence
**Rationale**:
- Consistent with existing architecture
- Reactive updates across components
- Built-in localStorage persistence
- Event-driven pattern

### Component Architecture

**Decision**: Single Web Component for theme switching
**Rationale**:
- Encapsulated UI logic
- Reusable across application
- Shadow DOM isolation
- Accessible by default

## Design Patterns

### Two-Tier CSS Variable System

```css
/* Brand Colors (raw values) */
--brand-primary: #446e9b;

/* Semantic Colors (purpose-based) */
--color-primary: var(--brand-primary);
--button-bg: var(--color-primary);
```

**Benefits**:
- Easy theme creation (override brand colors)
- Semantic naming improves maintainability
- Flexible customization layers

### Theme Loading Sequence

1. Parse HTML
2. Load `variables.css`
3. Execute inline `theme-loader.js` (sync)
4. Apply theme via `data-theme` attribute
5. Load theme-specific CSS
6. Render page (themed from first paint)

**Result**: Zero FOUC, < 50ms to themed first paint

### Component Theming Pattern

```javascript
_getStyles() {
  return `
    :host {
      /* Inherit variables from document */
      color: var(--text-primary);
    }

    .element {
      background: var(--bg-surface);
      border-color: var(--border-default);
      transition: var(--transition-colors);
    }
  `;
}
```

## File Organization

```
.claude/agents/zestic-frontend-architect/
├── README.md                                    # This file
├── phase-2.4-theme-system-blueprint.md          # Theme system blueprint
└── [future blueprints]
```

## Usage

### For Implementation Teams

1. Read the relevant blueprint document
2. Follow the implementation roadmap
3. Use provided code examples exactly
4. Refer to success criteria for validation
5. Report any ambiguities back to architect

### For Stakeholders

Blueprints provide:
- Executive summaries for decision-making
- Technical specifications for feasibility review
- Timeline estimates for planning
- Success metrics for validation

## Quality Standards

All blueprints must include:

1. **Executive Summary**: High-level overview, key decisions, benefits
2. **Current State Analysis**: What exists, what needs to change
3. **Architecture Overview**: System design, file structure, components
4. **Technical Specifications**: Detailed APIs, code examples, patterns
5. **Implementation Roadmap**: Phased approach, deliverables, timeline
6. **Testing Strategy**: Test plans, coverage requirements, validation
7. **Migration Guide**: Step-by-step instructions for developers
8. **Performance Considerations**: Benchmarks, optimizations, targets
9. **Accessibility Checklist**: WCAG compliance, screen readers, keyboard
10. **Troubleshooting Guide**: Common issues, solutions, debug tools

## Collaboration

### Working with @zestic-front-craftsman

The craftsman implements blueprints created by the architect:

- **Architect** → Designs system, creates blueprint
- **Craftsman** → Implements exactly as specified, reports issues
- **Architect** → Updates blueprint based on feedback

### Working with Other Roles

- **Backend Team**: API contracts, data shape requirements
- **UX Team**: Component behavior, interaction patterns
- **QA Team**: Test specifications, acceptance criteria

## Contact

For questions about architectural decisions or blueprint clarifications:
- Refer to blueprint documents first
- Check troubleshooting sections
- Review code examples and patterns
- Escalate ambiguities for blueprint updates

---

**Last Updated**: 2025-10-25
**Version**: 1.0
**Status**: Active
