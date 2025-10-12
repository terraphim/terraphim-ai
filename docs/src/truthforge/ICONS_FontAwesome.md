# Font Awesome Icon Reference: TruthForge UI

**Version**: 1.0
**Date**: 2025-10-08
**Font Awesome Version**: 6.5.1 (Classic)
**CDN**: `https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.1/css/all.min.css`

---

## Overview

This document provides a comprehensive reference for all Font Awesome icons used in the TruthForge Two-Pass Debate Arena UI. All icons are from the Font Awesome Classic pack.

### CDN Integration

Add to HTML `<head>`:
```html
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.1/css/all.min.css" />
```

---

## Primary UI Components

### Header & Branding

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| 🔍 | `fas fa-search` | Main application logo | Header title |
| 📄 | `fas fa-file-alt` | Document/text input | Narrative input section |

**Example**:
```html
<h1><i class="fas fa-search"></i> TruthForge - Two-Pass Debate Arena</h1>
<i class="fas fa-file-alt"></i>
<textarea placeholder="Paste your text here..."></textarea>
```

---

## Input & Controls

### Action Buttons

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| ▶️ | `fas fa-play-circle` | Start analysis | Primary action button |
| 🎛️ | `fas fa-sliders-h` | Configuration toggles | Context settings |
| 📋 | `fas fa-copy` | Copy to clipboard | Results export |

**Example**:
```html
<button id="analyze-btn">
    <i class="fas fa-play-circle"></i> Analyze
</button>
<i class="fas fa-sliders-h"></i> Context Toggles
```

---

## Progress Tracking

### Phase Indicators

| Icon | Class | Phase | Context |
|------|-------|-------|---------|
| 🔄 | `fas fa-spinner fa-pulse` | Pass 1 (In Progress) | Animated loading |
| ⚖️ | `fas fa-balance-scale` | Debate Phase | Pass 1 debate |
| 🎯 | `fas fa-crosshairs` | Pass 2 Exploitation | Targeted analysis |
| 💡 | `fas fa-lightbulb` | Response Generation | Solution phase |
| 📊 | `fas fa-tasks` | Overall Progress | Progress header |

**Example**:
```html
<h2><i class="fas fa-tasks"></i> Analysis Progress</h2>
<h3><i class="fas fa-search"></i> Phase 1: Initial Analysis</h3>
<h3><i class="fas fa-balance-scale"></i> Phase 2: Debate</h3>
<h3><i class="fas fa-crosshairs"></i> Phase 3: Exploitation</h3>
<h3><i class="fas fa-lightbulb"></i> Phase 4: Response Generation</h3>
```

### Progress Status

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| ⬇️ | `fas fa-arrow-down` | Flow direction | Between phases |
| ✅ | `fas fa-check-circle` | Phase complete | Success state |
| ❌ | `fas fa-times-circle` | Phase failed | Error state |

---

## Results Display

### Key Findings Icons

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| ⭐ | `fas fa-star` | Executive briefing | Top priority content |
| 🔍 | `fas fa-search` | Key findings header | Main results section |
| ⚠️ | `fas fa-exclamation-triangle` | Bias detection | Warning indicator |
| 🏷️ | `fas fa-tag` | SCCT classification | Category label |
| 👁️‍🗨️ | `fas fa-eye-slash` | Omissions detected | Missing content |

**Example**:
```html
<h3><i class="fas fa-star"></i> Executive Briefing (TOP)</h3>
<h3><i class="fas fa-search"></i> Key Findings</h3>
<div class="finding">
    <i class="fas fa-exclamation-triangle"></i> Bias patterns
</div>
<div class="finding">
    <i class="fas fa-tag"></i> SCCT classification
</div>
<div class="finding">
    <i class="fas fa-eye-slash"></i> Top 5 omissions
</div>
```

### Playbook & Taxonomy

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| 📚 | `fas fa-book` | Playbooks section | Strategic guides |
| 🛡️ | `fas fa-shield-alt` | Crisis management | Protection strategy |
| ✅ | `fas fa-tasks` | Risk assessment | Task checklist |

**Example**:
```html
<h3><i class="fas fa-book"></i> Playbooks to Use</h3>
<div class="playbook">
    <i class="fas fa-shield-alt"></i> Issue & Crisis Management
</div>
<div class="playbook">
    <i class="fas fa-tasks"></i> Risk Assessment, Response
</div>
```

---

## Debate Arena

### Debate Visualization

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| 💬 | `fas fa-comments` | Debate arena header | Discussion section |
| 👍 | `fas fa-thumbs-up` | Supporting argument | Pro position |
| 👎 | `fas fa-thumbs-down` | Opposing argument | Con position |
| 💬 | `fas fa-comment-dots` | Pass 1 debate | Initial discussion |
| 🎯 | `fas fa-bullseye` | Pass 2 exploitation | Targeted attack |
| 📊 | `fas fa-chart-bar` | Comparison view | Pass 1 vs Pass 2 |
| 🔥 | `fas fa-fire` | Vulnerability heatmap | High-risk areas |

