import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { fileURLToPath, URL } from "url";

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [sveltekit()],
	resolve: {
		alias: {
			'$lib': fileURLToPath(new URL('./src/lib', import.meta.url)),
			'$workers': fileURLToPath(new URL('./src/workers', import.meta.url)),
		},
	},
	build: {
		rollupOptions: {
			external: [
				// Tauri APIs - these are provided by the Tauri runtime
				'@tauri-apps/api/tauri',
				'@tauri-apps/api/dialog',
				'@tauri-apps/api/fs',
				'@tauri-apps/api/window',
				'@tauri-apps/api/app',
				'@tauri-apps/api/shell',
				// FontAwesome - served as static asset
				'@fortawesome/fontawesome-free',
				// TipTap - external dependencies
				'@tiptap/core',
				'@tiptap/starter-kit',
				'tiptap-markdown',
				// Other external packages
				'd3',
				'bulma',
				'svelte-jsoneditor',
				'@tomic/lib',
				'@paralect/novel-svelte',
				'comlink-fetch',
				'marked'
			],
			output: {
				// Let Vite handle chunking automatically to avoid external conflicts
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
		include: ['@tomic/lib', '@tomic/svelte'],
		exclude: [
			// Don't try to optimize Tauri APIs for dev mode
			'@tauri-apps/api/tauri',
			'@tauri-apps/api/dialog',
			'@tauri-apps/api/fs',
			'@tauri-apps/api/window',
			'@tauri-apps/api/app',
			'@tauri-apps/api/shell',
			// Exclude problematic packages
			'svelte-jsoneditor'
		]
	}
});
