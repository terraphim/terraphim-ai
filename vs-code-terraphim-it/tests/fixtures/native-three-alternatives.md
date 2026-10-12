Pass me a thumbtack.

Keep this sentence. Ghost this aside.

Move this paragraph to Overflow.



```terraphim-alternatives
{
  "version": 1,
  "spans": [
    {
      "id": "s1",
      "kind": "word",
      "anchor": {
        "start": 10,
        "end": 19,
        "text": "thumbtack",
        "before": "Pass me a ",
        "after": ".\n\nKeep this sentence. Ghost "
      },
      "active": 2,
      "alts": [
        {
          "text": "paperclip",
          "source": "original"
        },
        {
          "text": "eraser",
          "source": "human"
        },
        {
          "text": "thumbtack",
          "source": "human"
        }
      ]
    }
  ],
  "ghosts": [
    {
      "id": "g1",
      "anchor": {
        "start": 42,
        "end": 59,
        "text": "Ghost this aside.",
        "before": "thumbtack.\n\nKeep this sentence. ",
        "after": "\n\nMove this paragraph to "
      }
    }
  ],
  "overflow": "Move this paragraph to Overflow."
}
```
