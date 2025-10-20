import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath, URL } from "url";
import { writeFileSync, readFileSync, existsSync, mkdirSync } from 'fs';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    svelte(),
    {
      name: 'splashscreen',
      async writeBundle() {
        try {
          console.log('Starting splashscreen build...');
          const { compile } = await import('svelte/compiler');
          const inputPath = resolve(__dirname, 'src/lib/StartupScreen.svelte');
          const jsOutputPath = resolve(__dirname, 'dist/splashscreen.js');
          const htmlOutputPath = resolve(__dirname, 'dist/splashscreen.html');

          console.log(`Reading Svelte component from ${inputPath}`);
          const input = readFileSync(inputPath, 'utf8');

          console.log('Compiling Svelte component...');
          const { js, css } = compile(input, {
            generate: 'dom',
            format: 'esm',
            name: 'SplashScreen'
          });

          console.log(`Writing JavaScript content to ${jsOutputPath}`);
          writeFileSync(jsOutputPath, js.code);

          console.log('Creating HTML content...');
          const html = `
          <!DOCTYPE html>
          <html lang="en">
          <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Splash Screen</title>
            <style>
              /* Add your CSS styles here */
              body {
                font-family: Arial, sans-serif;
                background-color: #f0f0f0;
              }
              .splash-container {
                display: flex;
                justify-content: center;
                align-items: center;
                height: 100vh;
              }
              .splash-content {
                text-align: center;
              }
            </style>
          </head>
          <body>
            <div class="splash-container">
              <div class="splash-content">
                <!-- Your Svelte component content goes here -->
              </div>
            </div>
            <script type="module">
              import SplashScreen from './splashscreen.js'; // Import the compiled Svelte component
              const target = document.querySelector('.splash-content');
              new SplashScreen({
                target
              });
            </script>
          </body>
          `;

          console.log(`Writing HTML content to ${htmlOutputPath}`);
          if (!existsSync(resolve(__dirname, 'src-tauri'))) {
            mkdirSync(resolve(__dirname, 'src-tauri'));
          }
          writeFileSync(htmlOutputPath, html);

          console.log('splashscreen.html has been created successfully.');
        } catch (error) {
          console.error('Error during splashscreen build:', error);
        }
      }
    }
  ],
  resolve: {
    alias: {
      '$lib': fileURLToPath(new URL('./src/lib', import.meta.url)),
      '$workers': fileURLToPath(new URL('./src/workers', import.meta.url)),

      // Map specific Svelte sub-paths back to the real runtime so they are **not** redirected to
      // our shim (which would cause ENOTDIR errors like svelte-shim.js/store).
      // Note: Order matters! More specific paths must come first.
      'svelte/internal/disclose-version': resolve(__dirname, 'node_modules/svelte/src/runtime/internal/disclose-version/index.js'),
      'svelte/internal': resolve(__dirname, 'node_modules/svelte/src/runtime/internal/index.js'),
      'svelte/store': resolve(__dirname, 'node_modules/svelte/src/runtime/store/index.js'),
      'svelte/transition': resolve(__dirname, 'node_modules/svelte/src/runtime/transition/index.js'),
      'svelte/animate': resolve(__dirname, 'node_modules/svelte/src/runtime/animate/index.js'),
      'svelte/easing': resolve(__dirname, 'node_modules/svelte/src/runtime/easing/index.js'),
      'svelte/motion': resolve(__dirname, 'node_modules/svelte/src/runtime/motion/index.js'),

      // Real runtime entry alias so the shim can import without causing an alias loop.
      'svelte-original': resolve(__dirname, 'node_modules/svelte/src/runtime/index.js'),

      // Any other bare `import "svelte"` should go to our shim that adds mount/unmount.
      'svelte': fileURLToPath(new URL('./src/svelte-shim.js', import.meta.url)),
    }
  },
  clearScreen: false,
  server: {
    proxy: {
      '/rolegraph': 'http://localhost:8000',
      '/documents': 'http://localhost:8000',
    }
  },
  css: {
    preprocessorOptions: {
      scss: {
        // Suppress all warnings from third-party dependencies
        quietDeps: true,
        // Silence deprecation warnings for all known categories
        silenceDeprecations: ['legacy-js-api', 'import', 'global-builtin', 'color-functions', 'mixed-decls'],
        // Add verbose flag to control output
        verbose: false
      }
    }
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Vendor libraries
          'vendor-ui': ['bulma', 'svelma', '@fortawesome/fontawesome-free'],
          'vendor-editor': ['svelte-jsoneditor', '@tiptap/core', '@tiptap/starter-kit', 'tiptap-markdown'],
          'vendor-charts': ['d3'],
          'vendor-atomic': ['@tomic/lib', '@tomic/svelte'],
          'vendor-utils': ['comlink-fetch', 'svelte-routing', 'tinro', 'svelte-markdown']
        }
      }
    },
    chunkSizeWarningLimit: 1000 // Increase limit to 1MB
  }
})
