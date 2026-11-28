<script lang="ts">
	import { locationLabel, placeable } from '#lib/util/locations.ts';
	import AcquisitionFields from '#lib/components/AcquisitionFields.svelte';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Location, Spool, SpoolStatus } from '#lib/api/types.ts';
	import { fromLocalInput, toLocalInput } from '#lib/util/format.ts';
	import { untrack } from 'svelte';

	let {
		spool,
		locations,
		onSaved,
		onCancel
	}: {
		spool: Spool;
		locations: Location[];
		onSaved: (s: Spool) => void;
		onCancel: () => void;
	} = $props();

	// A copy of the editable fields; the spool prop only seeds it.
	const start = untrack(() => $state.snapshot(spool) as Spool);
	let form = $state({
		label_code: start.label_code ?? '',
		location_id: start.location_id ?? '',
		status: start.status,
		initial_weight_g: start.initial_weight_g as number | null,
		tare_weight_g: start.tare_weight_g,
		purchase_price: start.purchase_price,
		currency: start.currency,
		purchase_date: start.purchase_date ?? '',
		vendor: start.vendor ?? '',
		acquisition: start.acquisition,
		discounted: start.discounted,
		discount_note: start.discount_note ?? '',
		lot_number: start.lot_number ?? '',
		last_dried_at: toLocalInput(start.last_dried_at),
		notes: start.notes ?? ''
	});
	let saving = $state(false);
	let error = $state<string | null>(null);

	const statuses: SpoolStatus[] = ['sealed', 'active', 'drying', 'empty', 'archived'];
	const blank = (v: string) => (v.trim() === '' ? null : v.trim());
	const num = (v: number | null | undefined) =>
		v === null || v === undefined || Number.isNaN(v) ? null : v;

	async function save(e: SubmitEvent) {
		e.preventDefault();
		const initial = num(form.initial_weight_g);
		if (!initial || initial <= 0) {
			error = 'Net weight must be more than 0 g.';
			return;
		}
		saving = true;
		error = null;
		try {
			const updated = await api.updateSpool(spool.id, {
				filament_id: spool.filament_id,
				label_code: blank(form.label_code),
				location_id: form.location_id || null,
				status: form.status,
				initial_weight_g: initial,
				tare_weight_g: num(form.tare_weight_g),
				purchase_price: num(form.purchase_price),
				currency: form.currency.trim() || 'USD',
				purchase_date: form.purchase_date || null,
				vendor: blank(form.vendor),
				acquisition: form.acquisition,
				discounted: form.discounted,
				discount_note: form.discounted ? blank(form.discount_note) : null,
				lot_number: blank(form.lot_number),
				last_dried_at: fromLocalInput(form.last_dried_at),
				notes: blank(form.notes)
			});
			onSaved(updated);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the spool.';
		} finally {
			saving = false;
		}
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

<form onsubmit={save} class="grid grid-cols-2 gap-3">
	<label class="col-span-2 sm:col-span-1">
		<span class={label}>Status</span>
		<select bind:value={form.status} class={input}>
			{#each statuses as s (s)}<option value={s}>{s}</option>{/each}
		</select>
	</label>
	<label class="col-span-2 sm:col-span-1">
		<span class={label}>Location</span>
		<select bind:value={form.location_id} class={input}>
			{#if !spool.owner_username}<option value="">None</option>{/if}
			{#each placeable(locations, spool) as l (l.id)}<option value={l.id}>
					{locationLabel(l)}
				</option>{/each}
		</select>
	</label>
	<label>
		<span class={label}>Net weight (g) *</span>
		<input type="number" min="1" step="0.1" bind:value={form.initial_weight_g} class={input} />
	</label>
	<label>
		<span class={label}>Empty spool weight (g)</span>
		<input type="number" min="0" step="0.1" bind:value={form.tare_weight_g} class={input} />
	</label>
	<AcquisitionFields
		bind:acquisition={form.acquisition}
		bind:date={form.purchase_date}
		bind:from={form.vendor}
		bind:price={form.purchase_price}
		bind:discounted={form.discounted}
		bind:discountNote={form.discount_note}
		{input}
		{label}
	/>
	<label>
		<span class={label}>Currency</span>
		<input type="text" maxlength="3" bind:value={form.currency} class="{input} uppercase" />
	</label>
	<label>
		<span class={label}>Label code</span>
		<input type="text" bind:value={form.label_code} class="{input} font-mono" />
	</label>
	<label>
		<span class={label}>Lot number</span>
		<input type="text" bind:value={form.lot_number} class={input} />
	</label>
	<label class="col-span-2 sm:col-span-1">
		<span class={label}>Last dried</span>
		<input type="datetime-local" bind:value={form.last_dried_at} class={input} />
	</label>
	<label class="col-span-2">
		<span class={label}>Notes</span>
		<textarea rows="3" bind:value={form.notes} class={input}></textarea>
	</label>
	<p class="col-span-2 text-xs text-slate-500 dark:text-slate-400">
		The remaining weight changes only through the weight buttons, so every change is in the history.
	</p>
	{#if error}<p class="col-span-2 text-sm text-red-600" role="alert">{error}</p>{/if}
	<div class="col-span-2 flex justify-end gap-2">
		<button
			type="button"
			onclick={onCancel}
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
