import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
// import { viteStaticCopy } from 'vite-plugin-static-copy'
import { fileURLToPath, URL } from "url";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    // viteStaticCopy({
    //   targets: [
    //     {
    //       src: '../../terraphim-ui-svelte-ts/src',
    //       dest: 'src'
    //     }
    //   ]
    // }),
    svelte()
  ],
    resolve: {
    alias: {
      '$lib': fileURLToPath(new URL('./src/lib', import.meta.url)),
      '$workers': fileURLToPath(new URL('./src/workers', import.meta.url)),
    }
  }
})
