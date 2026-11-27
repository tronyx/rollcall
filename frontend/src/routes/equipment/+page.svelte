<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { api, ApiError } from '#lib/api/client.ts';
	import type {
		Brand,
		Equipment,
		EquipmentCategory,
		EquipmentFilter,
		Photo,
		Printer
	} from '#lib/api/types.ts';
	import EquipmentForm from '#lib/components/EquipmentForm.svelte';
	import PhotosSection from '#lib/components/PhotosSection.svelte';
	import { filterEquipment, makerLine, usedIds } from '#lib/util/equipment.ts';
	import { acquiredLine } from '#lib/util/format.ts';
	import { onMount } from 'svelte';

	let items = $state<Equipment[]>([]);
	let brands = $state<Brand[]>([]);
	let printers = $state<Printer[]>([]);
	let currency = $state('USD');
	let loading = $state(true);
	let error = $state<string | null>(null);

	// The filters start from the address, so other pages can link here
	// (Settings links each printer to ?printer=<id>).
	const params = page.url.searchParams;
	let filter = $state<EquipmentFilter>({
		category: (params.get('category') as EquipmentCategory | null) ?? '',
		brand_id: params.get('brand_id') ?? '',
		printer: params.get('printer') ?? '',
		q: '',
		include_retired: false
	});

	// null: no form; 'new': adding; otherwise the id being edited.
	let editing = $state<string | 'new' | null>(null);

	onMount(async () => {
		try {
			const [all, b, p, s] = await Promise.all([
				api.equipment({ include_retired: true }),
				api.brands(),
				api.printers(),
				api.settings()
			]);
			items = all;
			brands = b;
			printers = p;
			currency = s.default_currency;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load your equipment.';
		} finally {
			loading = false;
		}
	});

	const shown = $derived(filterEquipment(items, filter));
	const used = $derived(usedIds(items));
	const brandOptions = $derived(brands.filter((b) => used.brands.has(b.id)));
	const counts = $derived({
		all: filterEquipment(items, { ...filter, category: '' }).length,
		equipment: filterEquipment(items, { ...filter, category: 'equipment' }).length,
		accessory: filterEquipment(items, { ...filter, category: 'accessory' }).length
	});
	const filtered = $derived(!!(filter.brand_id || filter.printer || filter.q?.trim()));

	function saved(e: Equipment) {
		items = items.some((x) => x.id === e.id)
			? items.map((x) => (x.id === e.id ? e : x))
			: [...items, e];
		editing = null;
		// A new brand shows up in the brand filter.
		if (e.brand_id && !brands.some((b) => b.id === e.brand_id)) {
			api.brands().then((b) => (brands = b));
		}
	}

	// Keeps the card's picture in step with the item's first photo.
	function photosChanged(id: string, photos: Photo[]) {
		items = items.map((x) => (x.id === id ? { ...x, photo_url: photos[0]?.url ?? null } : x));
	}

	function deleted(id: string) {
		items = items.filter((x) => x.id !== id);
		editing = null;
	}

	const tabs: { id: EquipmentCategory | ''; label: string; count: keyof typeof counts }[] = [
		{ id: '', label: 'All', count: 'all' },
		{ id: 'equipment', label: 'Equipment', count: 'equipment' },
		{ id: 'accessory', label: 'Accessories', count: 'accessory' }
	];

	const card =
		'rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800';
	const select =
		'rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
</script>

<svelte:head>
	<title>Equipment · Rollcall</title>
</svelte:head>

<nav class="mb-4">
	<a
		href={resolve('/')}
		class="text-sm font-medium text-orange-700 hover:underline dark:text-orange-400"
	>
		← All spools
	</a>
</nav>

