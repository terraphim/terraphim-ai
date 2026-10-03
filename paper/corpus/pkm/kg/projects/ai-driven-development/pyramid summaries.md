# Pyramid Summaries

Reversible summarization at multiple zoom levels -- compressing context while maintaining the ability to expand back to full detail. Inspired by multi-resolution image formats (Pyramid TIFF) and map tile systems (Google Maps). Core mechanism: successive summarization ('Summarize in 2 words. Now 4. Now 8. Now 16.'). Enables agents to rapidly enumerate many items at minimal detail, identify relevant ones, then selectively expand. Integrates with MapReduce: map (parallel summaries), cluster (group by compressed representations), reduce (synthesize across clusters expanding as needed). Key insight: 'Context windows are finite. Pyramid summaries let you see the forest and the trees, just not all at once.'

synonyms:: hierarchical summaries, multi-level summaries, summary pyramid, reversible summarization, zoom-level summaries, context window optimization, codebase navigation aid, multi-resolution context
