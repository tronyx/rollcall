<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type {
		FieldSource,
		Location,
		Material,
		ParseDraft,
		ParseResult,
		Spool,
		SpoolStatus
	} from '#lib/api/types.ts';
	import ColorSwatch from './ColorSwatch.svelte';

	let {
		open = $bindable(false),
		materials,
		locations,
		onCreated
	}: {
		open: boolean;
		materials: Material[];
		locations: Location[];
		onCreated: (s: Spool) => void;
	} = $props();

	type Form = {
		brand_name: string;
		material_code: string;
		product_name: string;
		color_name: string;
		color_hex: string;
		diameter_mm: number;
		nozzle_temp_min_c: number | null;
		nozzle_temp_max_c: number | null;
		bed_temp_min_c: number | null;
		bed_temp_max_c: number | null;
		print_speed_min_mm_s: number | null;
		print_speed_max_mm_s: number | null;
		drying_temp_c: number | null;
		drying_time_h: number | null;
		image_url: string;
		initial_weight_g: number | null;
		remaining_weight_g: number | null;
		tare_weight_g: number | null;
		purchase_price: number | null;
		purchase_date: string;
		location_id: string;
		status: SpoolStatus;
		label_code: string;
		notes: string;
	};

	const today = () => new Date().toISOString().slice(0, 10);
	const emptyForm = (): Form => ({
		brand_name: '',
		material_code: 'PLA',
		product_name: '',
		color_name: '',
		color_hex: '#888888',
		diameter_mm: 1.75,
		nozzle_temp_min_c: null,
		nozzle_temp_max_c: null,
		bed_temp_min_c: null,
		bed_temp_max_c: null,
		print_speed_min_mm_s: null,
		print_speed_max_mm_s: null,
		drying_temp_c: null,
		drying_time_h: null,
		image_url: '',
		initial_weight_g: 1000,
		remaining_weight_g: null,
		tare_weight_g: null,
		purchase_price: null,
		purchase_date: today(),
		location_id: '',
		status: 'sealed',
		label_code: '',
		notes: ''
	});

	let dialog: HTMLDialogElement | undefined = $state();
	let step = $state<'url' | 'review'>('url');
	let url = $state('');
	let parsing = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);
	let parsed = $state<ParseResult | null>(null);
	let form = $state<Form>(emptyForm());
	let hexTouched = $state(false);

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) dialog.showModal();
		if (!open && dialog.open) dialog.close();
	});

	function reset() {
		step = 'url';
		url = '';
		parsed = null;
		error = null;
		hexTouched = false;
		form = emptyForm();
	}

	function close() {
		open = false;
		reset();
	}

	/** Guessed fields get an amber "check this" hint; values from structured data don't. */
	const GUESSY: FieldSource[] = ['url', 'derived', 'page_text', 'llm', 'title'];
	function hint(field: keyof ParseDraft): 'missing' | 'check' | null {
		if (!parsed) return null;
		const src = parsed.sources[field];
		if (!src) return parsed.missing.includes(field) ? 'missing' : null;
		return GUESSY.includes(src) ? 'check' : null;
	}

	async function parse(e: SubmitEvent) {
		e.preventDefault();
		parsing = true;
		error = null;
		try {
			const res = await api.parseUrl(url.trim());
			parsed = res;
			applyDraft(res.draft);
			step = 'review';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not read that page.';
		} finally {
			parsing = false;
		}
	}

	function applyDraft(d: ParseDraft) {
		const f = emptyForm();
		form = {
			...f,
			brand_name: d.brand_name ?? '',
			material_code:
				d.material_code && materials.some((m) => m.code === d.material_code)
					? d.material_code
					: f.material_code,
			product_name: d.product_name ?? '',
			color_name: d.color_name ?? '',
			color_hex: d.color_hex ?? f.color_hex,
			diameter_mm: d.diameter_mm ?? 1.75,
			nozzle_temp_min_c: d.nozzle_temp_min_c,
			nozzle_temp_max_c: d.nozzle_temp_max_c,
			bed_temp_min_c: d.bed_temp_min_c,
			bed_temp_max_c: d.bed_temp_max_c,
			print_speed_min_mm_s: d.print_speed_min_mm_s,
			print_speed_max_mm_s: d.print_speed_max_mm_s,
			drying_temp_c: d.drying_temp_c,
			drying_time_h:
				d.drying_time_min != null ? Math.round((d.drying_time_min / 60) * 10) / 10 : null,
			image_url: d.image_url ?? '',
			initial_weight_g: d.nominal_weight_g ?? 1000,
			purchase_price: d.price
		};
		hexTouched = !!d.color_hex;
	}

	const ESSENTIALS: (keyof ParseDraft)[] = [
		'brand_name',
		'material_code',
		'color_name',
		'nozzle_temp_min_c',
		'bed_temp_min_c'
	];
	const essentialsMissing = $derived(
		parsed ? ESSENTIALS.filter((f) => parsed!.missing.includes(f)).length : 0
	);

	const num = (v: number | null | undefined) =>
		v === null || v === undefined || Number.isNaN(v) ? null : v;
	const text = (v: string) => (v.trim() ? v.trim() : null);

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!form.initial_weight_g) return;
		saving = true;
		error = null;
		try {
			const usedLlm = parsed && Object.values(parsed.sources).includes('llm');
			const spool = await api.createSpool({
				filament: {
					brand_name: form.brand_name.trim(),
					material_code: form.material_code,
					product_name: text(form.product_name),
					color_name: form.color_name.trim(),
					color_hex: hexTouched ? form.color_hex : null,
					diameter_mm: form.diameter_mm,
					nominal_weight_g: num(form.initial_weight_g),
					nozzle_temp_min_c: num(form.nozzle_temp_min_c),
					nozzle_temp_max_c: num(form.nozzle_temp_max_c),
					bed_temp_min_c: num(form.bed_temp_min_c),
					bed_temp_max_c: num(form.bed_temp_max_c),
					print_speed_min_mm_s: num(form.print_speed_min_mm_s),
					print_speed_max_mm_s: num(form.print_speed_max_mm_s),
					drying_temp_c: num(form.drying_temp_c),
					drying_time_min: form.drying_time_h ? Math.round(form.drying_time_h * 60) : null,
					source_url: parsed?.final_url ?? (text(url) || null),
					image_url: text(form.image_url),
					spec_source: parsed ? (usedLlm ? 'llm' : 'scraped') : 'manual',
					scrape_raw: parsed ?? undefined
				},
				initial_weight_g: form.initial_weight_g,
				remaining_weight_g: num(form.remaining_weight_g),
				tare_weight_g: num(form.tare_weight_g),
				purchase_price: num(form.purchase_price),
				currency: parsed?.draft.currency ?? null,
				purchase_date: form.purchase_date || null,
				location_id: form.location_id || null,
				status: form.status,
				label_code: text(form.label_code),
				notes: text(form.notes)
			});
			onCreated(spool);
			close();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the spool.';
		} finally {
			saving = false;
		}
	}

	const input =
		'mt-1 w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const ringFor = (h: 'missing' | 'check' | null) =>
		h === 'missing' ? '!ring-red-400' : h === 'check' ? '!ring-amber-400' : '';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
