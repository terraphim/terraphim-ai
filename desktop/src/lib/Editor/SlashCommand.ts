import { Extension } from '@tiptap/core';
import { Suggestion, type SuggestionOptions } from '@tiptap/suggestion';
import { PluginKey } from 'prosemirror-state';
import tippy, { type Instance, type Props } from 'tippy.js';

type CommandItem = {
  title: string;
  subtitle?: string;
  icon?: string;
  run: (ctx: { editor: any }) => void;
};

const DEFAULT_ITEMS: CommandItem[] = [
  {
    title: 'Paragraph',
    icon: '¶',
    run: ({ editor }) => editor.chain().focus().setParagraph().run(),
  },
  {
    title: 'Heading 1',
    icon: 'H1',
    run: ({ editor }) => editor.chain().focus().toggleHeading({ level: 1 }).run(),
  },
  {
    title: 'Heading 2',
    icon: 'H2',
    run: ({ editor }) => editor.chain().focus().toggleHeading({ level: 2 }).run(),
  },
  {
    title: 'Heading 3',
    icon: 'H3',
    run: ({ editor }) => editor.chain().focus().toggleHeading({ level: 3 }).run(),
  },
  {
    title: 'Bullet List',
    icon: '•',
    run: ({ editor }) => editor.chain().focus().toggleBulletList().run(),
  },
  {
    title: 'Ordered List',
    icon: '1.',
    run: ({ editor }) => editor.chain().focus().toggleOrderedList().run(),
  },
  {
    title: 'Blockquote',
    icon: '❝',
    run: ({ editor }) => editor.chain().focus().toggleBlockquote().run(),
  },
  {
    title: 'Code Block',
    icon: '</>',
    run: ({ editor }) => editor.chain().focus().toggleCodeBlock().run(),
  },
  {
    title: 'Horizontal Rule',
    icon: '—',
    run: ({ editor }) => editor.chain().focus().setHorizontalRule().run(),
  },
];

export interface SlashCommandOptions {
  trigger: string;
  pluginKey: PluginKey;
  items: CommandItem[];
}

export const SlashCommand = Extension.create<SlashCommandOptions>({
  name: 'slashCommand',

  addOptions() {
    return {
      trigger: '/',
      pluginKey: new PluginKey('slashCommand'),
      items: DEFAULT_ITEMS,
    };
  },

  addProseMirrorPlugins() {
    const suggestion: Partial<SuggestionOptions> = {
      editor: this.editor,
      char: this.options.trigger,
      pluginKey: this.options.pluginKey,
      allowSpaces: true,
      startOfLine: true,

      command: ({ editor, range, props }) => {
        const item = props as CommandItem;
        // Remove the typed "/..." text first
        editor.chain().focus().deleteRange(range).run();
        // Execute the command
        item.run({ editor });
      },

      items: ({ query }) => {
        const q = query.toLowerCase();
        return this.options.items.filter((i) =>
          i.title.toLowerCase().includes(q)
        );
      },

      render: () => {
        let component: SlashRenderer;
        let popup: Instance<Props>;

        return {
          onStart: (props) => {
            component = new SlashRenderer({
              items: props.items as CommandItem[],
              onSelect: (item) => props.command(item),
            });

            if (!props.clientRect) return;

            popup = tippy('body', {
              getReferenceClientRect: props.clientRect as () => DOMRect,
              appendTo: () => document.body,
              content: component.element,
              showOnCreate: true,
              interactive: true,
              trigger: 'manual',
              placement: 'bottom-start',
              theme: 'slash-command',
              maxWidth: 'none',
            })[0];
          },

          onUpdate(props) {
            component?.updateItems(props.items as CommandItem[]);

            if (!props.clientRect) return;
            popup?.setProps({
              getReferenceClientRect: props.clientRect as () => DOMRect,
            });
          },

          onKeyDown(props) {
            if (props.event.key === 'Escape') {
              popup?.hide();
              return true;
            }
            return component?.onKeyDown(props) ?? false;
          },

          onExit() {
            popup?.destroy();
            component?.destroy();
          },
        };
      },
    };

    return [Suggestion(suggestion)];
  },
});

