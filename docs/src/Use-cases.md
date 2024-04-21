# Use cases for Terraphim AI

## Search your local markdown files, build your own personal knowledge graph, then publish your knowledge to the web, then reuse

### End to End flow
The current focus is on perfecting overall end-to-end human usable flow:
    - engineer or expert takes notes in Logseq knowledge graph
    - notes are used to create thesaurus for Terraphim (using Terraphim desktop, feature done, alpha testing)
    - Thesaurus is used per each role to create Terraphim graph (used for ranking search results). Feature done, alpha testing
    - Terraphim desktop is used to search over local markdown files using Terraphim Graph for ranking (Feature done, alpha testing)
    - Terraphim desktop search automatically populates Atomic server with search results (first attempt and approach didn't work, required changes in Atomic Rust client, atomic server-side updated with new client fetch, Terraphim feature for automatic cache population WIP https://github.com/terraphim/terraphim-ai/issues/12)
    - User works with search results forming blogs or articles in Atomic server using Atomic as standard CMS (all features done)
    - Final articles are published via different domains using Atomic Server + Sveltekit
        - current websites: https://systems.tf
        - planned set of websites on different topics: learning rust, knowledge graphs, search engines, Metacortex Engineering (cross-discipline collaboration)
    - Articles published (or drafts) in Atomic Server are used as input for Terraphim (both desktop and cloud) as haystack for the role (instead or alongside markdown files or any other sources) Blocked on https://github.com/atomicdata-dev/atomic-server/issues/778