**Example**:
```html
<h3><i class="fas fa-comments"></i> Debate Arena</h3>
<div class="debate-pass1">
    <i class="fas fa-comment-dots"></i> PASS 1: Initial Debate
    <div><i class="fas fa-thumbs-up"></i> Supporting: 72%</div>
    <div><i class="fas fa-thumbs-down"></i> Opposing: 68%</div>
    <div><i class="fas fa-eye-slash"></i> Omissions: 12 found</div>
</div>
<div class="debate-pass2">
    <i class="fas fa-crosshairs"></i> PASS 2: Exploitation
    <div><i class="fas fa-shield-alt"></i> Supporting: 58%</div>
    <div><i class="fas fa-bullseye"></i> Opposing: 84%</div>
    <div><i class="fas fa-fire"></i> Vulnerability: HIGH</div>
</div>
```

---

## Omission Cards

### Risk Level Indicators

| Icon | Class | Risk Level | Color | Context |
|------|-------|-----------|-------|---------|
| ❗ | `fas fa-exclamation-circle` | High (≥0.7) | Red | Critical omission |
| ⚠️ | `fas fa-exclamation-triangle` | Medium (≥0.5) | Orange | Important omission |
| ℹ️ | `fas fa-info-circle` | Low (<0.5) | Yellow | Notable omission |

### Omission Details

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| 🏷️ | `fas fa-tag` | Category label | Omission type |
| 💬 | `fas fa-comment-dots` | Description | Explanation text |
| 💭 | `fas fa-quote-left` | Text reference | Original quote |
| ➕ | `fas fa-plus-circle` | Suggestion | Recommended addition |

**Example**:
```javascript
const riskIcon = omission.composite_risk >= 0.7 ?
                 '<i class="fas fa-exclamation-circle"></i>' :
                 omission.composite_risk >= 0.5 ?
                 '<i class="fas fa-exclamation-triangle"></i>' :
                 '<i class="fas fa-info-circle"></i>';

return `
    <div class="omission-card">
        <div class="omission-header">
            <span class="category">
                <i class="fas fa-tag"></i> ${omission.category}
            </span>
            <span class="risk-score">
                ${riskIcon} ${risk}%
            </span>
        </div>
        <div class="omission-description">
            <i class="fas fa-comment-dots"></i> ${omission.description}
        </div>
        <div class="omission-reference">
            <i class="fas fa-quote-left"></i> "${omission.text_reference}"
        </div>
        <div class="omission-suggestion">
            <i class="fas fa-plus-circle"></i>
            <strong>Suggested addition:</strong> ${omission.suggested_addition}
        </div>
    </div>
`;
```

---

## Strategic Responses

### Response Strategy Types

| Icon | Class | Strategy | Context |
|------|-------|----------|---------|
| 🔄 | `fas fa-sync-alt` | Reframe | Shift narrative context |
| ⚖️ | `fas fa-gavel` | Counter-Argue | Direct rebuttal |
| 🤝 | `fas fa-handshake` | Bridge | Collaborative approach |
| 💬 | `fas fa-reply-all` | Responses header | All strategies |

**Example**:
```html
<h3><i class="fas fa-reply-all"></i> Strategic Responses</h3>
<div class="response-tabs">
    <button class="tab active">
        <i class="fas fa-sync-alt"></i> Reframe
    </button>
    <button class="tab">
        <i class="fas fa-gavel"></i> Counter-Argue
    </button>
    <button class="tab">
        <i class="fas fa-handshake"></i> Bridge
    </button>
</div>
```

---

## Analysis Results

### Results Overview

| Icon | Class | Usage | Context |
|------|-------|-------|---------|
| 📈 | `fas fa-chart-line` | Analysis results header | Main results |
| 🎯 | `fas fa-bullseye` | Detected omissions | Gap analysis |
| 🔥 | `fas fa-fire` | Vulnerability assessment | Risk evaluation |
| 💬 | `fas fa-reply-all` | Recommended responses | Action items |

---

## CSS Styling Guidelines

### Icon Sizing

```css
/* Base icon size */
.fas {
    font-size: 1rem;
}

/* Header icons */
h1 .fas {
    font-size: 1.5rem;
    margin-right: 0.5rem;
}

h2 .fas {
    font-size: 1.25rem;
    margin-right: 0.5rem;
}

h3 .fas {
    font-size: 1.1rem;
    margin-right: 0.5rem;
}

/* Inline icons */
.category .fas,
.risk-score .fas {
    font-size: 0.9rem;
    margin-right: 0.25rem;
}
```

### Icon Colors

