<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Filament, Material } from '#lib/api/types.ts';
	import { untrack } from 'svelte';

	let {
		filament,
		materials,
		spoolCount,
		onSaved,
		onCancel
	}: {
		filament: Filament;
		materials: Material[];
		/** How many spools share this filament, for the warning. */
		spoolCount: number | null;
		onSaved: (f: Filament) => void;
		onCancel: () => void;
	} = $props();

	// The PUT replaces every field, so start from the whole filament.
	let form = $state(untrack(() => $state.snapshot(filament) as Filament));
	let saving = $state(false);
	let error = $state<string | null>(null);

	const blank = (v: string | null) => (v == null || v.trim() === '' ? null : v.trim());
	const num = (v: number | null | undefined) =>
		v === null || v === undefined || Number.isNaN(v) ? null : v;

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!form.brand_name.trim() || !form.color_name.trim()) {
			error = 'Brand and color are required.';
			return;
		}
		saving = true;
		error = null;
		try {
			const updated = await api.updateFilament(filament.id, {
				brand_name: form.brand_name,
				material_code: form.material_code,
				product_name: blank(form.product_name),
				color_name: form.color_name,
				color_hex: blank(form.color_hex),
				color_hex_secondary: blank(form.color_hex_secondary),
				diameter_mm: Number(form.diameter_mm),
				nominal_weight_g: num(form.nominal_weight_g),
				typical_tare_g: num(form.typical_tare_g),
				nozzle_temp_min_c: num(form.nozzle_temp_min_c),
				nozzle_temp_max_c: num(form.nozzle_temp_max_c),
				bed_temp_min_c: num(form.bed_temp_min_c),
				bed_temp_max_c: num(form.bed_temp_max_c),
				print_speed_min_mm_s: num(form.print_speed_min_mm_s),
				print_speed_max_mm_s: num(form.print_speed_max_mm_s),
				drying_temp_c: num(form.drying_temp_c),
				drying_time_min: num(form.drying_time_min),
				density_g_cm3: num(form.density_g_cm3),
				td_value: num(form.td_value),
				source_url: blank(form.source_url),
				image_url: blank(form.image_url)
			});
			onSaved(updated);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the filament.';
		} finally {
			saving = false;
		}
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

{#snippet pair(
	title: string,
	unit: string,
	minKey: 'nozzle_temp_min_c' | 'bed_temp_min_c' | 'print_speed_min_mm_s',
	maxKey: 'nozzle_temp_max_c' | 'bed_temp_max_c' | 'print_speed_max_mm_s'
)}
	<fieldset class="col-span-2 sm:col-span-1">
		<legend class={label}>{title} ({unit})</legend>
		<div class="flex items-center gap-2">
			<input type="number" placeholder="min" bind:value={form[minKey]} class={input} />
			<span class="mt-1 text-slate-400">–</span>
			<input type="number" placeholder="max" bind:value={form[maxKey]} class={input} />
		</div>
	</fieldset>
{/snippet}

<form onsubmit={save} class="grid grid-cols-2 gap-3">
	{#if spoolCount !== null && spoolCount > 1}
		<p
			class="col-span-2 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-100"
		>
			{spoolCount} spools use this filament. Your changes apply to all of them.
		</p>
	{/if}
	<label>
		<span class={label}>Brand *</span>
		<input type="text" bind:value={form.brand_name} class={input} required />
	</label>
	<label>
		<span class={label}>Material *</span>
		<select bind:value={form.material_code} class={input}>
			{#each materials as m (m.id)}<option value={m.code}>{m.code}</option>{/each}
		</select>
	</label>
	<label class="col-span-2 sm:col-span-1">
		<span class={label}>Product line</span>
		<input type="text" bind:value={form.product_name} class={input} />
	</label>
	<label class="col-span-2 sm:col-span-1">
		<span class={label}>Color *</span>
		<input type="text" bind:value={form.color_name} class={input} required />
	</label>
	<label>
		<span class={label}>Swatch</span>
		<div class="flex items-center gap-2">
			<input
				type="color"
				value={form.color_hex ?? '#888888'}
				oninput={(e) => (form.color_hex = e.currentTarget.value.toUpperCase())}
				class="mt-1 h-9 w-12 rounded border-0 bg-transparent"
				aria-label="Pick swatch color"
			/>
			<input
				type="text"
				bind:value={form.color_hex}
				placeholder="#RRGGBB"
				class="{input} font-mono"
			/>
		</div>
	</label>
	<label>
		<span class={label}>Diameter</span>
		<select bind:value={form.diameter_mm} class={input}>
			<option value={1.75}>1.75 mm</option>
			<option value={2.85}>2.85 mm</option>
			<option value={3}>3.00 mm</option>
		</select>
	</label>
	{@render pair('Nozzle', '°C', 'nozzle_temp_min_c', 'nozzle_temp_max_c')}
	{@render pair('Bed', '°C', 'bed_temp_min_c', 'bed_temp_max_c')}
	{@render pair('Speed', 'mm/s', 'print_speed_min_mm_s', 'print_speed_max_mm_s')}
	<fieldset class="col-span-2 sm:col-span-1">
		<legend class={label}>Drying (°C, minutes)</legend>
		<div class="flex items-center gap-2">
			<input type="number" placeholder="°C" bind:value={form.drying_temp_c} class={input} />
			<input type="number" placeholder="min" bind:value={form.drying_time_min} class={input} />
		</div>
	</fieldset>
	<label>
		<span class={label}>TD value (HueForge)</span>
		<input type="number" step="0.1" min="0.1" max="100" bind:value={form.td_value} class={input} />
	</label>
	<label>
		<span class={label}>Density (g/cm³)</span>
		<input type="number" step="0.01" min="0.01" bind:value={form.density_g_cm3} class={input} />
	</label>
	<label class="col-span-2">
		<span class={label}>Product page</span>
		<input type="url" bind:value={form.source_url} class={input} />
	</label>
	{#if error}<p class="col-span-2 text-sm text-red-600" role="alert">{error}</p>{/if}
	<div class="col-span-2 flex justify-end gap-2">
		<button
			type="button"
			onclick={onCancel}
			class="rounded-lg px-4 py-2 text-sm font-semibold ring-1 ring-slate-300 dark:ring-slate-700"
			>Cancel</button
		>
		<button
			type="submit"
			disabled={saving}
			class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
			>{saving ? 'Saving…' : 'Save'}</button
		>
	</div>
</form>
