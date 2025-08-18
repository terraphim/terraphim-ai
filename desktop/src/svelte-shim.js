// shim that re-exports Svelte runtime plus mount/unmount helpers expected by some
// third-party packages (e.g. @paralect/novel-svelte).
//
// It proxies every export from the real 'svelte' entry, then adds two helpers:
//   mount(Component, options)  – instantiates the component and returns it.
//   unmount(instance)          – calls $destroy() on the instance.
//
// This keeps our code compatible with packages built against newer Svelte
// versions that expose those helpers, while we still run on Svelte 3.
//
// Usage: the Vite config aliases 'svelte' to this file.

// @ts-expect-error – virtual alias provided via Vite's resolve.alias
export * from 'svelte-original';

/**
 * Create and mount a Svelte component programmatically.
 * Mirrors the helper available in newer Svelte builds.
 */
export function mount(Component, options) {
  // Allow calling with Component default export object
  return new Component(options);
}

/**
 * Destroy a Svelte component that was previously mounted with `mount`.
 */
export function unmount(component) {
  if (component && typeof component.$destroy === 'function') {
    component.$destroy();
  }
} 