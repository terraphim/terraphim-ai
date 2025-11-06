import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import sveltePreprocess from 'svelte-preprocess'
import { fileURLToPath, URL } from "url";
import { writeFileSync, readFileSync, existsSync, mkdirSync } from 'fs';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    svelte({
      compilerOptions: {
        css: 'injected'
      },
      preprocess: sveltePreprocess({
        scss: true
      }),
      onwarn: (warning, handler) => {
        // Ignore dependency warnings
        if (warning.code === 'css_nesting_selector_invalid_placement') return;
        if (warning.code === 'css_invalid_global') return;
        if (warning.code === 'css_unused_selector') return;
        if (warning.code === 'css_invalid_identifier') return;
        if (warning.code === 'a11y_consider_explicit_label') return;
        if (warning.code === 'a11y_label_has_associated_control') return;
        if (warning.code === 'a11y_click_events_have_key_events') return;
        if (warning.code === 'a11y_missing_attribute') return;
        if (warning.code === 'a11y_missing_content') return;
        if (warning.code === 'node_invalid_placement') return;
        if (warning.code === 'node_invalid_placement_ssr') return;
        // Ignore Svelma-specific CSS issues
        if (warning.message && warning.message.includes('Tooltip.svelte')) return;
        if (warning.message && warning.message.includes('Expected a valid CSS identifier')) return;
        handler(warning);
      }
    }),
  ],
  resolve: {
    alias: {
      '$lib': fileURLToPath(new URL('./src/lib', import.meta.url)),
      '$workers': fileURLToPath(new URL('./src/workers', import.meta.url)),
      // Map specific Svelte sub-paths back to real runtime so they are **not** redirected to
      // our shim (which would cause ENOTDIR errors like svelte-shim.js/store).
      'svelte/internal': resolve(__dirname, 'node_modules/svelte/src/internal'),
      'svelte/store': resolve(__dirname, 'node_modules/svelte/src/store/index-client.js'),
      'svelte/transition': resolve(__dirname, 'node_modules/svelte/src/transition/index.js'),
      'svelte/animate': resolve(__dirname, 'node_modules/svelte/src/animate/index.js'),
      'svelte/easing': resolve(__dirname, 'node_modules/svelte/src/easing/index.js'),
      'svelte/motion': resolve(__dirname, 'node_modules/svelte/src/motion/index.js'),
      'svelte/reactivity': resolve(__dirname, 'node_modules/svelte/src/reactivity/index-client.js'),

      // Real runtime entry alias so that shim can import without causing an alias loop.
      'svelte-original': resolve(__dirname, 'node_modules/svelte/src/index-client.js'),
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Vendor libraries
          'vendor-ui': ['bulma', 'svelma', '@fortawesome/fontawesome-free'],
          'vendor-editor': ['@tiptap/core', '@tiptap/starter-kit', 'tiptap-markdown'],
          'vendor-charts': ['d3'],
          'vendor-atomic': ['@tomic/lib'],
          'vendor-utils': ['comlink-fetch', 'svelte-routing', 'tinro', 'svelte-markdown'],
          // Large components
          'novel-editor': ['@paralect/novel-svelte']
        }
      }
    },
    chunkSizeWarningLimit: 1000 // Increase limit to 1MB
  },
  server: {
    fs: {
      // Allow serving files from project root
      allow: ['..']
    }
  },
  optimizeDeps: {
    include: ['@tomic/lib', '@tomic/svelte']
  }
});
