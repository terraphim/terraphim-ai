/**
 * VanillaMarkdownEditor - Pure JavaScript ContentEditable-based Markdown Editor
 *
 * A zero-dependency markdown editor using ContentEditable for editing and
 * optional preview rendering. Provides cursor tracking, text insertion,
 * and basic formatting toolbar.
 *
 * @class VanillaMarkdownEditor
 * @fires change - Emitted when content changes
 * @fires keydown - Emitted on keydown events
 * @example
 * const editor = new VanillaMarkdownEditor({
 *   content: '# Hello World',
 *   readOnly: false,
 *   showPreview: false
 * });
 * document.body.appendChild(editor.render());
 * editor.addEventListener('change', (event) => {
 *   console.log('Content:', event.detail.content);
 * });
 */
export class VanillaMarkdownEditor extends EventTarget {
  /**
   * Create editor instance
   * @param {Object} options - Configuration options
   * @param {string} options.content - Initial markdown content
   * @param {boolean} options.readOnly - Enable read-only mode
   * @param {boolean} options.showPreview - Show preview pane
   * @param {boolean} options.showToolbar - Show formatting toolbar
   */
  constructor(options = {}) {
    super();

    this.options = {
      content: '',
      readOnly: false,
      showPreview: false,
      showToolbar: true,
      ...options
    };

    this.element = null;
    this.contentElement = null;
    this.previewElement = null;
    this.toolbarElement = null;

    this._suppressChangeEvent = false;
    this._mutationObserver = null;
  }

  /**
   * Render the editor and return the root element
   * @returns {HTMLElement} The editor root element
   */
  render() {
    if (this.element) {
      return this.element;
    }

    this.element = document.createElement('div');
    this.element.className = 'vanilla-markdown-editor';

    if (this.options.showToolbar && !this.options.readOnly) {
      this.toolbarElement = this._createToolbar();
      this.element.appendChild(this.toolbarElement);
    }

    const editorContainer = document.createElement('div');
    editorContainer.className = 'editor-container';

    this.contentElement = this._createContentElement();
    editorContainer.appendChild(this.contentElement);

    if (this.options.showPreview) {
      this.previewElement = this._createPreviewElement();
      editorContainer.appendChild(this.previewElement);
    }

    this.element.appendChild(editorContainer);

    this._attachEventListeners();
    this._setupMutationObserver();

    return this.element;
  }

  /**
   * Get current markdown content
   * @returns {string} Current markdown content
   */
  getContent() {
    if (!this.contentElement) {
      return this.options.content;
    }
    return this.contentElement.textContent || '';
  }

  /**
   * Set markdown content
   * @param {string} content - New markdown content
   */
  setContent(content) {
    this.options.content = content;
    if (this.contentElement) {
      this._suppressChangeEvent = true;
      this.contentElement.textContent = content;
      this._suppressChangeEvent = false;

      if (this.previewElement) {
        this._updatePreview();
      }
    }
  }

  /**
   * Insert text at current cursor position
   * @param {string} text - Text to insert
   */
  insertTextAtCursor(text) {
    if (!this.contentElement || this.options.readOnly) {
      return;
    }

    this.contentElement.focus();

    const selection = window.getSelection();
    if (!selection.rangeCount) {
      this.contentElement.textContent += text;
      return;
    }

    const range = selection.getRangeAt(0);
    range.deleteContents();

    const textNode = document.createTextNode(text);
    range.insertNode(textNode);

    range.setStartAfter(textNode);
    range.setEndAfter(textNode);
    selection.removeAllRanges();
    selection.addRange(range);

    this._emitChangeEvent();
  }

  /**
   * Get current cursor position as character offset
   * @returns {number} Character offset from start
   */
  getCursorPosition() {
    if (!this.contentElement) {
      return 0;
    }

    const selection = window.getSelection();
    if (!selection.rangeCount) {
      return 0;
    }

    const range = selection.getRangeAt(0);
    const preCaretRange = range.cloneRange();
    preCaretRange.selectNodeContents(this.contentElement);
    preCaretRange.setEnd(range.endContainer, range.endOffset);

    return preCaretRange.toString().length;
  }

