declare module "svelte-jsoneditor" {
  import type { Readable } from "svelte/store";
  import type { SvelteComponentTyped } from "svelte";

  export interface JsonContent {
    json: unknown;
  }

  export interface TextContent {
    text: string;
  }

  export type Content = JsonContent | TextContent;

  export type OnChange = (content: Content, previousContent: Content) => void;

  export type JSONEditorProps = {
    content: Content;
    onChange?: OnChange;
    readOnly?: boolean;
    statusBar?: boolean;
    navigationBar?: boolean;
    mainMenuBar?: boolean;
  };

  export class JSONEditor extends SvelteComponentTyped<
    JSONEditorProps,
    Record<string, never>,
    { default: {} }
  > {}
}
