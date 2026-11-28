<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { AuthStatus } from '#lib/api/types.ts';
	import { session } from '#lib/state/session.svelte.ts';
	import { safeNext } from '#lib/util/auth.ts';
	import { onMount } from 'svelte';

	let status = $state<AuthStatus | null>(null);
	let mode = $state<'login' | 'register'>('login');
	let username = $state('');
	let password = $state('');
	let confirm = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);

	const next = $derived(safeNext(page.url.searchParams.get('next')));
	const setup = $derived(status?.setup_needed ?? false);
	const registering = $derived(setup || mode === 'register');

	onMount(async () => {
		try {
			status = await api.authStatus();
			if (status.user) {
				session.user = status.user;
				await goto(next, { replaceState: true });
			}
		} catch (e) {
			error = e instanceof ApiError ? e.message : "Can't reach the Rollcall server.";
		}
	});

	function switchMode(m: 'login' | 'register') {
		mode = m;
		error = null;
		confirm = '';
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		error = null;
		if (registering && password !== confirm) {
			error = "The passwords don't match.";
			return;
		}
		busy = true;
		try {
			const body = { username: username.trim(), password };
			session.user = registering ? await api.register(body) : await api.login(body);
			await goto(next, { replaceState: true });
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Something went wrong. Try again.';
		} finally {
			busy = false;
		}
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

<svelte:head><title>{registering ? 'Create an account' : 'Sign in'} · Rollcall</title></svelte:head>

<div class="mx-auto mt-8 max-w-sm sm:mt-16">
	<div class="mb-6 flex flex-col items-center gap-3">
		<img src="/logo-mark.svg" alt="" class="h-16 w-auto" />
		<h1 class="text-2xl font-bold tracking-tight">Rollcall</h1>
	</div>

	{#if status}
		<form
			onsubmit={submit}
			class="space-y-4 rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800"
		>
			<div>
				<h2 class="font-semibold">
					{setup ? 'Create the admin account' : registering ? 'Create an account' : 'Sign in'}
				</h2>
				{#if setup}
					<p class="mt-1 text-sm text-slate-600 dark:text-slate-400">
						This is the first account, so it can manage the others and decide whether people can
						register.
					</p>
				{/if}
			</div>
			<label class="block">
				<span class={label}>Username</span>
				<input
					type="text"
					bind:value={username}
					autocomplete="username"
					autocapitalize="none"
					spellcheck="false"
					required
					maxlength="40"
					class={input}
				/>
			</label>
			<label class="block">
				<span class={label}>Password</span>
				<input
					type="password"
					bind:value={password}
					autocomplete={registering ? 'new-password' : 'current-password'}
					required
					minlength={registering ? 8 : undefined}
					class={input}
				/>
				{#if registering}
					<span class="mt-1 block text-xs text-slate-500 dark:text-slate-400">
						At least 8 characters.
					</span>
				{/if}
			</label>
			{#if registering}
				<label class="block">
					<span class={label}>Password again</span>
					<input
						type="password"
						bind:value={confirm}
						autocomplete="new-password"
						required
						class={input}
					/>
				</label>
			{/if}
			{#if error}<p class="text-sm text-red-600" role="alert">{error}</p>{/if}
			<button
				type="submit"
				disabled={busy}
				class="w-full rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
			>
				{busy ? 'One moment…' : registering ? 'Create account' : 'Sign in'}
			</button>
		</form>

		{#if !setup}
			<p class="mt-4 text-center text-sm text-slate-600 dark:text-slate-400">
				{#if mode === 'register'}
					Have an account?
					<button
						type="button"
						class="font-medium text-orange-700 hover:underline dark:text-orange-400"
						onclick={() => switchMode('login')}
					>
						Sign in
					</button>
				{:else if status.registration_enabled}
					New here?
					<button
						type="button"
						class="font-medium text-orange-700 hover:underline dark:text-orange-400"
						onclick={() => switchMode('register')}
					>
						Create an account
					</button>
				{:else}
					Need an account? Ask your Rollcall admin.
				{/if}
			</p>
		{/if}
	{:else if error}
		<p class="text-sm text-red-600" role="alert">{error}</p>
	{/if}
</div>