</script>

{#snippet hintBadge(h: 'missing' | 'check' | null)}
	{#if h === 'missing'}<span class="ml-1 text-red-600">not found</span>{/if}
	{#if h === 'check'}<span class="ml-1 text-amber-600">check</span>{/if}
{/snippet}

{#snippet rangeField(
	title: string,
	unit: string,
	minKey: keyof Form & keyof ParseDraft,
	maxKey: keyof Form & keyof ParseDraft
)}
	<fieldset>
		<legend class={label}>{title} ({unit}) {@render hintBadge(hint(minKey))}</legend>
		<div class="flex items-center gap-1">
			<input
				type="number"
				inputmode="numeric"
				class="{input} {ringFor(hint(minKey))}"
				placeholder="min"
				aria-label="{title} minimum"
				bind:value={form[minKey] as number | null}
			/>
			<span class="mt-1 text-slate-400">–</span>
			<input
				type="number"
				inputmode="numeric"
				class="{input} {ringFor(hint(maxKey))}"
				placeholder="max"
				aria-label="{title} maximum"
				bind:value={form[maxKey] as number | null}
			/>
		</div>
	</fieldset>
{/snippet}

<dialog
	bind:this={dialog}
	onclose={close}
	class="m-0 mt-auto max-h-[92dvh] w-full max-w-none overflow-y-auto rounded-t-2xl bg-slate-50 p-0 text-slate-900 shadow-2xl backdrop:bg-black/50 sm:m-auto sm:max-w-2xl sm:rounded-2xl dark:bg-slate-900 dark:text-slate-100"
	aria-labelledby="add-spool-title"
>
	<div
		class="sticky top-0 z-10 flex items-center justify-between border-b border-slate-200 bg-slate-50/95 px-4 py-3 backdrop-blur dark:border-slate-800 dark:bg-slate-900/95"
	>
		<h2 id="add-spool-title" class="text-lg font-semibold">
			{step === 'url' ? 'Add spool' : 'Review and save'}
		</h2>
		<button
			type="button"
			onclick={close}
			class="rounded-lg px-2 text-2xl leading-none text-slate-500"
			aria-label="Close">×</button
		>
	</div>

	{#if step === 'url'}
		<form onsubmit={parse} class="space-y-4 p-4">
			<label class="block">
				<span class={label}>Product link (Amazon or manufacturer page)</span>
				<input
					type="url"
					required
					inputmode="url"
					placeholder="https://us.polymaker.com/products/…"
					bind:value={url}
					class={input}
				/>
			</label>
			<p class="text-xs text-slate-500 dark:text-slate-400">
				Rollcall reads the page and fills in what it can. You review everything before saving.
			</p>
			{#if error}<p class="text-sm text-red-600" role="alert">{error}</p>{/if}
			<div class="flex flex-col gap-2 sm:flex-row-reverse">
				<button
					type="submit"
					disabled={parsing}
					class="rounded-lg bg-orange-600 px-4 py-2.5 font-semibold text-white disabled:opacity-60"
				>
					{parsing ? 'Reading page…' : 'Fetch details'}
				</button>
				<button
					type="button"
					class="rounded-lg px-4 py-2.5 font-medium text-slate-700 ring-1 ring-slate-300 dark:text-slate-200 dark:ring-slate-700"
					onclick={() => (step = 'review')}>Enter manually</button
				>
			</div>
		</form>
	{:else}
		<form onsubmit={save} class="space-y-5 p-4">
			{#if parsed}
				<div
					class="flex gap-3 rounded-xl bg-white p-3 ring-1 ring-slate-200 dark:bg-slate-800 dark:ring-slate-700"
				>
					{#if form.image_url}
						<img
							src={form.image_url}
							alt=""
							class="size-16 shrink-0 rounded-lg object-cover"
							referrerpolicy="no-referrer"
						/>
					{/if}
					<div class="min-w-0 text-sm">
						<p class="line-clamp-2 font-medium">{parsed.draft.title ?? parsed.url}</p>
						<p
							class="mt-1 text-xs {parsed.complete
								? 'text-emerald-700 dark:text-emerald-400'
								: 'text-amber-700 dark:text-amber-400'}"
						>
							{parsed.complete
								? 'All key details found. Give them a quick look.'
								: `${essentialsMissing} key details weren't found. They're marked below.`}
						</p>
					</div>
				</div>
				{#each parsed.warnings as w (w)}
					<p
						class="rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-100"
					>
						{w}
					</p>
				{/each}
			{/if}

			<section class="grid grid-cols-2 gap-3">
				<h3 class="col-span-2 text-sm font-semibold">Filament</h3>
				<label class="col-span-2 sm:col-span-1">
					<span class={label}>Brand * {@render hintBadge(hint('brand_name'))}</span>
					<input
						required
						bind:value={form.brand_name}
						class="{input} {ringFor(hint('brand_name'))}"
					/>
				</label>
				<label class="col-span-2 sm:col-span-1">
					<span class={label}>Material * {@render hintBadge(hint('material_code'))}</span>
					<select
						required
						bind:value={form.material_code}
						class="{input} {ringFor(hint('material_code'))}"
					>
						{#each materials as m (m.id)}<option value={m.code}>{m.display_name}</option>{/each}
					</select>
				</label>
				<label class="col-span-2">
					<span class={label}>Product line {@render hintBadge(hint('product_name'))}</span>
					<input
						bind:value={form.product_name}
						placeholder="e.g. PLA Basic, PolyTerra"
						class="{input} {ringFor(hint('product_name'))}"
					/>
				</label>
				<label class="col-span-2 sm:col-span-1">
					<span class={label}>Color name * {@render hintBadge(hint('color_name'))}</span>
					<input
						required
						bind:value={form.color_name}
						class="{input} {ringFor(hint('color_name'))}"
					/>
				</label>
				<div class="col-span-2 sm:col-span-1">
					<span class={label}>Swatch {@render hintBadge(hint('color_hex'))}</span>
					<div class="mt-1 flex items-center gap-2">
						<ColorSwatch
							hex={hexTouched ? form.color_hex : null}
							label="Selected color"
							size="md"
						/>
						<input
							type="color"
							bind:value={form.color_hex}
							oninput={() => (hexTouched = true)}
							class="h-11 w-16 cursor-pointer rounded-lg bg-transparent"
							aria-label="Pick swatch color"
						/>
						<span class="font-mono text-xs text-slate-500"
							>{hexTouched ? form.color_hex.toUpperCase() : 'none'}</span
						>
					</div>
				</div>
				<label>
					<span class={label}>Diameter</span>
					<select bind:value={form.diameter_mm} class={input}>
						<option value={1.75}>1.75 mm</option>
						<option value={2.85}>2.85 mm</option>
						<option value={3}>3.00 mm</option>
					</select>
				</label>
			</section>

			<details
				open={!!parsed}
				class="rounded-xl bg-white p-3 ring-1 ring-slate-200 dark:bg-slate-800/50 dark:ring-slate-700"
			>
				<summary class="cursor-pointer text-sm font-semibold">Manufacturer print settings</summary>
				<div class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
					{@render rangeField('Nozzle temp', '°C', 'nozzle_temp_min_c', 'nozzle_temp_max_c')}
					{@render rangeField('Bed temp', '°C', 'bed_temp_min_c', 'bed_temp_max_c')}
					{@render rangeField(
						'Print speed',
						'mm/s',
						'print_speed_min_mm_s',
						'print_speed_max_mm_s'
					)}
					<fieldset>
						<legend class={label}>Drying {@render hintBadge(hint('drying_temp_c'))}</legend>
						<div class="flex items-center gap-1">
							<input
								type="number"
								inputmode="numeric"
								placeholder="°C"
								aria-label="Drying temperature"
								bind:value={form.drying_temp_c}
								class="{input} {ringFor(hint('drying_temp_c'))}"
							/>
							<input
								type="number"
								inputmode="decimal"
								step="0.5"
								placeholder="hours"
								aria-label="Drying hours"
								bind:value={form.drying_time_h}
								class="{input} {ringFor(hint('drying_time_min'))}"
							/>
						</div>
					</fieldset>
				</div>
			</details>

			<section class="grid grid-cols-2 gap-3">
				<h3 class="col-span-2 text-sm font-semibold">This spool</h3>
				<label>
					<span class={label}>Net weight (g) *</span>
					<input
						type="number"
						inputmode="decimal"
						required
						min="1"
						bind:value={form.initial_weight_g}
						class={input}
					/>
				</label>
				<label>
					<span class={label}>Remaining (g)</span>
					<input
						type="number"
						inputmode="decimal"
						min="0"
						placeholder="same as net"
						bind:value={form.remaining_weight_g}
						class={input}
					/>
				</label>
				<label>
					<span class={label}>Empty spool weight (g)</span>
					<input
						type="number"
						inputmode="decimal"
						min="0"
						placeholder="for weigh-ins"
						bind:value={form.tare_weight_g}
						class={input}
					/>
				</label>
				<label>
					<span class={label}>Price</span>
					<input
						type="number"
						inputmode="decimal"
						min="0"
						step="0.01"
						bind:value={form.purchase_price}
						class={input}
					/>
				</label>
				<label>
					<span class={label}>Purchased</span>
					<input type="date" bind:value={form.purchase_date} class={input} />
				</label>
				<label>
					<span class={label}>Status</span>
					<select bind:value={form.status} class={input}>
						<option value="sealed">Sealed</option>
						<option value="active">Active</option>
						<option value="drying">Drying</option>
					</select>
				</label>
				<label>
					<span class={label}>Location</span>
					<select bind:value={form.location_id} class={input}>
						<option value="">None</option>
						{#each locations as l (l.id)}<option value={l.id}>{l.name}</option>{/each}
					</select>
				</label>
				<label>
					<span class={label}>Label code</span>
					<input bind:value={form.label_code} placeholder="R-0042" class={input} />
				</label>
				<label class="col-span-2">
					<span class={label}>Notes</span>
					<textarea rows="2" bind:value={form.notes} class={input}></textarea>
				</label>
			</section>

			{#if error}<p class="text-sm text-red-600" role="alert">{error}</p>{/if}
			<div
				class="sticky bottom-0 -mx-4 flex gap-2 border-t border-slate-200 bg-slate-50/95 px-4 py-3 backdrop-blur dark:border-slate-800 dark:bg-slate-900/95"
			>
				<button
					type="button"
					onclick={reset}
					class="rounded-lg px-4 py-2.5 font-medium ring-1 ring-slate-300 dark:ring-slate-700"
					>Back</button
				>
				<button
					type="submit"
					disabled={saving}
					class="flex-1 rounded-lg bg-orange-600 px-4 py-2.5 font-semibold text-white disabled:opacity-60"
				>
					{saving ? 'Saving…' : 'Save spool'}
				</button>
			</div>
		</form>
	{/if}
</dialog>
