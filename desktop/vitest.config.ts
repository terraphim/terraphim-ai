import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  test: {
    include: ['src/**/*.{test,spec}.{js,ts}'],
    globals: true,
    environment: 'jsdom',
    setupFiles: ['src/test-utils/setup.ts'],
    coverage: {
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/test-utils/',
        '**/*.d.ts',
        '**/*.config.*',
        'dist/',
        'build/'
      ]
    }
  },
  resolve: {
    alias: {
      '$lib': new URL('./src/lib', import.meta.url).pathname,
      '$app': new URL('./.svelte-kit/dev/runtime/app', import.meta.url).pathname
    }
  }
});