<header class="mb-4 flex flex-wrap items-center justify-between gap-3">
	<h1 class="text-xl font-bold tracking-tight">🧰 Equipment and accessories</h1>
	{#if editing === null}
		<button
			type="button"
			onclick={() => (editing = 'new')}
			class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-orange-700"
		>
			+ Add
		</button>
	{/if}
</header>

{#if editing === 'new'}
	<section class="{card} mb-4">
		<h2 class="mb-3 font-semibold">New item</h2>
		<EquipmentForm
			category={filter.category || 'equipment'}
			printerId={filter.printer && filter.printer !== 'none' ? filter.printer : ''}
			{brands}
			{printers}
			defaultCurrency={currency}
			onSaved={saved}
			onDeleted={deleted}
			onCancel={() => (editing = null)}
		/>
		<p class="mt-3 text-xs text-slate-500 dark:text-slate-400">
			You can add photos once it's saved.
		</p>
	</section>
{/if}

<div class="mb-4 space-y-3">
	<div class="flex gap-1 overflow-x-auto" role="tablist" aria-label="Category">
		{#each tabs as t (t.id)}
			<button
				type="button"
				role="tab"
				aria-selected={filter.category === t.id}
				onclick={() => (filter.category = t.id)}
				class="rounded-full px-3 py-1.5 text-sm font-medium whitespace-nowrap {filter.category ===
				t.id
					? 'bg-orange-600 text-white'
					: 'text-slate-700 ring-1 ring-slate-300 hover:bg-slate-50 dark:text-slate-200 dark:ring-slate-700 dark:hover:bg-slate-800'}"
			>
				{t.label} <span class="tabular-nums opacity-75">{counts[t.count]}</span>
			</button>
		{/each}
	</div>
	<div class="flex flex-wrap items-center gap-2">
		<input
			type="search"
			bind:value={filter.q}
			placeholder="Search name, kind, model, notes…"
			aria-label="Search equipment"
			class="{select} min-w-0 flex-1 sm:max-w-xs"
		/>
		<select bind:value={filter.brand_id} aria-label="Brand" class={select}>
			<option value="">All brands</option>
			{#each brandOptions as b (b.id)}<option value={b.id}>{b.name}</option>{/each}
		</select>
		<select bind:value={filter.printer} aria-label="Printer" class={select}>
			<option value="">All printers</option>
			{#each printers as p (p.id)}<option value={p.id}>For {p.name}</option>{/each}
			<option value="none">Not tied to a printer</option>
		</select>
		<label class="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
			<input
				type="checkbox"
				bind:checked={filter.include_retired}
				class="size-4 accent-orange-600"
			/>
			Show retired
		</label>
	</div>
</div>

{#if error}
	<p
		class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
		role="alert"
	>
		{error}
	</p>
{:else if loading}
	<p class="py-12 text-center text-slate-500">Loading…</p>
{:else if !shown.length}
	<div
		class="rounded-2xl border-2 border-dashed border-slate-300 py-12 text-center dark:border-slate-700"
	>
		{#if items.length && (filtered || filter.category)}
			<p class="font-medium">Nothing matches.</p>
			<p class="mt-1 text-sm text-slate-500">Try another brand, printer, or tab.</p>
		{:else}
			<p class="font-medium">No equipment yet.</p>
			<p class="mt-1 text-sm text-slate-500">
				Add nozzles, AMS units, dryers, build plates, tools and spares to keep track of them.
			</p>
		{/if}
	</div>
{:else}
	<ul class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
		{#each shown as e (e.id)}
			<li class="{card} {e.retired ? 'opacity-60' : ''}" class:sm:col-span-2={editing === e.id}>
				{#if editing === e.id}
					<h2 class="mb-3 font-semibold">Edit {e.name}</h2>
					<EquipmentForm
						item={e}
						{brands}
						{printers}
						defaultCurrency={currency}
						onSaved={saved}
						onDeleted={deleted}
						onCancel={() => (editing = null)}
					/>
					<div class="mt-4 border-t border-slate-200 pt-4 dark:border-slate-800">
						<PhotosSection
							owner={{ equipment_id: e.id }}
							onchange={(photos) => photosChanged(e.id, photos)}
						/>
					</div>
				{:else}
					<div class="flex items-start gap-3">
						{#if e.photo_url}
							<img
								src={e.photo_url}
								alt=""
								loading="lazy"
								class="size-14 shrink-0 rounded-lg object-cover ring-1 ring-slate-200 dark:ring-slate-700"
							/>
						{/if}
						<div class="min-w-0 flex-1">
							<p class="text-xs font-medium tracking-wide text-slate-500 uppercase">
								{e.kind ?? (e.category === 'equipment' ? 'Equipment' : 'Accessory')}
								{#if e.retired}· retired{/if}
							</p>
							<h2 class="font-semibold">
								{e.name}{#if e.quantity !== 1}<span class="ml-1 text-slate-500 tabular-nums">
										×{e.quantity}
									</span>{/if}
							</h2>
							{#if makerLine(e)}
								<p class="text-sm text-slate-600 dark:text-slate-400">{makerLine(e)}</p>
							{/if}
						</div>
						<button
							type="button"
							class="shrink-0 text-sm font-semibold text-orange-700 dark:text-orange-400"
							onclick={() => (editing = e.id)}
							disabled={editing !== null}
						>
							Edit
						</button>
					</div>
					<p class="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-xs text-slate-500 dark:text-slate-400">
						<span>🖨️ {e.printer_name ?? 'Any printer'}</span>
						{#if acquiredLine( { acquisition: e.acquisition, purchase_date: e.acquired_on, vendor: e.acquired_from } )}
							<span>
								{acquiredLine({
									acquisition: e.acquisition,
									purchase_date: e.acquired_on,
									vendor: e.acquired_from
								})}
							</span>
						{/if}
						{#if e.price != null}<span>{e.price.toFixed(2)} {e.currency}</span>{/if}
						{#if e.discounted}<span>🏷️ {e.discount_note ?? 'Discounted'}</span>{/if}
					</p>
					{#if e.notes}
						<p class="mt-2 text-sm whitespace-pre-line text-slate-700 dark:text-slate-300">
							{e.notes}
						</p>
					{/if}
				{/if}
			</li>
		{/each}
	</ul>
{/if}
