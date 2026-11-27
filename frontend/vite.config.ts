import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		// Forward API calls to the Rust backend during development.
		proxy: { '/api': process.env.API_URL ?? 'http://localhost:8080' }
	},
	test: { include: ['src/**/*.test.ts'] }
});
