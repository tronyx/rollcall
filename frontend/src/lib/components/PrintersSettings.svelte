<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type { NewPrinter, Printer } from '#lib/api/types.ts';

	let { printers = $bindable() }: { printers: Printer[] } = $props();

	const empty = (): NewPrinter => ({
		name: '',
		model: '',
		nozzle_diameter_mm: 0.4,
		is_default: false,
		notes: ''
	});
	let editingId = $state<string | null>(null);
	let form = $state<NewPrinter>(empty());
	let confirmId = $state<string | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);

	function startAdd() {
		form = empty();
		form.is_default = printers.length === 0;
		editingId = 'new';
		error = null;
	}

	function startEdit(p: Printer) {
		form = { ...p, model: p.model ?? '', notes: p.notes ?? '' };
		editingId = p.id;
		error = null;
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!form.name.trim()) return;
		busy = true;
		error = null;
		const body: NewPrinter = {
			name: form.name.trim(),
			model: form.model?.trim() || null,
			nozzle_diameter_mm: Number(form.nozzle_diameter_mm) || 0.4,
			is_default: form.is_default,
			notes: form.notes?.trim() || null
		};
		try {
			if (editingId === 'new') await api.createPrinter(body);
			else if (editingId) await api.updatePrinter(editingId, body);
			// Reload: a new default clears the flag on the others.
			printers = await api.printers();
			editingId = null;
		} catch (err) {
			error =
				err instanceof ApiError && err.status === 409
					? `There's already a printer called "${body.name}".`
					: err instanceof ApiError
						? err.message
						: 'Could not save the printer.';
		} finally {
			busy = false;
		}
	}

	async function remove(id: string) {
		busy = true;
		error = null;
		try {
			await api.deletePrinter(id);
			printers = printers.filter((p) => p.id !== id);
			confirmId = null;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not delete the printer.';
		} finally {
			busy = false;
		}
	}

	const input =
		'w-full rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
	const label = 'block text-xs font-medium text-slate-600 dark:text-slate-300';
	const link = 'text-sm font-semibold text-orange-700 dark:text-orange-400';
</script>

{#snippet editor()}
	<form onsubmit={save} class="grid grid-cols-2 gap-3 py-3">
		<label>
			<span class={label}>Name *</span>
			<input type="text" bind:value={form.name} placeholder="X1C" class="{input} mt-1" required />
		</label>
		<label>
			<span class={label}>Model</span>
			<input
				type="text"
				bind:value={form.model}
				placeholder="Bambu Lab X1 Carbon"
				class="{input} mt-1"
			/>
		</label>
		<label>
			<span class={label}>Nozzle (mm)</span>
			<input
				type="number"
				step="0.05"
				min="0.1"
				bind:value={form.nozzle_diameter_mm}
				class="{input} mt-1"
			/>
		</label>
		<label class="mt-6 flex items-center gap-2 text-sm">
			<input type="checkbox" bind:checked={form.is_default} class="size-4 accent-orange-600" />
			Default printer
		</label>
		<label class="col-span-2">
			<span class={label}>Notes</span>
			<input type="text" bind:value={form.notes} class="{input} mt-1" />
		</label>
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
				disabled={busy}
				class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
			>
				{editingId === 'new' ? 'Add' : 'Save'}
			</button>
		</div>
	</form>
{/snippet}

<div class="mb-1 flex items-center justify-between">
	<h2 class="font-semibold">Printers</h2>
	{#if editingId !== 'new'}
		<button type="button" class={link} onclick={startAdd}>+ Add printer</button>
	{/if}
</div>
<p class="mb-2 text-sm text-slate-500 dark:text-slate-400">
	Your printers, for saving tuned settings per printer later.
</p>

{#if editingId === 'new'}{@render editor()}{/if}

{#if printers.length === 0 && editingId !== 'new'}
	<p class="py-2 text-sm text-slate-500">No printers yet.</p>
{/if}
<ul class="divide-y divide-slate-100 dark:divide-slate-800">
	{#each printers as p (p.id)}
		<li>
			{#if editingId === p.id}
				{@render editor()}
			{:else}
				<div class="flex items-center gap-3 py-2 text-sm">
					<div class="min-w-0 flex-1">
						<p class="font-medium">
							{p.name}
							{#if p.is_default}
								<span
									class="ml-1 rounded-full bg-orange-100 px-2 py-0.5 text-xs font-medium text-orange-800 dark:bg-orange-900/60 dark:text-orange-200"
								>
									default
								</span>
							{/if}
						</p>
						<p class="truncate text-xs text-slate-500 dark:text-slate-400">
							{p.model ? `${p.model} · ` : ''}{p.nozzle_diameter_mm} mm nozzle{p.notes
								? ` · ${p.notes}`
								: ''}
						</p>
					</div>
					{#if confirmId === p.id}
						<button
							type="button"
							class="text-sm font-semibold text-red-600"
							disabled={busy}
							onclick={() => remove(p.id)}
						>
							Delete
						</button>
						<button type="button" class="text-sm" onclick={() => (confirmId = null)}>Cancel</button>
					{:else}
						<button type="button" class={link} onclick={() => startEdit(p)}>Edit</button>
						<button
							type="button"
							class="text-sm text-slate-500 hover:text-red-600"
							aria-label="Delete {p.name}"
							onclick={() => (confirmId = p.id)}
						>
							Delete
						</button>
					{/if}
				</div>
			{/if}
		</li>
	{/each}
</ul>
{#if error}<p class="mt-2 text-sm text-red-600" role="alert">{error}</p>{/if}
