import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// Built as a single-page app and served by the Rust backend.
		adapter: adapter({ fallback: 'index.html' })
	}
};

export default config;
