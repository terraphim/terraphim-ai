/** Link terms with the packaged Terraphim automata implementation. */
export async function replaceLinks(text: string, thesaurus: string): Promise<string> {
  // Resolve an explicit packaged file; the generated package.json is not shipped.
  const modulePath = '../../rust-lib/pkg/terraphim_it.js';
  const engine = await import(modulePath) as { replace_links(text: string, thesaurus: string): string };
  return engine.replace_links(text, thesaurus);
}
