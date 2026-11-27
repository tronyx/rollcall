<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { api, ApiError } from '#lib/api/client.ts';
	import type {
		Filament,
		Location,
		Material,
		Spool,
		WeightAdjustment,
		WeightEvent
	} from '#lib/api/types.ts';
	import ColorSwatch from '#lib/components/ColorSwatch.svelte';
	import FilamentEditForm from '#lib/components/FilamentEditForm.svelte';
	import SpoolEditForm from '#lib/components/SpoolEditForm.svelte';
	import {
		dateOnly,
		dateTime,
		eventLabel,
		grams,
		levelColor,
		range,
		signedGrams
	} from '#lib/util/format.ts';

	const id = $derived(page.params.id ?? '');

	let spool = $state<Spool | null>(null);
	let filament = $state<Filament | null>(null);
	let events = $state<WeightEvent[]>([]);
	let detailsLoaded = $state(false);
	let locations = $state<Location[]>([]);
	let materials = $state<Material[]>([]);
	let sharedCount = $state<number | null>(null);
	let loadError = $state<string | null>(null);

	let editing = $state<'spool' | 'filament' | null>(null);
	let busy = $state(false);
	let actionError = $state<string | null>(null);
	let mode = $state<'print' | 'weigh' | 'set' | null>(null);
	let amount = $state<number | null>(null);
	let printName = $state('');
	let confirmDelete = $state(false);

	const HISTORY_LIMIT = 500;

	async function load(spoolId: string) {
		loadError = null;
		detailsLoaded = false;
		try {
			const s = await api.spool(spoolId);
			spool = s;
			const [f, ev, locs, mats, all] = await Promise.all([
				api.filament(s.filament_id),
				api.weightEvents(spoolId, HISTORY_LIMIT),
				api.locations(),
				api.materials(),
				api.spools({ include_empty: true })
			]);
			filament = f;
			events = ev;
			locations = locs;
			materials = mats;
			sharedCount = all.filter((x) => x.filament_id === s.filament_id).length;
			detailsLoaded = true;
		} catch (e) {
			loadError =
				e instanceof ApiError && e.status === 404
					? 'This spool no longer exists.'
					: e instanceof ApiError
						? e.message
						: 'Could not load this spool.';
		}
	}

	$effect(() => {
		if (id) load(id);
	});

	const pct = $derived(spool ? Math.min(100, Math.max(0, spool.remaining_pct)) : 0);

	async function adjust(body: WeightAdjustment) {
		if (!spool) return false;
		busy = true;
		actionError = null;
		try {
			const res = await api.adjustWeight(spool.id, body);
			spool = res.spool;
			events = [res.event, ...events];
			return true;
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Something went wrong.';
			return false;
		} finally {
			busy = false;
		}
	}

	function openMode(m: typeof mode) {
		mode = mode === m ? null : m;
		amount = m === 'set' && spool ? spool.remaining_weight_g : null;
		printName = '';
		actionError = null;
	}

	async function submitAmount(e: SubmitEvent) {
		e.preventDefault();
		if (!spool || amount === null || Number.isNaN(amount)) return;
		let ok = false;
		if (mode === 'print') {
			if (amount <= 0) return;
			ok = await adjust({
				kind: 'print',
				delta_g: -amount,
				print_name: printName.trim() || undefined
			});
		} else if (mode === 'weigh') {
			ok = await adjust({ kind: 'weigh_in', gross_reading_g: amount });
		} else if (mode === 'set') {
			const delta = Math.round((amount - spool.remaining_weight_g) * 10) / 10;
			if (delta === 0) {
				mode = null;
				return;
			}
			ok = await adjust({ kind: 'correction', delta_g: delta, note: 'Set on the spool page' });
		}
		if (ok) mode = null;
	}

	function undo(ev: WeightEvent) {
		adjust({ kind: 'correction', delta_g: -ev.delta_g, note: `Undo event ${ev.id}` });
	}

	async function remove() {
		if (!spool) return;
		busy = true;
		try {
			await api.deleteSpool(spool.id);
			await goto(resolve('/'));
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Could not delete the spool.';
			busy = false;
		}
	}

	function filamentSaved(f: Filament) {
		filament = f;
		editing = null;
		// Names and swatch on the spool come from the filament.
		if (spool) load(spool.id);
	}

	const statusStyles: Record<Spool['status'], string> = {
		active: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/60 dark:text-emerald-200',
		sealed: 'bg-sky-100 text-sky-800 dark:bg-sky-900/60 dark:text-sky-200',
		drying: 'bg-amber-100 text-amber-800 dark:bg-amber-900/60 dark:text-amber-200',
		empty: 'bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-300',
		archived: 'bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-300'
	};
	const card =
		'rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800';
	const btn =
		'rounded-lg px-3 py-2 text-sm font-semibold ring-1 ring-slate-300 transition hover:bg-slate-50 disabled:opacity-40 active:scale-95 dark:ring-slate-700 dark:hover:bg-slate-800';
	const input =
		'w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
