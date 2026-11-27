<script lang="ts">
	import { api, ApiError } from '$lib/api/client';
	import type {
		Brand,
		InventoryStats,
		Location,
		Material,
		Spool,
		SpoolFilter
	} from '$lib/api/types';
	import AddSpoolModal from '$lib/components/AddSpoolModal.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import SpoolCard from '$lib/components/SpoolCard.svelte';
	import StatTiles from '$lib/components/StatTiles.svelte';
	import { onMount } from 'svelte';

	let spools = $state<Spool[]>([]);
	let stats = $state<InventoryStats | null>(null);
	let brands = $state<Brand[]>([]);
	let materials = $state<Material[]>([]);
	let locations = $state<Location[]>([]);
	let filter = $state<SpoolFilter>({});
	let loading = $state(true);
	let error = $state<string | null>(null);
	let addOpen = $state(false);

	async function loadSpools(f: SpoolFilter) {
		try {
			spools = await api.spools(f);
			error = null;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load spools.';
		} finally {
			loading = false;
		}
	}

	async function refreshStats() {
		stats = await api.stats().catch(() => stats);
	}

	async function loadLookups() {
		[brands, materials, locations] = await Promise.all([
			api.brands(),
			api.materials(),
			api.locations()
		]);
	}

	onMount(() => {
		loadLookups().catch(() => {});
		refreshStats();
	});

	// Reload when filters change, debounced so typing in search doesn't spam the API.
	$effect(() => {
		const snapshot = $state.snapshot(filter) as SpoolFilter;
		const t = setTimeout(() => loadSpools(snapshot), 200);
		return () => clearTimeout(t);
	});

	function onChanged(updated: Spool) {
		spools = spools.map((s) => (s.id === updated.id ? updated : s));
		refreshStats();
	}

	function onCreated(created: Spool) {
		spools = [created, ...spools];
		refreshStats();
		loadLookups().catch(() => {});
	}
</script>

<header class="mb-4 flex items-center justify-between">
	<h1 class="flex items-center gap-2 text-xl font-bold tracking-tight">
		<img src="/favicon.svg" alt="" class="size-7" /> Rollcall
	</h1>
	<button
		type="button"
		onclick={() => (addOpen = true)}
		class="hidden rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-orange-700 sm:block"
		>+ Add spool</button
	>
</header>

<div class="space-y-4">
	<StatTiles
		{stats}
		lowStockActive={!!filter.low_stock}
		onLowStock={() => (filter.low_stock = !filter.low_stock)}
	/>
	<FilterBar bind:filter {brands} {materials} {locations} />

	{#if error}
		<p
			class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
			role="alert"
		>
			{error}
		</p>
	{/if}

	{#if loading}
		<p class="py-12 text-center text-slate-500">Loading spools…</p>
	{:else if spools.length === 0}
		<div
			class="rounded-2xl border-2 border-dashed border-slate-300 py-12 text-center dark:border-slate-700"
		>
			<p class="font-medium">No spools match.</p>
			<p class="mt-1 text-sm text-slate-500">Add one from a product link, or clear the filters.</p>
		</div>
	{:else}
		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
			{#each spools as spool (spool.id)}
				<SpoolCard {spool} {onChanged} />
			{/each}
		</div>
	{/if}
</div>

<!-- Thumb-reachable add button on phones. -->
<button
	type="button"
	onclick={() => (addOpen = true)}
	class="fixed right-4 bottom-[max(1rem,env(safe-area-inset-bottom))] rounded-full bg-orange-600 px-5 py-3.5 font-semibold text-white shadow-lg sm:hidden"
	aria-label="Add spool">+ Add spool</button
>

<AddSpoolModal bind:open={addOpen} {materials} {locations} {onCreated} />
