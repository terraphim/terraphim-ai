'use strict';
const KNOWN_GROUPS = new Set(['plain-chrome', 'word-indicators', 'live-first-key', 'kg-provenance', 'grammar-only', 'granularities', 'native-format-shortcuts', 'remaining-panel-provenance', 'remaining-heading-indicators', 'exact-native-geometry', 'lab-positive', 'trim-keep-walk-cuts', 'ghost-context-and-overflow', 'overflow-typing-isolated', 'native-pull-drag-clipboard', 'native-clipboard-only', 'ghost-layer-advanced', 'empty-indicator-ghost', 'utf16-anchors', 'remaining-chrome-controls', 'context-hover-style', 'external-context-reanchor']);
function nativeScenarioPreflight(env) {
 const overflowOnly=env.TERRAPHIM_MAC_OVERFLOW_ONLY==='1',all41=env.TERRAPHIM_MAC_ALL41==='1';
 const groups=(env.TERRAPHIM_SCENARIO_GROUPS||'').split(',').map(x=>x.trim()).filter(Boolean);
 if(overflowOnly && (groups.length!==1||groups[0]!=='overflow-typing-isolated'))throw new Error('Native preflight refused: Overflow-only mode requires exactly TERRAPHIM_SCENARIO_GROUPS=overflow-typing-isolated before application launch');
 if(all41 && groups.length===0)throw new Error('Native preflight refused: finite all41 runs require explicit TERRAPHIM_SCENARIO_GROUPS before application launch');
 if(!overflowOnly&&!all41&&groups.length)throw new Error('Native preflight refused: scenario groups require the complete all41 fixture or explicit Overflow-only mode');
 if((overflowOnly||all41)&&!env.TERRAPHIM_MATRIX_TEMPLATE)throw new Error('Native preflight refused: finite scenarios require TERRAPHIM_MATRIX_TEMPLATE before application launch');
 if(groups.some(x=>!KNOWN_GROUPS.has(x)))throw new Error('Native preflight refused: unknown scenario group');
 if(new Set(groups).size!==groups.length)throw new Error('Native preflight refused: duplicate scenario group');
 return {mode:overflowOnly?'overflow-only':all41?'finite-all41':'native-core',groups};
}
module.exports={nativeScenarioPreflight};
