<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Location, LocationKind, NewLocation } from '#lib/api/types.ts';

	let { locations = $bindable() }: { locations: Location[] } = $props();

	const kinds: { value: LocationKind; label: string }[] = [
		{ value: 'dry_box', label: 'Dry box' },
		{ value: 'ams_slot', label: 'AMS slot' },
		{ value: 'printer', label: 'Printer' },
		{ value: 'dryer', label: 'Dryer' },
		{ value: 'shelf', label: 'Shelf' },
		{ value: 'other', label: 'Other' }
	];
	const kindLabel = (k: LocationKind) => kinds.find((x) => x.value === k)?.label ?? k;

	const empty = (): NewLocation => ({ name: '', kind: 'dry_box', description: '', sort_order: 0 });
	// `null` = closed, 'new' = adding, otherwise the id being edited.
	let editingId = $state<string | null>(null);
	let form = $state<NewLocation>(empty());
	let confirmId = $state<string | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);

	function startAdd() {
		form = empty();
		form.sort_order = (locations.at(-1)?.sort_order ?? 0) + 1;
		editingId = 'new';
		error = null;
	}

	function startEdit(l: Location) {
		form = {
			name: l.name,
			kind: l.kind,
			description: l.description ?? '',
			sort_order: l.sort_order
		};
		editingId = l.id;
		error = null;
	}

	const sorted = (list: Location[]) =>
		[...list].sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name));

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (!form.name.trim()) return;
		busy = true;
		error = null;
		const body = {
			...form,
			name: form.name.trim(),
			description: form.description?.trim() || null,
			sort_order: Number(form.sort_order) || 0
		};
		try {
			if (editingId === 'new') {
				const created = await api.createLocation(body);
				locations = sorted([...locations, created]);
			} else if (editingId) {
				const updated = await api.updateLocation(editingId, body);
				locations = sorted(locations.map((l) => (l.id === updated.id ? updated : l)));
			}
			editingId = null;
		} catch (err) {
			error =
				err instanceof ApiError && err.status === 409
					? `There's already a location called "${body.name}".`
					: err instanceof ApiError
						? err.message
						: 'Could not save the location.';
		} finally {
			busy = false;
		}
	}

	async function remove(id: string) {
		busy = true;
		error = null;
		try {
			await api.deleteLocation(id);
			locations = locations.filter((l) => l.id !== id);
			confirmId = null;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not delete the location.';
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
	<form onsubmit={save} class="grid grid-cols-2 gap-3 py-3 sm:grid-cols-[2fr_1fr_5rem]">
		<label class="col-span-2 sm:col-span-1">
			<span class={label}>Name *</span>
			<input type="text" bind:value={form.name} class="{input} mt-1" required />
		</label>
		<label>
			<span class={label}>Kind</span>
			<select bind:value={form.kind} class="{input} mt-1">
				{#each kinds as k (k.value)}<option value={k.value}>{k.label}</option>{/each}
			</select>
		</label>
		<label>
			<span class={label}>Order</span>
			<input type="number" step="1" bind:value={form.sort_order} class="{input} mt-1" />
		</label>
		<label class="col-span-2 sm:col-span-3">
			<span class={label}>Description</span>
			<input type="text" bind:value={form.description} class="{input} mt-1" />
		</label>
		<div class="col-span-2 flex justify-end gap-2 sm:col-span-3">
			<button
				type="button"
				onclick={() => (editingId = null)}
				class="rounded-lg px-4 py-2 text-sm font-semibold ring-1 ring-slate-300 dark:ring-slate-700"
				>Cancel</button
			>
			<button
				type="submit"
				disabled={busy}
				class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
				>{editingId === 'new' ? 'Add' : 'Save'}</button
			>
		</div>
	</form>
{/snippet}

<div class="mb-1 flex items-center justify-between">
	<h2 class="font-semibold">Locations</h2>
	{#if editingId !== 'new'}
		<button type="button" class={link} onclick={startAdd}>+ Add location</button>
	{/if}
</div>
<p class="mb-2 text-sm text-slate-500 dark:text-slate-400">
	Where your spools live: dry boxes, AMS slots, shelves. Lower order numbers come first.
</p>

{#if editingId === 'new'}{@render editor()}{/if}

{#if locations.length === 0 && editingId !== 'new'}
	<p class="py-2 text-sm text-slate-500">No locations yet.</p>
{/if}
<ul class="divide-y divide-slate-100 dark:divide-slate-800">
	{#each locations as l (l.id)}
		<li>
			{#if editingId === l.id}
				{@render editor()}
			{:else}
				<div class="flex items-center gap-3 py-2 text-sm">
					<div class="min-w-0 flex-1">
						<p class="font-medium">{l.name}</p>
						<p class="truncate text-xs text-slate-500 dark:text-slate-400">
							{kindLabel(l.kind)}{l.description ? ` · ${l.description}` : ''}
						</p>
					</div>
					{#if confirmId === l.id}
						<span class="text-xs text-slate-600 dark:text-slate-300"
							>Spools here lose their location.</span
						>
						<button
							type="button"
							class="text-sm font-semibold text-red-600"
							disabled={busy}
							onclick={() => remove(l.id)}>Delete</button
						>
						<button type="button" class="text-sm" onclick={() => (confirmId = null)}>Cancel</button>
					{:else}
						<button type="button" class={link} onclick={() => startEdit(l)}>Edit</button>
						<button
							type="button"
							class="text-sm text-slate-500 hover:text-red-600"
							aria-label="Delete {l.name}"
							onclick={() => (confirmId = l.id)}>Delete</button
						>
					{/if}
				</div>
			{/if}
		</li>
	{/each}
</ul>
{#if error}<p class="mt-2 text-sm text-red-600" role="alert">{error}</p>{/if}
