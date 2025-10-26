import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
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
      onwarn: (warning, handler) => {
        // Ignore svelma warnings
        if (warning.code === 'css_nesting_selector_invalid_placement') return;
        if (warning.code === 'css_invalid_global') return;
        if (warning.code === 'css_expected_identifier' && warning.filename?.includes('svelma')) return;

        // Ignore third-party library warnings
        if (warning.filename?.includes('svelte-jsoneditor')) return;
        if (warning.code === 'node_invalid_placement') return;
        if (warning.code === 'node_invalid_placement_ssr') return;
        if (warning.code === 'a11y_click_events_have_key_events') return;
        if (warning.code === 'a11y_no_static_element_interactions') return;
        if (warning.code === 'a11y_consider_explicit_label') return;
        if (warning.code === 'a11y_label_has_associated_control') return;
        if (warning.code === 'element_invalid_self_closing_tag') return;

        handler(warning);
      },
      preprocess: {
        style: ({ content, filename }) => {
          // Handle problematic CSS nesting in svelma
          if (filename && filename.includes('svelma')) {
            let processedContent = content.replace(/:global\(&\[([^\]]+)\]\)/g, '&[$1]');

            // Fix Tooltip SCSS variable declaration issue
            if (filename && filename.includes('Tooltip.svelte')) {
              processedContent = processedContent.replace(
                /@use '[^']*\/bulma\/sass\/utilities\/initial-variables' as \*;\s*\$tooltip-arrow-size: 8px;\s*\$tooltip-arrow-margin: 2px;/g,
                `$tooltip-arrow-size: 8px;\n  $tooltip-arrow-margin: 2px;\n  @use '../../../../bulma/sass/utilities/initial-variables' as *;`
              );
            }

            return {
              code: processedContent
            };
          }
          return { code: content };
        }
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
          'vendor-ui': ['bulma', '@fortawesome/fontawesome-free'],
          'vendor-editor': ['svelte-jsoneditor', '@tiptap/core', '@tiptap/starter-kit', 'tiptap-markdown'],
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