  /**
   * Get cursor coordinates relative to viewport
   * @returns {Object} Coordinates object { top, left, bottom, right }
   */
  getCursorCoordinates() {
    if (!this.contentElement) {
      return { top: 0, left: 0, bottom: 0, right: 0 };
    }

    const selection = window.getSelection();
    if (!selection.rangeCount) {
      const rect = this.contentElement.getBoundingClientRect();
      return {
        top: rect.top,
        left: rect.left,
        bottom: rect.top,
        right: rect.left
      };
    }

    const range = selection.getRangeAt(0);
    const rect = range.getBoundingClientRect();

    return {
      top: rect.top,
      left: rect.left,
      bottom: rect.bottom,
      right: rect.right
    };
  }

  /**
   * Set cursor position to character offset
   * @param {number} offset - Character offset from start
   */
  setCursorPosition(offset) {
    if (!this.contentElement) {
      return;
    }

    const selection = window.getSelection();
    const range = document.createRange();

    let currentOffset = 0;
    let found = false;

    const setPosition = (node) => {
      if (found) return;

      if (node.nodeType === Node.TEXT_NODE) {
        const textLength = node.textContent.length;
        if (currentOffset + textLength >= offset) {
          range.setStart(node, offset - currentOffset);
          range.setEnd(node, offset - currentOffset);
          found = true;
          return;
        }
        currentOffset += textLength;
      } else {
        for (let i = 0; i < node.childNodes.length; i++) {
          setPosition(node.childNodes[i]);
          if (found) return;
        }
      }
    };

    setPosition(this.contentElement);

    if (found) {
      selection.removeAllRanges();
      selection.addRange(range);
    }
  }

  /**
   * Destroy editor and clean up resources
   */
  destroy() {
    if (this._mutationObserver) {
      this._mutationObserver.disconnect();
      this._mutationObserver = null;
    }

    if (this.element && this.element.parentNode) {
      this.element.parentNode.removeChild(this.element);
    }

    this.element = null;
    this.contentElement = null;
    this.previewElement = null;
    this.toolbarElement = null;
  }

  // Private methods

  _createToolbar() {
    const toolbar = document.createElement('div');
    toolbar.className = 'editor-toolbar';

    const buttons = [
      { name: 'bold', label: 'B', title: 'Bold (Ctrl+B)', action: () => this._formatBold() },
      { name: 'italic', label: 'I', title: 'Italic (Ctrl+I)', action: () => this._formatItalic() },
      { name: 'code', label: 'Code', title: 'Inline Code', action: () => this._formatCode() },
      { name: 'link', label: 'Link', title: 'Insert Link', action: () => this._formatLink() },
      { name: 'heading', label: 'H', title: 'Heading', action: () => this._formatHeading() },
      { name: 'list', label: 'List', title: 'Bullet List', action: () => this._formatList() }
    ];

    buttons.forEach(btn => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = `toolbar-button toolbar-button-${btn.name}`;
      button.textContent = btn.label;
      button.title = btn.title;
      button.addEventListener('click', (e) => {
        e.preventDefault();
        btn.action();
      });
      toolbar.appendChild(button);
    });

