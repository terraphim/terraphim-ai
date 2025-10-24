declare module "tinro" {
  import type { SvelteComponentTyped } from "svelte";

  export class Route extends SvelteComponentTyped<
    { path?: string; redirect?: string },
    Record<string, never>,
    { default: {} }
  > {}

  export const router: {
    goto(path: string, options?: { replace?: boolean }): void;
  };

  export function active(
    node: Element,
    options?: { exact?: boolean }
  ): { destroy(): void };
}
