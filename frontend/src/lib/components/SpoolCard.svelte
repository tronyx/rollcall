<script lang="ts">
	import { resolve } from '$app/paths';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Spool, WeightAdjustment, WeightEvent } from '#lib/api/types.ts';
	import { grams, levelColor } from '#lib/util/format.ts';
	import ColorSwatch from './ColorSwatch.svelte';

	let { spool, onChanged }: { spool: Spool; onChanged: (s: Spool) => void } = $props();

	let busy = $state(false);
	let error = $state<string | null>(null);
	let lastEvent = $state<WeightEvent | null>(null);
	let logOpen = $state(false);
	let printGrams = $state<number | null>(null);
	let printName = $state('');
	let undoTimer: ReturnType<typeof setTimeout> | undefined;

	const statusStyles: Record<Spool['status'], string> = {
		active: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/60 dark:text-emerald-200',
		sealed: 'bg-sky-100 text-sky-800 dark:bg-sky-900/60 dark:text-sky-200',
		drying: 'bg-amber-100 text-amber-800 dark:bg-amber-900/60 dark:text-amber-200',
		empty: 'bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-300',
		archived: 'bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-300'
	};
	const canDeduct = $derived(spool.remaining_weight_g > 0 && !busy);
	const pct = $derived(Math.min(100, Math.max(0, spool.remaining_pct)));

	async function adjust(body: WeightAdjustment) {
		busy = true;
		error = null;
		try {
			const res = await api.adjustWeight(spool.id, body);
			onChanged(res.spool);
			lastEvent = body.kind === 'correction' ? null : res.event;
			clearTimeout(undoTimer);
			undoTimer = setTimeout(() => (lastEvent = null), 8000);
			return true;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Something went wrong.';
			return false;
		} finally {
			busy = false;
		}
	}

	const deduct = (g: number) => adjust({ kind: 'quick_adjust', delta_g: -g });

	async function logPrint(e: SubmitEvent) {
		e.preventDefault();
		if (!printGrams || printGrams <= 0) return;
		const ok = await adjust({
			kind: 'print',
			delta_g: -printGrams,
			print_name: printName.trim() || undefined
		});
		if (ok) {
			logOpen = false;
			printGrams = null;
			printName = '';
		}
	}

	function undo() {
		if (!lastEvent || lastEvent.delta_g === 0) return;
		adjust({ kind: 'correction', delta_g: -lastEvent.delta_g, note: `Undo event ${lastEvent.id}` });
	}

	const btn =
		'rounded-lg px-3 py-2 text-sm font-semibold ring-1 transition disabled:opacity-40 active:scale-95';
</script>

<article
	class="flex flex-col gap-3 rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800"
	class:opacity-60={spool.status === 'empty' || spool.status === 'archived'}
>
	<header class="relative flex items-start gap-3">
		<ColorSwatch
			hex={spool.color_hex}
			secondary={spool.color_hex_secondary}
			label="{spool.color_name} swatch"
		/>
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-2">
				<h3 class="truncate font-semibold">
					<a
						href={resolve('/spools/[id]', { id: spool.id })}
						class="hover:underline after:absolute after:inset-0"
					>
						{spool.color_name}
					</a>
				</h3>
				<span class="rounded-full px-2 py-0.5 text-xs font-medium {statusStyles[spool.status]}">
					{spool.status}
				</span>
			</div>
			<p class="truncate text-sm text-slate-600 dark:text-slate-400">
				{spool.brand_name} · {spool.material_code}{spool.product_name
					? ` · ${spool.product_name}`
					: ''}
			</p>
			<p class="mt-0.5 flex flex-wrap gap-x-3 text-xs text-slate-500 dark:text-slate-400">
				{#if spool.location_name}<span>📍 {spool.location_name}</span>{/if}
				{#if spool.label_code}<span class="font-mono">#{spool.label_code}</span>{/if}
				{#if Number(spool.diameter_mm) !== 1.75}<span>{spool.diameter_mm} mm</span>{/if}
			</p>
		</div>
	</header>

	<div>
		<div class="flex items-baseline justify-between text-sm">
			<span class="text-lg font-semibold tabular-nums" class:text-red-600={spool.is_low_stock}>
				{grams(spool.remaining_weight_g)}
			</span>
			<span class="text-xs text-slate-500 tabular-nums dark:text-slate-400">
				of {grams(spool.initial_weight_g)} · {pct}%
			</span>
		</div>
		<div
			class="mt-1 h-2 overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800"
			role="progressbar"
			aria-valuenow={pct}
			aria-valuemin={0}
			aria-valuemax={100}
			aria-label="Filament remaining"
		>
			<div
				class="h-full rounded-full transition-all {levelColor(pct, spool.is_low_stock)}"
				style:width="{pct}%"
			></div>
		</div>
	</div>

	{#if logOpen}
		<form onsubmit={logPrint} class="flex flex-wrap items-center gap-2">
			<input
				type="number"
				inputmode="decimal"
				min="0.1"
				step="0.1"
				placeholder="Grams used"
				bind:value={printGrams}
				class="w-28 rounded-lg border-0 px-3 py-2 text-sm ring-1 ring-slate-300 dark:bg-slate-800 dark:ring-slate-700"
				aria-label="Grams used"
				required
			/>
			<input
				type="text"
				placeholder="Print name (optional)"
				bind:value={printName}
				class="min-w-0 flex-1 rounded-lg border-0 px-3 py-2 text-sm ring-1 ring-slate-300 dark:bg-slate-800 dark:ring-slate-700"
				aria-label="Print name"
			/>
			<button type="submit" disabled={busy} class="{btn} bg-orange-600 text-white ring-orange-600">
				Log
			</button>
			<button
				type="button"
				class="{btn} ring-slate-300 dark:ring-slate-700"
				onclick={() => (logOpen = false)}
			>
				Cancel
			</button>
		</form>
	{:else}
		<div class="flex gap-2">
			<button
				type="button"
				disabled={!canDeduct}
				onclick={() => deduct(10)}
				class="{btn} flex-1 ring-slate-300 hover:bg-slate-50 dark:ring-slate-700 dark:hover:bg-slate-800"
			>
				−10g
			</button>
			<button
				type="button"
				disabled={!canDeduct}
				onclick={() => deduct(50)}
				class="{btn} flex-1 ring-slate-300 hover:bg-slate-50 dark:ring-slate-700 dark:hover:bg-slate-800"
			>
				−50g
			</button>
			<button
				type="button"
				disabled={!canDeduct}
				onclick={() => (logOpen = true)}
				class="{btn} flex-[1.4] bg-slate-900 text-white ring-slate-900 dark:bg-slate-100 dark:text-slate-900 dark:ring-slate-100"
			>
				Log print
			</button>
		</div>
	{/if}

	{#if lastEvent}
		<div
			class="flex items-center justify-between rounded-lg bg-slate-100 px-3 py-1.5 text-sm dark:bg-slate-800"
			role="status"
		>
			<span>
				{grams(Math.abs(lastEvent.delta_g))} deducted{lastEvent.print_name
					? ` for ${lastEvent.print_name}`
					: ''}
			</span>
			<button
				type="button"
				class="font-semibold text-orange-700 dark:text-orange-400"
				onclick={undo}
				disabled={busy}
			>
				Undo
			</button>
		</div>
	{/if}
	{#if error}
		<p class="text-sm text-red-600" role="alert">{error}</p>
	{/if}
</article>
