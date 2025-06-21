<script lang="ts">
  import { Editor as NovelEditor } from '@paralect/novel-svelte';
  export let html: any = '';          // initial content in HTML/JSON
  export let readOnly: boolean = false;

  /** Handler called by Novel editor on every update; we translate it to the
   *  wrapper's `html` variable so the parent can bind to it. */
  const handleUpdate = (editor) => {
    // TipTap editor → HTML string (you may want JSON later)
    html = editor.getHTML();
  }
</script>

<NovelEditor
  defaultValue={html}
  isEditable={!readOnly}
  onUpdate={handleUpdate}
/>
