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
          const outputPath = resolve(__dirname, 'dist/splashscreen.html');
          
          console.log(`Reading Svelte component from ${inputPath}`);
          const input = readFileSync(inputPath, 'utf8');
          
          console.log('Compiling Svelte component...');
          const { js, css, warnings } = compile(input, { generate: 'dom' });

          if (warnings.length) {
            warnings.forEach(warning => {
              console.warn(warning);
            });
          }

          console.log('Creating HTML content...');
          const html = `
            <!DOCTYPE html>
            <html lang="en">
            <head>
              <meta charset="UTF-8">
              <meta name="viewport" content="width=device-width, initial-scale=1.0">
              <title>Splash Screen</title>
              <style>${css.code}</style>
            </head>
            <body>
              <script>${js.code}</script>
            </body>
            </html>
          `;

          // Ensure the dist directory exists
          const distDir = resolve(__dirname, 'dist');
          if (!existsSync(distDir)) {
            console.log(`Creating directory ${distDir}`);
            mkdirSync(distDir);
          }

          console.log(`Writing HTML content to ${outputPath}`);
          writeFileSync(outputPath, html);
          
          // Verify the file was written
          if (existsSync(outputPath)) {
            console.log('splashscreen.html has been created successfully.');
          } else {
            console.error('Failed to create splashscreen.html.');
          }
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
  }
})