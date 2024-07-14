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
    }
  },
  clearScreen: false,
})