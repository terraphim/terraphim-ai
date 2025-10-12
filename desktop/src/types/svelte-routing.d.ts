// Type definitions for svelte-routing
declare module 'svelte-routing' {
  import type { SvelteComponentTyped } from 'svelte';

  export class Route extends SvelteComponentTyped<{
    path?: string;
    component?: any;
  }> {}

  export class Router extends SvelteComponentTyped<{
    url?: string;
    basepath?: string;
    primary?: boolean;
  }> {}

  export class Link extends SvelteComponentTyped<{
    to: string;
    replace?: boolean;
    state?: any;
    getProps?: (props: any) => any;
  }> {}

  export function navigate(to: string, options?: { replace?: boolean; state?: any }): void;
  export function link(node: HTMLElement): { destroy(): void };
}