class SlashRenderer {
  public element: HTMLElement;
  private items: CommandItem[] = [];
  private selectedIndex = 0;
  private onSelect: (item: CommandItem) => void;

  constructor(options: { items: CommandItem[]; onSelect: (item: CommandItem) => void }) {
    this.items = options.items;
    this.onSelect = options.onSelect;
    this.element = document.createElement('div');
    this.element.className = 'slash-dropdown';
    this.render();
  }

  updateItems(items: CommandItem[]) {
    this.items = items;
    this.selectedIndex = 0;
    this.render();
  }

  onKeyDown({ event }: { event: KeyboardEvent }): boolean {
    if (event.key === 'ArrowUp') {
      this.selectPrevious();
      return true;
    }
    if (event.key === 'ArrowDown') {
      this.selectNext();
      return true;
    }
    if (event.key === 'Enter' || event.key === 'Tab') {
      this.selectItem(this.selectedIndex);
      return true;
    }
    return false;
  }

  private selectPrevious() {
    this.selectedIndex = Math.max(0, this.selectedIndex - 1);
    this.render();
  }

  private selectNext() {
    this.selectedIndex = Math.min(this.items.length - 1, this.selectedIndex + 1);
    this.render();
  }

  private selectItem(index: number) {
    const item = this.items[index];
    if (item) this.onSelect(item);
  }

  private render() {
    this.element.innerHTML = '';

    if (this.items.length === 0) {
      this.element.innerHTML = `
        <div class="slash-item slash-empty">No commands</div>
      `;
      return;
    }

    this.items.forEach((item, index) => {
      const el = document.createElement('div');
      el.className = `slash-item ${index === this.selectedIndex ? 'slash-selected' : ''}`;
      el.innerHTML = `
        <div class="slash-icon">${item.icon ?? ''}</div>
        <div class="slash-text">
          <div class="slash-title">${this.escape(item.title)}</div>
          ${item.subtitle ? `<div class="slash-subtitle">${this.escape(item.subtitle)}</div>` : ''}
        </div>
      `;
      el.addEventListener('click', () => this.selectItem(index));
      el.addEventListener('mouseenter', () => {
        this.selectedIndex = index;
        this.render();
      });
      this.element.appendChild(el);
    });
  }

  private escape(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  destroy() {
    this.element.remove();
  }
}

export const slashCommandStyles = `
.slash-dropdown {
  background: white;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 10px 38px -10px rgba(22, 23, 24, 0.35), 0 10px 20px -15px rgba(22, 23, 24, 0.2);
  overflow: hidden;
  min-width: 260px;
  max-height: 320px;
  overflow-y: auto;
  z-index: 1000;
}
.slash-item {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 8px 12px;
  cursor: pointer;
  border-bottom: 1px solid #f1f5f9;
}
.slash-item:last-child {
  border-bottom: none;
}
.slash-item:hover {
  background: #f8fafc;
}
.slash-selected {
  background: #eff6ff !important;
  border-left: 3px solid #3b82f6;
}
.slash-icon {
  width: 24px;
  text-align: center;
  color: #64748b;
  font-size: 12px;
}
.slash-title {
  font-weight: 600;
  color: #1f2937;
}
.slash-subtitle {
  font-size: 12px;
  color: #64748b;
}
@media (prefers-color-scheme: dark) {
  .slash-dropdown {
    background: #1e293b;
    border-color: #334155;
  }
  .slash-item {
    border-bottom-color: #334155;
  }
  .slash-item:hover {
    background: #334155;
  }
  .slash-selected {
    background: #1e40af !important;
  }
  .slash-title { color: #f1f5f9; }
  .slash-subtitle { color: #94a3b8; }
  .slash-icon { color: #94a3b8; }
}
`;
