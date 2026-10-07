Derived from Jason Fried's Write_On demo (x.com/jasonfried/status/2105403067793584590, posted 30 Sept 2026, 6:55 video). Every requirement below cites the evidence it comes from: `[T mm:ss]` is the spoken transcript, `[F mm:ss]` is a video frame. Items marked **inferred** are design decisions the demo does not show and that need confirmation.

Target codebase: `terraphim-editor` (Rust → WASM via Trunk, vanilla JS, Shoelace, `markdown` crate, Rinja templates). Current state: a textarea + live Markdown preview with formatting shortcuts, a `/` command palette and an experimental "Blocks" view. None of the features below exist yet.

Write_On is "alternative control, not version control" at the word, sentence and paragraph level, plus two supporting ideas: dimming text back without deleting it, and stashing text nearby. The author explicitly does not want AI rewriting his text; AI only proposes alternatives or marks candidates for cutting `[T 5:10–5:20]`.

Out of scope for this spec: collaboration, cloud sync, publishing to X/LinkedIn (the demo has buttons for these but they are incidental to the editing model `[T 3:50–4:05]`).

These are the most distinctive UI elements and must be reproduced precisely.

Menu is a dark rounded panel, monospace labels left, dim shortcut text right, hovered row slightly lighter.

From frames (approximate; sample before finalising): background `#0a0d1c`; body text warm cream `#e8d9c4`; dim/ghost text ≈ body at 10% opacity; accent lavender `#8c86e6` (active tab, lit dot, selected trim button border, save icon, word-count arrow); underline/dot colour ≈ accent at 40%; panels `#11142a` with 1px lighter border; context menu and Lab popover `#171a2e`, 8px radius. Body and UI in a monospace face (looks like JetBrains Mono / IBM Plex Mono); panel titles in a cursive display face. Line height ≈ 1.75.

Companion to `alternative-control.md` (the Write_On requirements) and `zed-plugin-fit.md`. Written 2026-10-05.

The gap is **ghosting (R-5.1)**: a highlighted region cannot change the colour of its text. A local spike on build 4215 tested four colour-scheme variants, including bright red, and all of them rendered at full body colour. So ghosting, and the Lab trim preview that depends on it (R-8.4), are **Partial**. They can be shown as a background tint or a stippled underline, not as text faded to about 10%. The only way to get real fading is a syntax-level scope, which needs markers in the buffer (see "Ghosting options").

`terraphim-editor` remains the only target where the full spec can be built as written. Sublime is the best existing editor host for a reduced version: it covers far more of the spec than Zed and keeps the same document format.

Sublime Text build 4215, macOS arm64. The API stub is the bundled `Contents/MacOS/Lib/python314/sublime.py` (a Python 3.14 plugin host, plus the legacy 3.3 host).

A throwaway package `Packages/WriteOnSpike` used a custom colour scheme (`bg #0a0d1c`, `fg #e8d9c4`, `accent #8c86e6`) on a scratch Markdown file.

`View.style_for_scope` does resolve the ghost rule (`foreground #20212d`). The colour scheme accepts the rule, but the renderer does not apply it to text in a highlighted region.

Key: **Full** = implementable as specified; **Partial** = an approximation; **None** = not possible.

Summary: §2, §6, §8.2/8.3/8.5–8.7 and §9 carry over in full. The context menus carry over in substance (native styling). §3 and §4 carry over approximately, with phantom rows changing line spacing. **§5 ghosting and §8.4 trim preview carry over only partly, because Sublime cannot fade text in a region.** §7.2 chrome does not carry over.

Unlike Zed, Sublime has no extension sandbox, so the engine can run in-process. The `terraphim-automata` 1.0.0 wheel on PyPI is `cp39-abi3-macosx_11_0_arm64`. It uses Python's stable ABI, so it loads in the 3.14 plugin host. It provides `load_thesaurus`, `build_index`, `find_all_matches` and the `AutocompleteIndex` class, which is enough for thesaurus alternatives (R-8.6) and the KG-list marks (R-8.2). This repeats the approach in `editors_research/editor_autocomplete_integration_options.md`.

Caveats: only a macOS arm64 wheel was confirmed. Other platforms need their own wheels, vendored into the package because Package Control does not install arbitrary PyPI wheels.

The **span model must not be rewritten in Python.** Re-anchoring, the a/an rule, the trim/ghost span set and the annotation block format belong in `terraphim_alternatives` (`terraphim/terraphim-editor#2`). The Sublime package should reach it in one of two ways:

(a) shares the most with Zed. (b) avoids running a server. Pick one when `#16` (extraction) is scheduled. Until then, a Sublime prototype can call the crate through a CLI.
