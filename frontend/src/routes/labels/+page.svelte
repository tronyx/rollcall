<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Filament, Spool } from '#lib/api/types.ts';
	import ColorSwatch from '#lib/components/ColorSwatch.svelte';
	import SpoolLabel from '#lib/components/SpoolLabel.svelte';
	import {
		LABEL_SIZES,
		labelable,
		labelSize,
		spoolLink,
		validBase,
		type LabelLayout
	} from '#lib/util/label.ts';
	import { onMount } from 'svelte';

	// The print settings are remembered in this browser only.
	const STORE_KEY = 'rollcall.labels';

	let spools = $state<Spool[]>([]);
	let filaments = $state<Record<string, Filament>>({});
	let selected = $state<string[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let pickerOpen = $state(true);
	let q = $state('');

	let sizeId = $state(LABEL_SIZES[0].id);
	let layout = $state<LabelLayout>('printer');
	let base = $state('');

	onMount(() => {
		base = window.location.origin;
		try {
			const saved = JSON.parse(localStorage.getItem(STORE_KEY) ?? '{}');
			if (typeof saved.sizeId === 'string') sizeId = labelSize(saved.sizeId).id;
			if (saved.layout === 'printer' || saved.layout === 'sheet') layout = saved.layout;
			if (typeof saved.base === 'string' && validBase(saved.base)) base = saved.base;
		} catch {
			// Private windows and blocked storage just use the defaults.
		}
		load();
	});

	$effect(() => {
		const settings = JSON.stringify({ sizeId, layout, base });
		try {
			localStorage.setItem(STORE_KEY, settings);
		} catch {
			// Not saved; the labels still print.
		}
	});

	async function load() {
		try {
			const all = await api.spools({ include_empty: true });
			spools = all;
			const wanted = (page.url.searchParams.get('ids') ?? '').split(',').filter(Boolean);
			if (wanted.length) {
				selected = all.filter((s) => wanted.includes(s.id)).map((s) => s.id);
				pickerOpen = selected.length === 0;
			} else {
				selected = labelable(all).map((s) => s.id);
			}
			error = null;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load spools.';
		} finally {
			loading = false;
		}
	}

	const chosen = $derived(spools.filter((s) => selected.includes(s.id)));

	// Fetches each chosen filament once, for the print temperatures.
	$effect(() => {
		const missing = [...new Set(chosen.map((s) => s.filament_id))].filter((id) => !filaments[id]);
		if (!missing.length) return;
		Promise.all(missing.map((id) => api.filament(id).catch(() => null))).then((found) => {
			const next = { ...filaments };
			for (const f of found) if (f) next[f.id] = f;
			filaments = next;
		});
	});

	const size = $derived(labelSize(sizeId));
	const baseOk = $derived(validBase(base));
	const isLocal = $derived(/^https?:\/\/(localhost|127\.|\[::1\])/i.test(base.trim()));

	const shown = $derived.by(() => {
		const needle = q.trim().toLowerCase();
		if (!needle) return spools;
		return spools.filter((s) =>
			[s.color_name, s.brand_name, s.material_code, s.product_name, s.label_code, s.location_name]
				.filter(Boolean)
				.some((v) => v!.toLowerCase().includes(needle))
		);
	});

	function toggle(id: string) {
		selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id];
	}

	function selectShown(on: boolean) {
		const ids = new Set(shown.map((s) => s.id));
		selected = on ? [...new Set([...selected, ...ids])] : selected.filter((id) => !ids.has(id));
	}

	// One label per page at the label's size for label printers; a grid with
	// cut lines on ordinary paper.
	const pageStyle = $derived(
		layout === 'printer'
			? `<style>@page { size: ${size.width_mm}mm ${size.height_mm}mm; margin: 0; }
				@media print { .label-sheet { display: block; } .label-sheet > :not(:last-child) { break-after: page; } }</style>`
			: `<style>@page { margin: 10mm; }
				@media print { .label-sheet .spool-label { outline: 0.2mm dashed #94a3b8; } }</style>`
	);

	const card =
		'rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800';
	const input =
		'w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'mb-1 block text-xs font-medium text-slate-600 dark:text-slate-400';
</script>

<svelte:head>
	<title>Labels · Rollcall</title>
	<!-- The @page rules change with the chosen size and layout. -->
	{@html pageStyle}
</svelte:head>

