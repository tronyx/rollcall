<script lang="ts">
	import { resolve } from '$app/paths';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Filament, Printer, TunedSettings, TunedValues } from '#lib/api/types.ts';
	import { dayOf } from '#lib/util/format.ts';
	import {
		addableTargets,
		emptyValues,
		formatValue,
		setFields,
		TUNED_FIELDS
	} from '#lib/util/tuned.ts';

	let { filament, card }: { filament: Filament; card: string } = $props();

	let rows = $state<TunedSettings[]>([]);
	let printers = $state<Printer[]>([]);
	let loaded = $state(false);
	let loadError = $state<string | null>(null);

	// The form edits one row at a time: an existing one, or a new one for `target`.
	let editingId = $state<string | 'new' | null>(null);
	let target = $state<string>('');
	let form = $state<TunedValues & { notes: string; verified: boolean }>({
		...emptyValues(),
		notes: '',
		verified: false
	});
	let saving = $state(false);
	let formError = $state<string | null>(null);
	let confirmDelete = $state<string | null>(null);

	async function load(filamentId: string) {
		loaded = false;
		loadError = null;
		try {
			[rows, printers] = await Promise.all([api.tuned(filamentId), api.printers()]);
			loaded = true;
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : 'Could not load tuned settings.';
		}
	}

	$effect(() => {
		load(filament.id);
	});

	const addable = $derived(addableTargets(printers, rows));
	const canAdd = $derived(addable.anyPrinter || addable.printers.length > 0);
	// "" stands for "any printer" in the select.
	const firstTarget = $derived(
		addable.printers.find((p) => p.is_default)?.id ??
			addable.printers[0]?.id ??
			(addable.anyPrinter ? '' : null)
	);

	function startNew() {
		if (firstTarget === null) return;
		target = firstTarget;
		form = { ...emptyValues(), notes: '', verified: false };
		formError = null;
		editingId = 'new';
	}

	function startEdit(row: TunedSettings) {
		const values = Object.fromEntries(TUNED_FIELDS.map((f) => [f.key, row[f.key]]));
		form = {
			...(values as unknown as TunedValues),
			notes: row.notes ?? '',
			verified: !!row.verified_at
		};
		formError = null;
		editingId = row.id;
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		const row = rows.find((r) => r.id === editingId);
		const printerId = editingId === 'new' ? target || null : (row?.printer_id ?? null);
		const values = Object.fromEntries(
			TUNED_FIELDS.map((f) => {
				const v = form[f.key];
				return [f.key, v === null || v === undefined || Number.isNaN(v) ? null : Number(v)];
			})
		) as unknown as TunedValues;
		saving = true;
		formError = null;
		try {
			const saved = await api.saveTuned(filament.id, {
				...values,
				printer_id: printerId,
				notes: form.notes.trim() || null,
				verified: form.verified
			});
			rows = [...rows.filter((r) => r.id !== saved.id), saved].sort(order);
			editingId = null;
		} catch (err) {
			formError = err instanceof ApiError ? err.message : 'Could not save these settings.';
		} finally {
			saving = false;
		}
	}

	// The same order as the API: any printer first, then the default, then by name.
	function order(a: TunedSettings, b: TunedSettings) {
		const rank = (t: TunedSettings) =>
			t.printer_id === null ? 0 : printers.find((p) => p.id === t.printer_id)?.is_default ? 1 : 2;
		return rank(a) - rank(b) || (a.printer_name ?? '').localeCompare(b.printer_name ?? '');
	}

	async function remove(row: TunedSettings) {
		try {
			await api.deleteTuned(row.id);
			rows = rows.filter((r) => r.id !== row.id);
			confirmDelete = null;
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'Could not delete these settings.';
		}
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
	const link = 'text-sm font-semibold text-orange-700 dark:text-orange-400';
</script>

{#snippet editor(title: string)}
	<form
		onsubmit={save}
		class="mt-2 grid grid-cols-2 gap-3 rounded-xl bg-slate-50 p-3 dark:bg-slate-800/50"
	>
		<p class="col-span-2 text-sm font-semibold">{title}</p>
		{#if editingId === 'new'}
			<label class="col-span-2">
				<span class={label}>Printer</span>
				<select bind:value={target} class={input}>
					{#each addable.printers as p (p.id)}
						<option value={p.id}>{p.name}{p.is_default ? ' (default)' : ''}</option>
					{/each}
					{#if addable.anyPrinter}<option value="">Any printer</option>{/if}
				</select>
			</label>
		{/if}
		<p class="col-span-2 text-xs text-slate-500 dark:text-slate-400">
			Fill in only what you changed. Empty fields use the filament's values, shown in grey.
		</p>
		{#each TUNED_FIELDS as f (f.key)}
			<label>
				<span class={label}>{f.label}{f.unit ? ` (${f.unit})` : ''}</span>
				<input
					type="number"
					step={f.step}
					min={f.min}
					max={f.max}
					bind:value={form[f.key]}
					placeholder={f.fallback?.(filament) ?? ''}
					class={input}
				/>
			</label>
		{/each}
		<label class="col-span-2">
			<span class={label}>Notes</span>
			<textarea
				bind:value={form.notes}
				rows="2"
				class={input}
				placeholder="Plate, slicer profile, what you changed and why"></textarea>
		</label>
		<label class="col-span-2 flex items-center gap-2 text-sm">
			<input type="checkbox" bind:checked={form.verified} class="size-4 accent-orange-600" />
			I've printed with these settings and they work
		</label>
		{#if formError}<p class="col-span-2 text-sm text-red-600" role="alert">{formError}</p>{/if}
		<div class="col-span-2 flex justify-end gap-2">
			<button
				type="button"
				onclick={() => (editingId = null)}
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
{/snippet}

<section class={card}>
	<div class="mb-2 flex items-center justify-between gap-2">
		<h2 class="font-semibold">Tuned settings</h2>
		{#if loaded && canAdd && editingId === null}
			<button type="button" class={link} onclick={startNew}>+ Add</button>
		{/if}
	</div>

	{#if loadError}
		<p class="text-sm text-red-600" role="alert">{loadError}</p>
	{:else if !loaded}
		<p class="text-sm text-slate-500">Loading…</p>
	{:else}
		{#if rows.length === 0 && editingId === null}
			<p class="text-sm text-slate-600 dark:text-slate-400">
				Save the settings you've dialed in for this filament, for each printer or for any printer.
				They're shared by every spool of it.
			</p>
		{/if}
		{#if printers.length === 0}
			<p class="mt-2 text-xs text-slate-500 dark:text-slate-400">
				To save settings for a specific printer, add it in <a
					href={resolve('/settings')}
					class="font-medium text-orange-700 hover:underline dark:text-orange-400"
				>
					Settings
				</a>.
			</p>
		{/if}

		<ul class="space-y-3">
			{#each rows as row (row.id)}
				<li class="rounded-xl ring-1 ring-slate-200 dark:ring-slate-800">
					{#if editingId === row.id}
						{@render editor(row.printer_name ? `🖨️ ${row.printer_name}` : 'Any printer')}
					{:else}
						<div class="p-3">
							<div class="flex flex-wrap items-center justify-between gap-2">
								<p class="font-medium">
									{row.printer_name ? `🖨️ ${row.printer_name}` : 'Any printer'}
								</p>
								<div class="flex items-center gap-3">
									{#if row.verified_at}
										<span
											class="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200"
											title="Marked as tested"
										>
											✓ Tested {dayOf(row.verified_at)}
										</span>
									{/if}
									<button type="button" class={link} onclick={() => startEdit(row)}>Edit</button>
								</div>
							</div>
							{#if setFields(row).length}
								<dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
									{#each setFields(row) as f (f.key)}
										<dt class="text-slate-500">{f.label}</dt>
										<dd>
											<span class="font-medium tabular-nums">{formatValue(f, row[f.key]!)}</span>
											{#if f.fallback?.(filament)}
												<span class="text-xs text-slate-500 dark:text-slate-400">
													(maker: {f.fallback(filament)})
												</span>
											{/if}
										</dd>
									{/each}
								</dl>
							{:else}
								<p class="mt-1 text-sm text-slate-500">No values yet.</p>
							{/if}
							{#if row.notes}
								<p class="mt-2 text-sm whitespace-pre-line text-slate-700 dark:text-slate-300">
									{row.notes}
								</p>
							{/if}
							<div class="mt-2 text-right">
								{#if confirmDelete === row.id}
									<span class="text-sm">Delete these settings?</span>
									<button
										type="button"
										class="ml-2 text-sm font-semibold text-red-600"
										onclick={() => remove(row)}
									>
										Delete
									</button>
									<button
										type="button"
										class="ml-2 text-sm font-semibold"
										onclick={() => (confirmDelete = null)}
									>
										Keep
									</button>
								{:else}
									<button
										type="button"
										class="text-xs font-medium text-slate-500 hover:text-red-600"
										onclick={() => (confirmDelete = row.id)}
									>
										Delete…
									</button>
								{/if}
							</div>
						</div>
					{/if}
				</li>
			{/each}
		</ul>

		{#if editingId === 'new'}
			{@render editor('New tuned settings')}
		{/if}
	{/if}
</section>