```css
/* Status colors */
.fas.fa-check-circle {
    color: #28a745; /* Green - success */
}

.fas.fa-times-circle {
    color: #dc3545; /* Red - error */
}

.fas.fa-exclamation-triangle {
    color: #ffc107; /* Yellow - warning */
}

.fas.fa-info-circle {
    color: #17a2b8; /* Blue - info */
}

/* Risk level colors */
.risk-red .fas.fa-exclamation-circle {
    color: #dc3545;
}

.risk-orange .fas.fa-exclamation-triangle {
    color: #fd7e14;
}

.risk-yellow .fas.fa-info-circle {
    color: #ffc107;
}

/* Phase-specific colors */
.phase-1 .fas {
    color: #007bff; /* Blue - analysis */
}

.phase-2 .fas {
    color: #6f42c1; /* Purple - debate */
}

.phase-3 .fas {
    color: #dc3545; /* Red - exploitation */
}

.phase-4 .fas {
    color: #28a745; /* Green - response */
}
```

### Animation

```css
/* Spinner animation */
.fas.fa-spinner.fa-pulse {
    animation: fa-spin 1s infinite steps(8);
}

@keyframes fa-spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

/* Hover effects */
button .fas {
    transition: transform 0.2s ease;
}

button:hover .fas {
    transform: scale(1.1);
}
```

---

## Complete Icon Reference

### Alphabetical List

| Icon | Class | Primary Usage |
|------|-------|---------------|
| ⬇️ | `fas fa-arrow-down` | Flow direction |
| ⚖️ | `fas fa-balance-scale` | Debate phase |
| 📚 | `fas fa-book` | Playbooks |
| 🎯 | `fas fa-bullseye` | Pass 2 targeting |
| 📊 | `fas fa-chart-bar` | Comparison view |
| 📈 | `fas fa-chart-line` | Results header |
| ✅ | `fas fa-check-circle` | Success status |
| 💬 | `fas fa-comment-dots` | Descriptions/Pass 1 |
| 💬 | `fas fa-comments` | Debate arena |
| 📋 | `fas fa-copy` | Copy button |
| 🎯 | `fas fa-crosshairs` | Pass 2 phase |
| ❗ | `fas fa-exclamation-circle` | High risk |
| ⚠️ | `fas fa-exclamation-triangle` | Medium risk/Bias |
| 👁️‍🗨️ | `fas fa-eye-slash` | Omissions |
| 📄 | `fas fa-file-alt` | Text input |
| 🔥 | `fas fa-fire` | Vulnerability |
| ⚖️ | `fas fa-gavel` | Counter-argue |
| 🤝 | `fas fa-handshake` | Bridge strategy |
| ℹ️ | `fas fa-info-circle` | Low risk/Info |
| 💡 | `fas fa-lightbulb` | Response phase |
| ▶️ | `fas fa-play-circle` | Start analysis |
| ➕ | `fas fa-plus-circle` | Suggestions |
| 💭 | `fas fa-quote-left` | Text quotes |
| 💬 | `fas fa-reply-all` | Responses |
| 🔍 | `fas fa-search` | Logo/Key findings |
| 🛡️ | `fas fa-shield-alt` | Protection/Defense |
| 🎛️ | `fas fa-sliders-h` | Settings |
| 🔄 | `fas fa-spinner fa-pulse` | Loading (animated) |
| ⭐ | `fas fa-star` | Executive briefing |
| 🔄 | `fas fa-sync-alt` | Reframe strategy |
| 🏷️ | `fas fa-tag` | Category labels |
| ✅ | `fas fa-tasks` | Progress/Tasks |
| 👍 | `fas fa-thumbs-up` | Supporting argument |
| 👎 | `fas fa-thumbs-down` | Opposing argument |
| ❌ | `fas fa-times-circle` | Error status |

---

## Implementation Checklist

- [x] Add Font Awesome CDN to `<head>`
- [x] Replace emoji icons with `<i class="fas fa-*"></i>` tags
- [x] Apply consistent icon sizing classes
- [x] Implement risk-level color coding
- [x] Add hover animations for interactive elements
- [x] Test icon rendering across browsers
- [x] Verify accessibility (screen reader compatibility)
- [x] Document custom icon usage in style guide

---

## Browser Compatibility

Font Awesome 6.5.1 Classic supports:
- ✅ Chrome (latest 2 versions)
- ✅ Firefox (latest 2 versions)
- ✅ Safari (latest 2 versions)
- ✅ Edge (latest 2 versions)
- ✅ Mobile browsers (iOS Safari, Chrome Mobile)

---

## Accessibility Notes

### Screen Reader Support

Font Awesome icons are decorative by default. For semantic icons, add `aria-label`:

```html
<!-- Decorative icon (visual only) -->
<i class="fas fa-search" aria-hidden="true"></i> Search Results

<!-- Semantic icon (conveys meaning) -->
<button>
    <i class="fas fa-play-circle" aria-label="Start analysis"></i>
    <span class="sr-only">Start analysis</span>
</button>
```

### Best Practices

1. **Always include text labels** alongside icons for clarity
2. **Use `aria-hidden="true"`** for purely decorative icons
3. **Add `aria-label`** for icon-only buttons
4. **Provide text alternatives** for critical information
5. **Test with screen readers** (NVDA, JAWS, VoiceOver)

---

**Document Status**: Final v1.0
**Last Updated**: 2025-10-08
**Maintainer**: Frontend Team
