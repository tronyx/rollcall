<script lang="ts">
	import { resolve } from '$app/paths';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { AppSettings, Location, Printer } from '#lib/api/types.ts';
	import AccountSettings from '#lib/components/AccountSettings.svelte';
	import LocationsSettings from '#lib/components/LocationsSettings.svelte';
	import PrintersSettings from '#lib/components/PrintersSettings.svelte';
	import UsersSettings from '#lib/components/UsersSettings.svelte';
	import { session } from '#lib/state/session.svelte.ts';
	import { onMount } from 'svelte';

	let settings = $state<AppSettings | null>(null);
	let locations = $state<Location[]>([]);
	let printers = $state<Printer[]>([]);
	let loadError = $state<string | null>(null);
	let saving = $state(false);
	let saved = $state(false);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			[settings, locations, printers] = await Promise.all([
				api.settings(),
				api.locations(),
				api.printers()
			]);
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : 'Could not load the settings.';
		}
	});

	async function saveGeneral(e: SubmitEvent) {
		e.preventDefault();
		if (!settings) return;
		saving = true;
		saved = false;
		error = null;
		try {
			settings = await api.updateSettings({
				low_stock_threshold_g: Number(settings.low_stock_threshold_g) || 0,
				default_currency: settings.default_currency
			});
			saved = true;
		} catch (err) {
			error =
				err instanceof ApiError && err.status === 422
					? 'Use a threshold of 0 or more, and a three-letter currency code such as USD.'
					: err instanceof ApiError
						? err.message
						: 'Could not save.';
		} finally {
			saving = false;
		}
	}

	const card =
		'rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800';
	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

<svelte:head><title>Settings · Rollcall</title></svelte:head>

<nav class="mb-4">
	<a
		href={resolve('/')}
		class="text-sm font-medium text-orange-700 hover:underline dark:text-orange-400"
	>
		← All spools
	</a>
</nav>
<h1 class="mb-4 text-xl font-bold tracking-tight">Settings</h1>

{#if loadError}
	<p
		class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
		role="alert"
	>
		{loadError}
	</p>
{:else if !settings}
	<p class="py-12 text-center text-slate-500">Loading settings…</p>
{:else}
	<div class="grid gap-4 lg:grid-cols-2">
		<section class="{card} lg:col-span-2">
			<h2 class="mb-2 font-semibold">General</h2>
			<form onsubmit={saveGeneral} class="flex flex-wrap items-end gap-3">
				<label class="w-48">
					<span class={label}>Low-stock alert below (g)</span>
					<input
						type="number"
						min="0"
						step="1"
						bind:value={settings.low_stock_threshold_g}
						class={input}
					/>
				</label>
				<label class="w-32">
					<span class={label}>Default currency</span>
					<input
						type="text"
						maxlength="3"
						bind:value={settings.default_currency}
						class="{input} uppercase"
					/>
				</label>
				<button
					type="submit"
					disabled={saving}
					class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
				>
					{saving ? 'Saving…' : 'Save'}
				</button>
				{#if saved}<span class="py-2 text-sm text-emerald-700 dark:text-emerald-400" role="status">
						Saved.
					</span>{/if}
			</form>
			<p class="mt-2 text-xs text-slate-500 dark:text-slate-400">
				The currency is used for new spools that don't name one.
			</p>
			{#if error}<p class="mt-2 text-sm text-red-600" role="alert">{error}</p>{/if}
		</section>

		<section class={card}>
			<LocationsSettings bind:locations />
		</section>
		<section class={card}>
			<PrintersSettings bind:printers />
		</section>
		<section class="{card} lg:col-span-2">
			<AccountSettings />
		</section>
		{#if session.user?.is_admin}
			<section class="{card} lg:col-span-2">
				<UsersSettings />
			</section>
		{/if}
	</div>
{/if}
