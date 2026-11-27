import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			preprocess: vitePreprocess(),
			// Built as a single-page app and served by the Rust backend.
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	server: {
		// Forward API calls to the Rust backend during development.
		proxy: { '/api': process.env.API_URL ?? 'http://localhost:8080' }
	},
	test: { include: ['src/**/*.test.ts'] }
});
