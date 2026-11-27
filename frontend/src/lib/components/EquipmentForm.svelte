<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type {
		AcquisitionKind,
		Brand,
		Equipment,
		EquipmentCategory,
		Printer
	} from '#lib/api/types.ts';
	import AcquisitionFields from '#lib/components/AcquisitionFields.svelte';
	import { KIND_SUGGESTIONS } from '#lib/util/equipment.ts';
	import { untrack } from 'svelte';

	let {
		item = null,
		category = 'equipment',
		printerId = '',
		brands,
		printers,
		defaultCurrency,
		onSaved,
		onDeleted,
		onCancel
	}: {
		/** The item to edit, or null to add one. */
		item?: Equipment | null;
		/** For a new item: the starting category and printer. */
		category?: EquipmentCategory;
		printerId?: string;
		brands: Brand[];
		printers: Printer[];
		defaultCurrency: string;
		onSaved: (e: Equipment) => void;
		onDeleted: (id: string) => void;
		onCancel: () => void;
	} = $props();

	const start = untrack(() => (item ? ($state.snapshot(item) as Equipment) : null));
	let form = $state({
		category: start?.category ?? untrack(() => category),
		kind: start?.kind ?? '',
		name: start?.name ?? '',
		brand_name: start?.brand_name ?? '',
		model: start?.model ?? '',
		printer_id: start?.printer_id ?? untrack(() => printerId),
		quantity: (start?.quantity ?? 1) as number | null,
		retired: start?.retired ?? false,
		acquisition: (start?.acquisition ?? 'bought') as AcquisitionKind,
		acquired_on: start?.acquired_on ?? '',
		acquired_from: start?.acquired_from ?? '',
		price: start?.price ?? null,
		currency: start?.currency ?? untrack(() => defaultCurrency),
		discounted: start?.discounted ?? false,
		discount_note: start?.discount_note ?? '',
		notes: start?.notes ?? ''
	});
	let saving = $state(false);
	let error = $state<string | null>(null);
	let confirmDelete = $state(false);

	const blank = (v: string) => (v.trim() === '' ? null : v.trim());
	const num = (v: number | null | undefined) =>
		v === null || v === undefined || Number.isNaN(v) ? null : v;

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!form.name.trim()) {
			error = 'Give it a name.';
			return;
		}
		saving = true;
		error = null;
		const body = {
			category: form.category,
			kind: blank(form.kind),
			name: form.name.trim(),
			brand_name: blank(form.brand_name),
			printer_id: form.printer_id || null,
			model: blank(form.model),
			quantity: num(form.quantity) ?? 1,
			retired: form.retired,
			acquisition: form.acquisition,
			acquired_on: form.acquired_on || null,
			acquired_from: blank(form.acquired_from),
			price: num(form.price),
			currency: blank(form.currency),
			discounted: form.discounted,
			discount_note: form.discounted ? blank(form.discount_note) : null,
			notes: blank(form.notes)
		};
		try {
			onSaved(item ? await api.updateEquipment(item.id, body) : await api.createEquipment(body));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save.';
		} finally {
			saving = false;
		}
	}

	async function remove() {
		if (!item) return;
		try {
			await api.deleteEquipment(item.id);
			onDeleted(item.id);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not delete.';
		}
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

<form onsubmit={save} class="grid grid-cols-2 gap-3">
	<label>
		<span class={label}>Type</span>
		<select bind:value={form.category} class={input}>
			<option value="equipment">Equipment</option>
			<option value="accessory">Accessory</option>
		</select>
	</label>
	<label>
		<span class={label}>Kind</span>
		<input
			type="text"
			list="kind-suggestions"
			bind:value={form.kind}
			placeholder={KIND_SUGGESTIONS[form.category][0]}
			class={input}
		/>
		<datalist id="kind-suggestions">
			{#each KIND_SUGGESTIONS[form.category] as k (k)}<option value={k}></option>{/each}
		</datalist>
	</label>
	<label class="col-span-2">
		<span class={label}>Name *</span>
		<input
			type="text"
			bind:value={form.name}
			placeholder="Hardened steel nozzle 0.4 mm"
			class={input}
			required
		/>
	</label>
	<label>
		<span class={label}>Brand</span>
		<input type="text" list="brand-suggestions" bind:value={form.brand_name} class={input} />
		<datalist id="brand-suggestions">
			{#each brands as b (b.id)}<option value={b.name}></option>{/each}
		</datalist>
	</label>
	<label>
		<span class={label}>Model or part number</span>
		<input type="text" bind:value={form.model} class={input} />
	</label>
	<label>
		<span class={label}>For printer</span>
		<select bind:value={form.printer_id} class={input}>
			<option value="">Any printer</option>
			{#each printers as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
		</select>
	</label>
	<label>
		<span class={label}>Quantity</span>
		<input type="number" min="0" step="1" bind:value={form.quantity} class={input} />
	</label>
	<AcquisitionFields
		bind:acquisition={form.acquisition}
		bind:date={form.acquired_on}
		bind:from={form.acquired_from}
		bind:price={form.price}
		bind:discounted={form.discounted}
		bind:discountNote={form.discount_note}
		{input}
		{label}
	/>
	<label>
		<span class={label}>Currency</span>
		<input type="text" maxlength="3" bind:value={form.currency} class="{input} uppercase" />
	</label>
	<label class="col-span-2">
		<span class={label}>Notes</span>
		<textarea rows="2" bind:value={form.notes} class={input}></textarea>
	</label>
	{#if item}
		<label class="col-span-2 flex items-center gap-2 text-sm">
			<input type="checkbox" bind:checked={form.retired} class="size-4 accent-orange-600" />
			Retired (worn out, sold or given away). Hidden unless you show retired items.
		</label>
	{/if}
	{#if error}<p class="col-span-2 text-sm text-red-600" role="alert">{error}</p>{/if}
	<div class="col-span-2 flex flex-wrap items-center justify-between gap-2">
		<div>
			{#if item}
				{#if confirmDelete}
					<span class="text-sm">Delete for good?</span>
					<button type="button" class="ml-2 text-sm font-semibold text-red-600" onclick={remove}>
						Delete
					</button>
					<button
						type="button"
						class="ml-2 text-sm font-semibold"
						onclick={() => (confirmDelete = false)}
					>
						Keep
					</button>
				{:else}
					<button
						type="button"
						class="text-sm font-medium text-slate-500 hover:text-red-600"
						onclick={() => (confirmDelete = true)}
					>
						Delete…
					</button>
				{/if}
			{/if}
		</div>
		<div class="flex gap-2">
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
	</div>
</form>
