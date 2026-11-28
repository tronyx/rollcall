<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { api, ApiError, handleSignedOut } from '#lib/api/client.ts';
	import { session } from '#lib/state/session.svelte.ts';
	import { onMount } from 'svelte';

	let { children } = $props();

	let checked = $state(false);
	let loadError = $state<string | null>(null);
	const onLogin = $derived(page.url.pathname === '/login');

	function toLogin() {
		session.user = null;
		if (page.url.pathname === '/login') return;
		const next = page.url.pathname + page.url.search;
		goto(`${resolve('/login')}?next=${encodeURIComponent(next)}`, { replaceState: true });
	}

	onMount(async () => {
		handleSignedOut(toLogin);
		try {
			session.user = (await api.authStatus()).user;
			if (!session.user) toLogin();
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : "Can't reach the Rollcall server.";
		} finally {
			checked = true;
		}
	});
</script>

<div class="mx-auto min-h-dvh max-w-6xl px-4 pt-4 pb-24 sm:px-6 print:max-w-none print:p-0">
	{#if loadError}
		<p
			class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
			role="alert"
		>
			{loadError}
		</p>
	{:else if checked && (session.user || onLogin)}
		{@render children()}
	{/if}
</div>
