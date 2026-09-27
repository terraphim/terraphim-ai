# Content Bridge

The Content Bridge links DLT learning content back to the ODILO catalogue. Track A ingest populates `ContentReference.catalogue_url` from `ContentBridgeConfig` and the document Content ID, with a deployment selector (`ODILO_CONTENT_BRIDGE_CUSTOMER`, overriding `[content_bridge].customer`) choosing MTN or Odilo URL shapes; Content ID casing is preserved. Downstream, grounded Ask Alma responses attach `SourceChip` attribution (`content_id`, `title`, optional `chapter`/`section`) so learners can trace answers to catalogue sources; synthesis responses carry a source prefix and section metadata, and anchor content URLs point at specific catalogue items.

synonyms:: content bridge, catalogue url, source attribution, source chip, anchor content url, section metadata, content reference

## Related Concepts
- Alma
- Content Licensing Guard
- Embedding Pipeline
- Digital Learning Twin

## Sources
- `.agent/handoffs/2026-07-01-track-a-content-bridge-url.md` (ODITECH-692)
- `.agent/handoffs/2026-07-15-oditech-810-backend-source-attribution.md` (PR #910)
- `.agent/handoffs/2026-07-14-zes-453-alma-synthesis-source-prefix.md`
- `.agent/handoffs/2026-07-14-zes-453-content-bridge-section-metadata.md`
- `.agent/handoffs/2026-07-21-zes-468-anchor-content-urls.md`