<div class="space-y-4 print:hidden">
	<nav>
		<a
			href={resolve('/')}
			class="text-sm font-medium text-orange-700 hover:underline dark:text-orange-400"
		>
			← All spools
		</a>
	</nav>

	<header class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="text-xl font-bold tracking-tight">🏷️ Labels</h1>
		<button
			type="button"
			onclick={() => window.print()}
			disabled={!chosen.length || !baseOk}
			class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-orange-700 disabled:opacity-50"
		>
			🖨️ Print {chosen.length}
			{chosen.length === 1 ? 'label' : 'labels'}
		</button>
	</header>

	<section class="{card} grid gap-3 sm:grid-cols-3">
		<div>
			<label class={label} for="label-size">Label size</label>
			<select id="label-size" bind:value={sizeId} class={input}>
				{#each LABEL_SIZES as s (s.id)}
					<option value={s.id}>{s.name}</option>
				{/each}
			</select>
		</div>
		<div>
			<label class={label} for="label-layout">Print on</label>
			<select id="label-layout" bind:value={layout} class={input}>
				<option value="printer">A label printer, one label per page</option>
				<option value="sheet">Paper, as a sheet to cut out</option>
			</select>
		</div>
		<div>
			<label class={label} for="label-base">QR codes open</label>
			<input
				id="label-base"
				type="url"
				bind:value={base}
				class="{input} font-mono"
				placeholder="http://rollcall.local:8080"
			/>
		</div>
		{#if !baseOk}
			<p class="text-sm text-red-700 sm:col-span-3 dark:text-red-400" role="alert">
				Enter the address you open Rollcall at, starting with http:// or https://.
			</p>
		{:else if isLocal}
			<p class="text-sm text-amber-700 sm:col-span-3 dark:text-amber-400">
				This address only works on this computer. Use the one your phone opens Rollcall at, such as
				its IP address on your network, so the labels scan from your phone.
			</p>
		{/if}
	</section>

	{#if error}
		<p
			class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
			role="alert"
		>
			{error}
		</p>
	{/if}

	<section class={card}>
		<div class="flex flex-wrap items-center justify-between gap-2">
			<h2 class="font-semibold">
				{chosen.length} of {spools.length}
				{spools.length === 1 ? 'spool' : 'spools'} chosen
			</h2>
			<button
				type="button"
				onclick={() => (pickerOpen = !pickerOpen)}
				aria-expanded={pickerOpen}
				class="text-sm font-medium text-orange-700 hover:underline dark:text-orange-400"
			>
				{pickerOpen ? 'Hide the list' : 'Choose spools'}
			</button>
		</div>
		{#if pickerOpen}
			<div class="mt-3 flex flex-wrap items-center gap-2">
				<input
					type="search"
					bind:value={q}
					placeholder="Filter by color, brand, material…"
					class="{input} max-w-xs"
					aria-label="Filter spools"
				/>
				<button
					type="button"
					onclick={() => selectShown(true)}
					class="rounded-lg px-3 py-2 text-sm font-medium ring-1 ring-slate-300 hover:bg-slate-50 dark:ring-slate-700 dark:hover:bg-slate-800"
				>
					Select all
				</button>
				<button
					type="button"
					onclick={() => selectShown(false)}
					class="rounded-lg px-3 py-2 text-sm font-medium ring-1 ring-slate-300 hover:bg-slate-50 dark:ring-slate-700 dark:hover:bg-slate-800"
				>
					Select none
				</button>
			</div>
			{#if loading}
				<p class="py-6 text-center text-sm text-slate-500">Loading spools…</p>
			{:else if !shown.length}
				<p class="py-6 text-center text-sm text-slate-500">No spools match.</p>
			{:else}
				<ul class="mt-3 grid max-h-96 gap-1 overflow-y-auto sm:grid-cols-2 lg:grid-cols-3">
					{#each shown as s (s.id)}
						<li>
							<label
								class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-slate-50 dark:hover:bg-slate-800"
							>
								<input
									type="checkbox"
									checked={selected.includes(s.id)}
									onchange={() => toggle(s.id)}
									class="size-4 accent-orange-600"
								/>
								<ColorSwatch
									hex={s.color_hex}
									secondary={s.color_hex_secondary}
									size="sm"
									label="{s.color_name} swatch"
								/>
								<span class="min-w-0 truncate text-sm">
									<span class="font-medium">{s.color_name}</span>
									<span class="text-slate-500 dark:text-slate-400">
										· {s.brand_name}
										{s.material_code}{s.label_code ? ` · #${s.label_code}` : ''}
										{s.status === 'empty' || s.status === 'archived' ? ` · ${s.status}` : ''}
									</span>
								</span>
							</label>
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	</section>

	{#if chosen.length}
		<h2 class="font-semibold">Preview, at the printed size</h2>
	{/if}
</div>

{#if baseOk}
	<div class="label-sheet flex flex-wrap gap-3 print:gap-0">
		{#each chosen as s (s.id)}
			<div class="shadow-sm ring-1 ring-slate-300 print:shadow-none print:ring-0">
				<SpoolLabel
					spool={s}
					filament={filaments[s.filament_id] ?? null}
					link={spoolLink(base, resolve('/spools/[id]', { id: s.id }))}
					{size}
				/>
			</div>
		{/each}
	</div>
{/if}