</script>

<svelte:head>
	<title>{spool ? `${spool.color_name} · ${spool.brand_name}` : 'Spool'} · Rollcall</title>
</svelte:head>

<nav class="mb-4">
	<a
		href={resolve('/')}
		class="text-sm font-medium text-orange-700 hover:underline dark:text-orange-400"
	>
		← All spools
	</a>
</nav>

{#if loadError}
	<p
		class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
		role="alert"
	>
		{loadError}
	</p>
{:else if !spool}
	<p class="py-12 text-center text-slate-500">Loading spool…</p>
{:else}
	<div class="grid gap-4 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
		<div class="space-y-4">
			<!-- Summary and weight -->
			<section class={card}>
				<header class="flex items-start gap-4">
					<ColorSwatch
						hex={spool.color_hex}
						secondary={spool.color_hex_secondary}
						label="{spool.color_name} swatch"
					/>
					<div class="min-w-0 flex-1">
						<div class="flex flex-wrap items-center gap-2">
							<h1 class="text-xl font-bold tracking-tight">{spool.color_name}</h1>
							<span
								class="rounded-full px-2 py-0.5 text-xs font-medium {statusStyles[spool.status]}"
							>
								{spool.status}
							</span>
						</div>
						<p class="text-sm text-slate-600 dark:text-slate-400">
							{spool.brand_name} · {spool.material_code}{spool.product_name
								? ` · ${spool.product_name}`
								: ''} · {spool.diameter_mm} mm
						</p>
						<p class="mt-0.5 flex flex-wrap gap-x-3 text-xs text-slate-500 dark:text-slate-400">
							{#if spool.location_name}<span>📍 {spool.location_name}</span>{/if}
							{#if spool.label_code}<span class="font-mono">#{spool.label_code}</span>{/if}
						</p>
					</div>
				</header>

				<div class="mt-4">
					<div class="flex items-baseline justify-between">
						<span class="text-3xl font-bold tabular-nums" class:text-red-600={spool.is_low_stock}>
							{grams(spool.remaining_weight_g)}
						</span>
						<span class="text-sm text-slate-500 tabular-nums dark:text-slate-400">
							of {grams(spool.initial_weight_g)} · {pct}%
						</span>
					</div>
					<div
						class="mt-2 h-3 overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800"
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

				<div class="mt-4 flex flex-wrap gap-2">
					<button
						type="button"
						class={btn}
						disabled={busy || spool.remaining_weight_g <= 0}
						onclick={() => adjust({ kind: 'quick_adjust', delta_g: -10 })}
					>
						−10g
					</button>
					<button
						type="button"
						class={btn}
						disabled={busy || spool.remaining_weight_g <= 0}
						onclick={() => adjust({ kind: 'quick_adjust', delta_g: -50 })}
					>
						−50g
					</button>
					<button type="button" class={btn} disabled={busy} onclick={() => openMode('print')}>
						Log print
					</button>
					<button
						type="button"
						class={btn}
						disabled={busy || spool.tare_weight_g == null}
						title={spool.tare_weight_g == null
							? 'Set the empty spool weight first'
							: 'Enter what the scale shows'}
						onclick={() => openMode('weigh')}
					>
						Weigh
					</button>
					<button type="button" class={btn} disabled={busy} onclick={() => openMode('set')}>
						Set remaining
					</button>
				</div>

				{#if mode}
					<form onsubmit={submitAmount} class="mt-3 flex flex-wrap items-center gap-2">
						<input
							type="number"
							inputmode="decimal"
							step="0.1"
							min={mode === 'print' ? 0.1 : 0}
							bind:value={amount}
							class="{input} w-40"
							aria-label={mode === 'print'
								? 'Grams used'
								: mode === 'weigh'
									? 'Scale reading in grams'
									: 'Remaining grams'}
							placeholder={mode === 'print'
								? 'Grams used'
								: mode === 'weigh'
									? 'Scale reading (g)'
									: 'Remaining (g)'}
							required
						/>
						{#if mode === 'print'}
							<input
								type="text"
								bind:value={printName}
								placeholder="Print name (optional)"
								aria-label="Print name"
								class="{input} min-w-0 flex-1"
							/>
						{/if}
						<button
							type="submit"
							disabled={busy}
							class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
						>
							Save
						</button>
						<button type="button" class={btn} onclick={() => (mode = null)}>Cancel</button>
						{#if mode === 'weigh'}
							<p class="w-full text-xs text-slate-500 dark:text-slate-400">
								Weigh the whole spool. Rollcall subtracts the empty spool's {grams(
									spool.tare_weight_g ?? 0
								)}.
							</p>
						{/if}
					</form>
				{/if}
				{#if actionError}<p class="mt-2 text-sm text-red-600" role="alert">{actionError}</p>{/if}
			</section>

			<!-- Weight history -->
			<section class={card}>
				<h2 class="mb-2 font-semibold">Weight history</h2>
				{#if !detailsLoaded && events.length === 0}
					<p class="text-sm text-slate-500">Loading…</p>
				{:else if events.length === 0}
					<p class="text-sm text-slate-500">No changes yet.</p>
				{:else}
					<ul class="divide-y divide-slate-100 text-sm dark:divide-slate-800">
						{#each events as ev, i (ev.id)}
							<li class="flex items-center gap-3 py-2">
								<div class="min-w-0 flex-1">
									<p class="font-medium">
										{eventLabel(ev.kind)}{ev.print_name ? ` · ${ev.print_name}` : ''}
									</p>
									<p class="text-xs text-slate-500 dark:text-slate-400">
										{dateTime(ev.created_at)}{ev.gross_reading_g != null
											? ` · scale ${grams(ev.gross_reading_g)}`
											: ''}{ev.note ? ` · ${ev.note}` : ''}
									</p>
								</div>
								<div class="text-right tabular-nums">
									<p
										class:text-red-600={ev.delta_g < 0}
										class:text-emerald-600={ev.delta_g > 0}
										class="font-semibold"
									>
										{signedGrams(ev.delta_g)}
									</p>
									<p class="text-xs text-slate-500">→ {grams(ev.weight_after_g)}</p>
								</div>
								{#if i === 0 && ev.kind !== 'initial' && ev.delta_g !== 0}
									<button
										type="button"
										class="text-xs font-semibold text-orange-700 dark:text-orange-400"
										disabled={busy}
										onclick={() => undo(ev)}
									>
										Undo
									</button>
								{/if}
							</li>
						{/each}
					</ul>
					{#if events.length === HISTORY_LIMIT}
						<p class="mt-2 text-xs text-slate-500">Showing the latest {HISTORY_LIMIT} changes.</p>
					{/if}
				{/if}
			</section>
		</div>

		<div class="space-y-4">
			<!-- This spool -->
			<section class={card}>
				<div class="mb-2 flex items-center justify-between">
					<h2 class="font-semibold">This spool</h2>
					{#if editing !== 'spool'}
						<button
							type="button"
							class="text-sm font-semibold text-orange-700 dark:text-orange-400"
							onclick={() => (editing = 'spool')}
						>
							Edit
						</button>
					{/if}
				</div>
				{#if editing === 'spool'}
					<SpoolEditForm
						{spool}
						{locations}
						onSaved={(s) => {
							spool = s;
							editing = null;
						}}
						onCancel={() => (editing = null)}
					/>
				{:else}
					<dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
						<dt class="text-slate-500">Net weight</dt>
						<dd>{grams(spool.initial_weight_g)}</dd>
						<dt class="text-slate-500">Empty spool</dt>
						<dd>{spool.tare_weight_g != null ? grams(spool.tare_weight_g) : '—'}</dd>
						<dt class="text-slate-500">Price</dt>
						<dd>
							{spool.purchase_price != null
								? `${spool.purchase_price.toFixed(2)} ${spool.currency}`
								: '—'}
						</dd>
						<dt class="text-slate-500">Purchased</dt>
						<dd>
							{spool.purchase_date ? dateOnly(spool.purchase_date) : '—'}{spool.vendor
								? ` from ${spool.vendor}`
								: ''}
						</dd>
						<dt class="text-slate-500">Lot</dt>
						<dd>{spool.lot_number ?? '—'}</dd>
						<dt class="text-slate-500">Opened</dt>
						<dd>{spool.opened_at ? dateTime(spool.opened_at) : '—'}</dd>
						<dt class="text-slate-500">Last dried</dt>
						<dd>{spool.last_dried_at ? dateTime(spool.last_dried_at) : '—'}</dd>
						{#if spool.emptied_at}
							<dt class="text-slate-500">Emptied</dt>
							<dd>{dateTime(spool.emptied_at)}</dd>
						{/if}
						<dt class="text-slate-500">Added</dt>
						<dd>{dateTime(spool.created_at)}</dd>
					</dl>
					{#if spool.notes}
						<p class="mt-3 text-sm whitespace-pre-line text-slate-700 dark:text-slate-300">
							{spool.notes}
						</p>
					{/if}
				{/if}
			</section>

			<!-- Filament -->
			{#if filament}
				<section class={card}>
					<div class="mb-2 flex items-center justify-between">
						<h2 class="font-semibold">Filament</h2>
						{#if editing !== 'filament'}
							<button
								type="button"
								class="text-sm font-semibold text-orange-700 dark:text-orange-400"
								onclick={() => (editing = 'filament')}
							>
								Edit
							</button>
						{/if}
					</div>
					{#if editing === 'filament'}
						<FilamentEditForm
							{filament}
							{materials}
							spoolCount={sharedCount}
							onSaved={filamentSaved}
							onCancel={() => (editing = null)}
						/>
					{:else}
						<dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
							<dt class="text-slate-500">Nozzle</dt>
							<dd>{range(filament.nozzle_temp_min_c, filament.nozzle_temp_max_c, '°C') ?? '—'}</dd>
							<dt class="text-slate-500">Bed</dt>
							<dd>{range(filament.bed_temp_min_c, filament.bed_temp_max_c, '°C') ?? '—'}</dd>
							<dt class="text-slate-500">Speed</dt>
							<dd>
								{range(filament.print_speed_min_mm_s, filament.print_speed_max_mm_s, ' mm/s') ??
									'—'}
							</dd>
							<dt class="text-slate-500">Drying</dt>
							<dd>
								{filament.drying_temp_c != null
									? `${filament.drying_temp_c}°C${filament.drying_time_min ? ` for ${filament.drying_time_min} min` : ''}`
									: '—'}
							</dd>
							<dt class="text-slate-500">TD value</dt>
							<dd>{filament.td_value ?? '—'}</dd>
							{#if filament.source_url}
								<dt class="text-slate-500">Product page</dt>
								<dd class="truncate">
									<a
										href={filament.source_url}
										target="_blank"
										rel="noopener noreferrer"
										class="text-orange-700 hover:underline dark:text-orange-400"
									>
										{new URL(filament.source_url).hostname}
									</a>
								</dd>
							{/if}
						</dl>
						{#if sharedCount !== null && sharedCount > 1}
							<p class="mt-3 text-xs text-slate-500">
								{sharedCount} spools share this filament's details.
							</p>
						{/if}
					{/if}
				</section>
			{/if}

			<!-- Delete -->
			<section class={card}>
				{#if confirmDelete}
					<p class="text-sm">
						Delete this spool and its weight history? This can't be undone. To keep the history, set
						the status to <em>archived</em> instead.
					</p>
					<div class="mt-3 flex gap-2">
						<button
							type="button"
							disabled={busy}
							onclick={remove}
							class="rounded-lg bg-red-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
						>
							Delete spool
						</button>
						<button type="button" class={btn} onclick={() => (confirmDelete = false)}>
							Cancel
						</button>
					</div>
				{:else}
					<button
						type="button"
						class="text-sm font-semibold text-red-600"
						onclick={() => (confirmDelete = true)}
					>
						Delete spool…
					</button>
				{/if}
			</section>
		</div>
	</div>
{/if}
