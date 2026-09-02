# Demo: how training and brain switching change MegaHAL's text.
# Regenerate with:  vhs crates/terraphim_megahal/docs/demo/megahal-brain-training-demo.tape
# (from the repository root; requires a debug build with the `personalities`
# feature: cargo build -p terraphim_megahal --bin megahal --features personalities)

Output crates/terraphim_megahal/docs/demo/megahal-brain-training-demo.gif

Set FontSize 18
Set Width 1200
Set Height 780
Set TypingSpeed 60ms
Set Margin 40
Set Theme { "background": "#10141a", "foreground": "#d7dee9", "accent": "#4fb3ff" }

Type "MEGAHAL_SEED=42 ./target/debug/megahal"
Enter
Sleep 1200ms

Type "Hello there."
Enter
Sleep 1400ms

Type "Tell me about Terraphim."
Enter
Sleep 1800ms

# Teach the brain something new...
Type "/train crates/terraphim_megahal/docs/demo/train-terraphim.txt"
Enter
Sleep 1200ms

# ...and ask the same question again: the answer now draws on the new corpus.
Type "Tell me about Terraphim."
Enter
Sleep 1800ms

# Switch to a different brain entirely.
Type "/personality startrek"
Enter
Sleep 1000ms

Type "Mr Picard, report to the bridge."
Enter
Sleep 1800ms

# Snapshot the evolved brain.
Type "/save docs/demo/brain-final.json"
Enter
Sleep 1200ms

Type "/quit"
Enter
Sleep 800ms
