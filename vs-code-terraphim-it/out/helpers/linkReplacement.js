"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.replaceLinks = replaceLinks;
/** Link terms with the packaged Terraphim automata implementation. */
async function replaceLinks(text, thesaurus) {
    // Resolve an explicit packaged file; the generated package.json is not shipped.
    const modulePath = '../../rust-lib/pkg/terraphim_it.js';
    const engine = await Promise.resolve(`${modulePath}`).then(s => require(s));
    return engine.replace_links(text, thesaurus);
}
//# sourceMappingURL=linkReplacement.js.map