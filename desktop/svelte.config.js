import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'
import sveltePreprocess from 'svelte-preprocess'

export default {
  // Combine Vite defaults with our legacy preprocessing pipeline
  preprocess: [
    vitePreprocess(),
    sveltePreprocess()
  ]
}
