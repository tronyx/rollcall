<script lang="ts">
	import type { Brand, Location, Material, SpoolFilter } from '$lib/api/types';

	let {
		filter = $bindable(),
		brands,
		materials,
		locations
	}: {
		filter: SpoolFilter;
		brands: Brand[];
		materials: Material[];
		locations: Location[];
	} = $props();

	const families = $derived([...new Set(materials.map((m) => m.family))]);
	const active = $derived(
		!!(
			filter.q ||
			filter.material_family ||
			filter.brand_id ||
			filter.location_id ||
			filter.color ||
			filter.status
		)
	);
	const field =
		'w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-200 focus:ring-2 focus:ring-orange-500 dark:bg-slate-900 dark:ring-slate-700';
</script>

<div class="space-y-2">
	<input
		type="search"
		placeholder="Search brand, color, label, notes…"
		bind:value={filter.q}
		class={field}
		aria-label="Search spools"
	/>
	<div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
		<select bind:value={filter.material_family} class={field} aria-label="Material">
			<option value={undefined}>All materials</option>
			{#each families as f (f)}<option value={f}>{f}</option>{/each}
		</select>
		<select bind:value={filter.brand_id} class={field} aria-label="Brand">
			<option value={undefined}>All brands</option>
			{#each brands as b (b.id)}<option value={b.id}>{b.name}</option>{/each}
		</select>
		<select bind:value={filter.location_id} class={field} aria-label="Location">
			<option value={undefined}>All locations</option>
			{#each locations as l (l.id)}<option value={l.id}>{l.name}</option>{/each}
		</select>
		<input
			type="text"
			placeholder="Color"
			bind:value={filter.color}
			class={field}
			aria-label="Color"
		/>
	</div>
	<div class="flex items-center justify-between text-sm">
		<label class="inline-flex items-center gap-2 text-slate-600 dark:text-slate-300">
			<input
				type="checkbox"
				bind:checked={filter.include_empty}
				class="size-4 rounded accent-orange-600"
			/>
			Show empty spools
		</label>
		{#if active}
			<button
				type="button"
				class="font-medium text-orange-700 hover:underline dark:text-orange-400"
				onclick={() => (filter = { include_empty: filter.include_empty })}>Clear filters</button
			>
		{/if}
	</div>
</div>
