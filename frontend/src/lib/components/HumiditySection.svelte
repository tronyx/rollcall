<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type { HumidityReading } from '#lib/api/types.ts';
	import {
		dateTime,
		fromLocalInput,
		humidityLevel,
		toLocalInput,
		type HumidityLevel
	} from '#lib/util/format.ts';

	let {
		spoolId,
		card,
		readonly = false
	}: { spoolId: string; card: string; readonly?: boolean } = $props();

	let readings = $state<HumidityReading[]>([]);
	let loaded = $state(false);
	let error = $state<string | null>(null);
	let adding = $state(false);
	let saving = $state(false);
	let form = $state({ at: '', pct: null as number | null, temp: null as number | null, note: '' });
	let showAll = $state(false);

	async function load(id: string) {
		loaded = false;
		error = null;
		try {
			readings = await api.humidity(id);
			loaded = true;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load humidity readings.';
		}
	}

	$effect(() => {
		load(spoolId);
	});

	function open() {
		form = { at: toLocalInput(new Date().toISOString()), pct: null, temp: null, note: '' };
		error = null;
		adding = true;
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (form.pct === null || Number.isNaN(form.pct)) return;
		saving = true;
		error = null;
		try {
			const r = await api.addHumidity(spoolId, {
				measured_at: fromLocalInput(form.at),
				humidity_pct: form.pct,
				temperature_c: form.temp === null || Number.isNaN(form.temp) ? null : form.temp,
				note: form.note.trim() || null
			});
			readings = [r, ...readings].sort((a, b) => b.measured_at.localeCompare(a.measured_at));
			adding = false;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the reading.';
		} finally {
			saving = false;
		}
	}

	async function remove(r: HumidityReading) {
		try {
			await api.deleteHumidity(r.id);
			readings = readings.filter((x) => x.id !== r.id);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not delete the reading.';
		}
	}

	const latest = $derived(readings[0] ?? null);
	const shown = $derived(showAll ? readings : readings.slice(0, 5));

	// Oldest to newest, for the trend line.
	const trend = $derived([...readings].slice(0, 30).reverse());
	// Scaled to the readings' own range, so small changes still show.
	const points = $derived.by(() => {
		if (trend.length < 2) return '';
		const values = trend.map((r) => r.humidity_pct);
		const lo = Math.min(...values);
		const span = Math.max(...values) - lo || 1;
		return trend
			.map(
				(r, i) => `${(i / (trend.length - 1)) * 100},${28 - ((r.humidity_pct - lo) / span) * 26}`
			)
			.join(' ');
	});

	const levelStyles: Record<HumidityLevel, string> = {
		dry: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200',
		ok: 'bg-amber-100 text-amber-900 dark:bg-amber-950 dark:text-amber-100',
		humid: 'bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-100'
	};
	const levelText: Record<HumidityLevel, string> = {
		dry: 'Dry',
		ok: 'OK',
		humid: 'Humid: dry it soon'
	};

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

<section class={card}>
	<div class="mb-2 flex items-center justify-between gap-2">
		<h2 class="font-semibold">Humidity</h2>
		{#if loaded && !adding && !readonly}
			<button
				type="button"
				class="text-sm font-semibold text-orange-700 dark:text-orange-400"
				onclick={open}
			>
				+ Add reading
			</button>
		{/if}
	</div>

	{#if error}<p class="mb-2 text-sm text-red-600" role="alert">{error}</p>{/if}

	{#if adding}
		<form
			onsubmit={save}
			class="mb-3 grid grid-cols-2 gap-3 rounded-xl bg-slate-50 p-3 dark:bg-slate-800/50"
		>
			<label class="col-span-2 sm:col-span-1">
				<span class={label}>When</span>
				<input type="datetime-local" bind:value={form.at} class={input} required />
			</label>
			<label>
				<span class={label}>Humidity (%) *</span>
				<input
					type="number"
					inputmode="decimal"
					min="0"
					max="100"
					step="0.1"
					bind:value={form.pct}
					class={input}
					required
				/>
			</label>
			<label>
				<span class={label}>Temperature (°C)</span>
				<input
					type="number"
					inputmode="decimal"
					min="-40"
					max="150"
					step="0.1"
					bind:value={form.temp}
					class={input}
				/>
			</label>
			<label class="col-span-2 sm:col-span-1">
				<span class={label}>Note</span>
				<input
					type="text"
					bind:value={form.note}
					placeholder="Dry box, after drying…"
					class={input}
				/>
			</label>
			<div class="col-span-2 flex justify-end gap-2">
				<button
					type="button"
					onclick={() => (adding = false)}
					class="rounded-lg px-4 py-2 text-sm font-semibold ring-1 ring-slate-300 dark:ring-slate-700"
				>
					Cancel
				</button>
				<button
					type="submit"
					disabled={saving}
					class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
				>
					{saving ? 'Saving…' : 'Save'}
				</button>
			</div>
		</form>
	{/if}

	{#if !loaded && !error}
		<p class="text-sm text-slate-500">Loading…</p>
	{:else if loaded && !readings.length}
		<p class="text-sm text-slate-600 dark:text-slate-400">
			Log what the hygrometer in this spool's box or dryer reads, to see when it needs drying.
		</p>
	{:else if latest}
		<div class="flex items-center gap-3">
			<span class="text-2xl font-bold tabular-nums">{latest.humidity_pct}%</span>
			<span
				class="rounded-full px-2 py-0.5 text-xs font-medium {levelStyles[
					humidityLevel(latest.humidity_pct)
				]}"
			>
				{levelText[humidityLevel(latest.humidity_pct)]}
			</span>
			{#if points}
				<svg
					viewBox="0 0 100 30"
					preserveAspectRatio="none"
					class="h-8 flex-1 text-orange-600 dark:text-orange-400"
					role="img"
					aria-label="Humidity trend over the last {trend.length} readings"
				>
					<polyline
						{points}
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						vector-effect="non-scaling-stroke"
					/>
				</svg>
			{/if}
		</div>
		<p class="text-xs text-slate-500">{dateTime(latest.measured_at)}</p>
		<ul class="mt-3 divide-y divide-slate-100 text-sm dark:divide-slate-800">
			{#each shown as r (r.id)}
				<li class="flex items-center justify-between gap-2 py-1.5">
					<span class="min-w-0">
						<span class="font-medium tabular-nums">{r.humidity_pct}%</span>
						{#if r.temperature_c != null}
							<span class="text-slate-500 tabular-nums"> · {r.temperature_c}°C</span>
						{/if}
						<span class="block truncate text-xs text-slate-500">
							{dateTime(r.measured_at)}{r.note ? ` · ${r.note}` : ''}
						</span>
					</span>
					{#if !readonly}
						<button
							type="button"
							class="shrink-0 text-xs font-medium text-slate-500 hover:text-red-600"
							onclick={() => remove(r)}
							aria-label="Delete the {r.humidity_pct}% reading"
						>
							Delete
						</button>
					{/if}
				</li>
			{/each}
		</ul>
		{#if readings.length > 5}
			<button
				type="button"
				class="mt-1 text-sm font-medium text-orange-700 dark:text-orange-400"
				onclick={() => (showAll = !showAll)}
			>
				{showAll ? 'Show fewer' : `Show all ${readings.length}`}
			</button>
		{/if}
	{/if}
</section>
