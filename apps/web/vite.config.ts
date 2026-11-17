import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const apiTarget = process.env.API_PROXY_TARGET ?? 'http://127.0.0.1:8080';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// Built as a static SPA and served by the Rust API in production. The landing pages
			// are prerendered to index.html and nl/index.html; every other route uses 200.html.
			adapter: adapter({ fallback: '200.html' })
		})
	],
	server: {
		port: 5173,
		strictPort: true,
		proxy: {
			'/api': { target: apiTarget, changeOrigin: false }
		}
	}
});
