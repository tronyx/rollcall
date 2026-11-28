<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { ApiToken, NewApiToken } from '#lib/api/types.ts';
	import { session } from '#lib/state/session.svelte.ts';
	import { dateTime } from '#lib/util/format.ts';
	import { onMount } from 'svelte';

	let tokens = $state<ApiToken[]>([]);
	let tokenName = $state('');
	let created = $state<NewApiToken | null>(null);
	let copied = $state(false);
	let confirmToken = $state<string | null>(null);

	let current = $state('');
	let next = $state('');
	let pwSaved = $state(false);

	let busy = $state(false);
	let error = $state<string | null>(null);

	const message = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);

	onMount(async () => {
		try {
			tokens = await api.tokens();
		} catch (e) {
			error = message(e, 'Could not load your API tokens.');
		}
	});

	async function changePassword(e: SubmitEvent) {
		e.preventDefault();
		busy = true;
		error = null;
		pwSaved = false;
		try {
			await api.changePassword(current, next);
			current = '';
			next = '';
			pwSaved = true;
		} catch (err) {
			error = message(err, 'Could not change the password.');
		} finally {
			busy = false;
		}
	}

	async function createToken(e: SubmitEvent) {
		e.preventDefault();
		if (!tokenName.trim()) return;
		busy = true;
		error = null;
		try {
			created = await api.createToken(tokenName.trim());
			tokens = [...tokens, created];
			tokenName = '';
			copied = false;
		} catch (err) {
			error = message(err, 'Could not create the token.');
		} finally {
			busy = false;
		}
	}

	async function deleteToken(id: string) {
		busy = true;
		error = null;
		try {
			await api.deleteToken(id);
			tokens = tokens.filter((t) => t.id !== id);
			if (created?.id === id) created = null;
			confirmToken = null;
		} catch (err) {
			error = message(err, 'Could not delete the token.');
		} finally {
			busy = false;
		}
	}

	async function copy() {
		if (!created) return;
		await navigator.clipboard?.writeText(created.secret);
		copied = true;
	}

	async function signOut() {
		await api.logout().catch(() => {});
		session.user = null;
		await goto(resolve('/login'));
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
	const button =
		'rounded-lg px-3 py-1.5 text-sm font-medium ring-1 ring-slate-300 hover:bg-slate-50 disabled:opacity-50 dark:ring-slate-700 dark:hover:bg-slate-800';
	const primary =
		'rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50';
</script>

<div class="flex flex-wrap items-center justify-between gap-2">
	<h2 class="font-semibold">Your account</h2>
	<button type="button" class={button} onclick={signOut}>Sign out</button>
</div>
<p class="mt-1 text-sm text-slate-600 dark:text-slate-400">
	Signed in as <strong>{session.user?.username}</strong>{session.user?.is_admin ? ' (admin)' : ''}.
</p>

<div class="mt-4 grid gap-6 md:grid-cols-2">
	<form onsubmit={changePassword} class="space-y-3">
		<h3 class="text-sm font-semibold">Change your password</h3>
		<label class="block">
			<span class={label}>Current password</span>
			<input
				type="password"
				bind:value={current}
				autocomplete="current-password"
				required
				class={input}
			/>
		</label>
		<label class="block">
			<span class={label}>New password</span>
			<input
				type="password"
				bind:value={next}
				autocomplete="new-password"
				required
				minlength="8"
				class={input}
			/>
		</label>
		<div class="flex items-center gap-3">
			<button type="submit" disabled={busy} class={primary}>Change password</button>
			{#if pwSaved}<span class="text-sm text-emerald-700 dark:text-emerald-400" role="status">
					Changed. Other devices are signed out.
				</span>{/if}
		</div>
	</form>

	<div class="space-y-3">
		<h3 class="text-sm font-semibold">API tokens</h3>
		<p class="text-xs text-slate-500 dark:text-slate-400">
			For scripts, such as the slicer's print logger (<code>--token</code>). A token can do anything
			your account can, so keep it private and delete ones you don't use.
		</p>
		{#if created}
			<div
				class="space-y-2 rounded-lg bg-emerald-50 p-3 text-sm text-emerald-900 dark:bg-emerald-950 dark:text-emerald-100"
				role="status"
			>
				<p>Copy the token for <strong>{created.name}</strong> now. It won't be shown again.</p>
				<div class="flex gap-2">
					<code
						class="min-w-0 flex-1 overflow-x-auto rounded bg-white px-2 py-1 text-xs whitespace-nowrap dark:bg-slate-900"
					>
						{created.secret}
					</code>
					<button type="button" class={button} onclick={copy}>{copied ? 'Copied' : 'Copy'}</button>
				</div>
			</div>
		{/if}
		{#if tokens.length}
			<ul class="divide-y divide-slate-200 text-sm dark:divide-slate-800">
				{#each tokens as t (t.id)}
					<li class="flex flex-wrap items-center justify-between gap-2 py-2">
						<div>
							<div class="font-medium">{t.name}</div>
							<div class="text-xs text-slate-500 dark:text-slate-400">
								{t.last_used_at ? `Last used ${dateTime(t.last_used_at)}` : 'Not used yet'}
							</div>
						</div>
						{#if confirmToken === t.id}
							<span class="flex gap-2">
								<button
									type="button"
									disabled={busy}
									class="rounded-lg bg-red-600 px-3 py-1.5 text-sm font-medium text-white"
									onclick={() => deleteToken(t.id)}
								>
									Delete
								</button>
								<button type="button" class={button} onclick={() => (confirmToken = null)}>
									Keep
								</button>
							</span>
						{:else}
							<button type="button" class={button} onclick={() => (confirmToken = t.id)}>
								Delete
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
		<form onsubmit={createToken} class="flex items-end gap-2">
			<label class="min-w-0 flex-1">
				<span class={label}>Token name</span>
				<input
					type="text"
					bind:value={tokenName}
					placeholder="OrcaSlicer on the desktop"
					class={input}
				/>
			</label>
			<button type="submit" disabled={busy || !tokenName.trim()} class={primary}>Create</button>
		</form>
	</div>
</div>
{#if error}<p class="mt-3 text-sm text-red-600" role="alert">{error}</p>{/if}
