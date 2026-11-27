<script lang="ts">
	import type { InventoryStats } from '#lib/api/types.ts';
	import { kg } from '#lib/util/format.ts';

	let {
		stats,
		lowStockActive,
		onLowStock
	}: { stats: InventoryStats | null; lowStockActive: boolean; onLowStock: () => void } = $props();
</script>

<div class="grid grid-cols-3 gap-2 sm:gap-3">
	<div
		class="rounded-xl bg-white p-3 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800"
	>
		<div class="text-xs font-medium text-slate-500 dark:text-slate-400">Spools</div>
		<div class="mt-1 text-2xl font-semibold tabular-nums">{stats?.spools_in_stock ?? '–'}</div>
		{#if stats}
			<div class="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
				{stats.spools_active} open · {stats.spools_sealed} sealed
			</div>
		{/if}
	</div>
	<div
		class="rounded-xl bg-white p-3 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800"
	>
		<div class="text-xs font-medium text-slate-500 dark:text-slate-400">Remaining</div>
		<div class="mt-1 text-2xl font-semibold tabular-nums">
			{stats ? kg(stats.remaining_kg) : '–'}
		</div>
		{#if stats?.spools_drying}
			<div class="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
				{stats.spools_drying} drying
			</div>
		{/if}
	</div>
	<button
		type="button"
		onclick={onLowStock}
		aria-pressed={lowStockActive}
		class="rounded-xl p-3 text-left shadow-sm ring-1 transition {lowStockActive
			? 'bg-red-600 text-white ring-red-600'
			: stats?.low_stock_count
				? 'bg-red-50 text-red-900 ring-red-200 dark:bg-red-950 dark:text-red-100 dark:ring-red-900'
				: 'bg-white ring-slate-200 dark:bg-slate-900 dark:ring-slate-800'}"
	>
		<div class="text-xs font-medium opacity-80">Low stock</div>
		<div class="mt-1 text-2xl font-semibold tabular-nums">{stats?.low_stock_count ?? '–'}</div>
		{#if stats}
			<div class="mt-0.5 text-xs opacity-80">under {stats.low_stock_threshold_g} g</div>
		{/if}
	</button>
</div>