    return toolbar;
  }

  _createContentElement() {
    const content = document.createElement('div');
    content.className = 'editor-content';
    content.contentEditable = !this.options.readOnly;
    content.spellcheck = true;
    content.textContent = this.options.content;

    if (this.options.readOnly) {
      content.setAttribute('aria-readonly', 'true');
    }

    return content;
  }

  _createPreviewElement() {
    const preview = document.createElement('div');
    preview.className = 'editor-preview';
    this._updatePreview();
    return preview;
  }

  _attachEventListeners() {
    if (!this.contentElement) {
      return;
    }

    this.contentElement.addEventListener('input', () => {
      this._emitChangeEvent();
      if (this.previewElement) {
        this._updatePreview();
      }
    });

    this.contentElement.addEventListener('keydown', (event) => {
      this._handleKeydown(event);

      const customEvent = new CustomEvent('keydown', {
        detail: {
          key: event.key,
          ctrlKey: event.ctrlKey,
          metaKey: event.metaKey,
          shiftKey: event.shiftKey,
          altKey: event.altKey,
          originalEvent: event
        }
      });
      this.dispatchEvent(customEvent);
    });
  }

  _setupMutationObserver() {
    if (!this.contentElement) {
      return;
    }

    this._mutationObserver = new MutationObserver(() => {
      if (!this._suppressChangeEvent) {
        this._emitChangeEvent();
      }
    });

    this._mutationObserver.observe(this.contentElement, {
      childList: true,
      characterData: true,
      subtree: true
    });
  }

  _handleKeydown(event) {
    if (event.ctrlKey || event.metaKey) {
      switch (event.key.toLowerCase()) {
        case 'b':
          event.preventDefault();
          this._formatBold();
          break;
        case 'i':
          event.preventDefault();
          this._formatItalic();
          break;
      }
    }

    if (event.key === 'Tab') {
      event.preventDefault();
      this.insertTextAtCursor('  ');
    }
  }

  _formatBold() {
    this._wrapSelection('**', '**', 'bold text');
  }

  _formatItalic() {
    this._wrapSelection('*', '*', 'italic text');
  }

  _formatCode() {
    this._wrapSelection('`', '`', 'code');
  }

  _formatLink() {
    this._wrapSelection('[', '](url)', 'link text');
  }

  _formatHeading() {
    const selection = window.getSelection();
    if (!selection.rangeCount) {
      return;
    }

    const range = selection.getRangeAt(0);
    const startContainer = range.startContainer;

    let textNode = startContainer.nodeType === Node.TEXT_NODE
      ? startContainer
      : startContainer.childNodes[0];

    if (!textNode) {
      this.insertTextAtCursor('# ');
      return;
    }

    const text = textNode.textContent;
    const lineStart = text.lastIndexOf('\n', range.startOffset - 1) + 1;

    const beforeCursor = text.substring(0, range.startOffset);
    const afterCursor = text.substring(range.startOffset);
    const linePrefix = text.substring(lineStart, range.startOffset);

    if (linePrefix.startsWith('# ')) {
      return;
    }

    const newText = beforeCursor.substring(0, lineStart) + '# ' + linePrefix + afterCursor;
    textNode.textContent = newText;

    this.setCursorPosition(this.getCursorPosition() + 2);
    this._emitChangeEvent();
  }

  _formatList() {
    const selection = window.getSelection();
    if (!selection.rangeCount) {
      return;
    }

    const range = selection.getRangeAt(0);
    const startContainer = range.startContainer;

    let textNode = startContainer.nodeType === Node.TEXT_NODE
      ? startContainer
      : startContainer.childNodes[0];

    if (!textNode) {
      this.insertTextAtCursor('- ');
      return;
    }

    const text = textNode.textContent;
    const lineStart = text.lastIndexOf('\n', range.startOffset - 1) + 1;

    const beforeCursor = text.substring(0, range.startOffset);
    const afterCursor = text.substring(range.startOffset);
    const linePrefix = text.substring(lineStart, range.startOffset);

    if (linePrefix.startsWith('- ')) {
      return;
    }

    const newText = beforeCursor.substring(0, lineStart) + '- ' + linePrefix + afterCursor;
    textNode.textContent = newText;

    this.setCursorPosition(this.getCursorPosition() + 2);
    this._emitChangeEvent();
  }

  _wrapSelection(prefix, suffix, placeholder) {
    const selection = window.getSelection();
    if (!selection.rangeCount) {
      this.insertTextAtCursor(prefix + placeholder + suffix);
      return;
    }

    const range = selection.getRangeAt(0);
    const selectedText = range.toString();

    const text = selectedText || placeholder;
    const wrappedText = prefix + text + suffix;

    range.deleteContents();
    const textNode = document.createTextNode(wrappedText);
    range.insertNode(textNode);

    if (!selectedText) {
      const newRange = document.createRange();
      newRange.setStart(textNode, prefix.length);
      newRange.setEnd(textNode, prefix.length + placeholder.length);
      selection.removeAllRanges();
      selection.addRange(newRange);
    }

    this._emitChangeEvent();
  }

  _emitChangeEvent() {
    if (this._suppressChangeEvent) {
      return;
    }

    const event = new CustomEvent('change', {
      detail: {
        content: this.getContent()
      }
    });
    this.dispatchEvent(event);
  }

  _updatePreview() {
    if (!this.previewElement) {
      return;
    }

    const content = this.getContent();
    this.previewElement.innerHTML = this._renderMarkdownPreview(content);
  }

  _renderMarkdownPreview(markdown) {
    let html = markdown;

    html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>');
    html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>');
    html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>');

    html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');

    html = html.replace(/`(.+?)`/g, '<code>$1</code>');

    html = html.replace(/\[(.+?)\]\((.+?)\)/g, '<a href="$2">$1</a>');

    html = html.replace(/^- (.+)$/gim, '<li>$1</li>');
    html = html.replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>');

    html = html.replace(/\n/g, '<br>');

    return html;
  }
}
