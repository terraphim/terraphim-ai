<script lang="ts">
  import { Editor as NovelEditor } from '@paralect/novel-svelte';
  import { Markdown } from 'tiptap-markdown';
  
  export let html: any = '';          // initial content in HTML/JSON
  export let readOnly: boolean = false;
  export let outputFormat: 'html' | 'markdown' = 'html';  // New prop to control output format

  /** Handler called by Novel editor on every update; we translate it to the
   *  wrapper's `html` variable so the parent can bind to it. */
  const handleUpdate = (editor) => {
    // Choose output format based on the outputFormat prop
    // For markdown content, use getMarkdown() to preserve markdown syntax
    // For HTML content, use getHTML() to preserve rich formatting
    if (outputFormat === 'markdown') {
      html = editor.storage.markdown.getMarkdown();
    } else {
      html = editor.getHTML();
    }
  }
</script>

<NovelEditor
  defaultValue={html}
  isEditable={!readOnly}
  disableLocalStorage={true}
  onUpdate={handleUpdate}
  extensions={[Markdown]}
/>
