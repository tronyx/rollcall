<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Print, PrintSlot, Spool } from '#lib/api/types.ts';
	import ColorSwatch from '#lib/components/ColorSwatch.svelte';
	import { dateTime, grams } from '#lib/util/format.ts';
	import { gcodeSummary, spoolChoices, spoolLabel } from '#lib/util/prints.ts';
	import { onMount } from 'svelte';

	let prints = $state<Print[]>([]);
	let spools = $state<Spool[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let onlyPending = $state(page.url.searchParams.has('pending'));

	// Per slot ("<print id>:<slot>"): the spool picked and whether to remember the preset.
	let picked = $state<Record<string, string>>({});
	let remember = $state<Record<string, boolean>>({});
	let busy = $state<string | null>(null);
	let confirmDelete = $state<string | null>(null);

	let fileInput = $state<HTMLInputElement>();
	let uploading = $state(false);
	let notice = $state<string | null>(null);

	const origin = page.url.origin;
	const scriptLine = $derived(
		`"python3" "/path/to/rollcall-print.py" --url ${origin} --token YOUR_TOKEN`
	);
	let copied = $state(false);

	async function load() {
		try {
			[prints, spools] = await Promise.all([api.prints({ limit: 100 }), api.spools()]);
			error = null;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load prints.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	const pendingCount = $derived(
		prints.filter((p) => p.slots.some((s) => s.status === 'pending')).length
	);
	const shown = $derived(
		onlyPending ? prints.filter((p) => p.slots.some((s) => s.status === 'pending')) : prints
	);

	const key = (p: Print, s: PrintSlot) => `${p.id}:${s.slot}`;

	function replace(updated: Print) {
		prints = prints.map((p) => (p.id === updated.id ? updated : p));
	}

	async function act(k: string, fn: () => Promise<Print | void>) {
		busy = k;
		error = null;
		try {
			const result = await fn();
			if (result) replace(result);
			// Weights changed: refresh the spool list for the pickers.
			spools = await api.spools();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'That didn’t work.';
		} finally {
			busy = null;
		}
	}

	function log(p: Print, s: PrintSlot) {
		const k = key(p, s);
		const spoolId = picked[k] ?? spoolChoices(s, spools)[0]?.id;
		if (!spoolId) return;
		const spool = spools.find((x) => x.id === spoolId);
		notice = null;
		act(k, async () => {
			const updated = await api.assignPrintSlot(p.id, s.slot, {
				spool_id: spoolId,
				remember: remember[k] ?? true
			});
			// With "Needs a spool" showing, the print may drop out of the list.
			if (spool) notice = `Took ${grams(s.grams)} from ${spoolLabel(spool)} for ${p.name}.`;
			return updated;
		});
	}

	async function removePrint(p: Print) {
		await act(p.id, async () => {
			await api.deletePrint(p.id);
			prints = prints.filter((x) => x.id !== p.id);
		});
		confirmDelete = null;
	}

	async function upload(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const files = [...(input.files ?? [])];
		input.value = '';
		uploading = true;
		error = null;
		notice = null;
		const added: string[] = [];
		for (const f of files) {
			try {
				const p = await api.logPrint(f.name, gcodeSummary(await f.text()));
				if (prints.some((x) => x.id === p.id)) replace(p);
				else prints = [p, ...prints];
				added.push(p.name);
			} catch (err) {
				error = `${f.name}: ${err instanceof ApiError ? err.message : 'could not read it'}`;
			}
		}
		uploading = false;
		if (added.length) {
			notice = `Added ${added.join(', ')}.`;
			spools = await api.spools().catch(() => spools);
		}
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(scriptLine);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			// Selecting the text by hand still works.
		}
	}

	const card =
		'rounded-2xl bg-white p-4 shadow-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800';
	const small = 'text-sm font-semibold text-orange-700 dark:text-orange-400';
	const select =
		'min-w-0 flex-1 rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700';
</script>

<svelte:head>
	<title>Prints · Rollcall</title>
</svelte:head>

<nav class="mb-4">
	<a
		href={resolve('/')}
		class="text-sm font-medium text-orange-700 hover:underline dark:text-orange-400"
	>
		← All spools
	</a>
</nav>

<header class="mb-4 flex flex-wrap items-center justify-between gap-3">
	<h1 class="text-xl font-bold tracking-tight">🧾 Prints</h1>
	<button
		type="button"
		onclick={() => fileInput?.click()}
		disabled={uploading}
		class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-orange-700 disabled:opacity-50"
	>
		{uploading ? 'Reading…' : '+ Add from G-code'}
	</button>
	<input
		bind:this={fileInput}
		type="file"
		accept=".gcode,.gco,text/x-gcode"
		multiple
		class="hidden"
		onchange={upload}
		aria-label="G-code files"
	/>
</header>

<details class="{card} mb-4" open={!loading && prints.length === 0}>
	<summary class="cursor-pointer font-semibold">Log prints from your slicer automatically</summary>
	<div class="mt-3 space-y-3 text-sm text-slate-700 dark:text-slate-300">
		<p>
			OrcaSlicer and Bambu Studio can run a script each time they export G-code. Rollcall's script
			sends the grams of each filament here and deducts them from the right spool. It needs Python
			3.
		</p>
		<ol class="list-decimal space-y-2 pl-5">
			<li>
				Download <a href="/rollcall-print.py" download class={small}>rollcall-print.py</a> and save it
				somewhere it can stay.
			</li>
			<li>
				Create an API token under <a href={resolve('/settings')} class={small}>
					Settings › API tokens
				</a>, and copy it.
			</li>
			<li>
				In the slicer, open the process settings, then <strong>
					Others › Post-processing Scripts
				</strong>, and add this line, with the script's real path and your token:
				<div class="mt-2 flex gap-2">
					<code
						class="min-w-0 flex-1 overflow-x-auto rounded-lg bg-slate-100 px-3 py-2 text-xs whitespace-nowrap dark:bg-slate-800"
					>
						{scriptLine}
					</code>
					<button type="button" class={small} onclick={copy}>{copied ? 'Copied' : 'Copy'}</button>
				</div>
				<p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
					On Windows, use the full path to <code>python.exe</code>. Save the process preset so every
					print uses it.
				</p>
			</li>
			<li>
				So Rollcall knows which spool a print used, <strong>export the filament's preset</strong> from
				its tuned settings on the spool page, or pick the spool here the first time. Rollcall remembers
				the preset after that.
			</li>
		</ol>
		<p class="text-xs text-slate-500 dark:text-slate-400">
			A preset is logged by itself when its filament has one open spool (or one sealed spool).
			Otherwise the print waits here for you to pick one. The script runs when you export or send a
			print, so if you don't print it after all, delete it here to put the filament back.
		</p>
	</div>
</details>

<div class="mb-4 flex gap-1" role="tablist" aria-label="Show">
	{#each [{ id: false, label: 'All' }, { id: true, label: 'Needs a spool' }] as t (t.label)}
		<button
			type="button"
			role="tab"
			aria-selected={onlyPending === t.id}
			onclick={() => (onlyPending = t.id)}
			class="rounded-full px-3 py-1.5 text-sm font-medium {onlyPending === t.id
				? 'bg-orange-600 text-white'
				: 'text-slate-700 ring-1 ring-slate-300 hover:bg-slate-50 dark:text-slate-200 dark:ring-slate-700 dark:hover:bg-slate-800'}"
		>
			{t.label}
			{#if t.id}<span class="tabular-nums opacity-75">{pendingCount}</span>{/if}
		</button>
	{/each}
</div>

{#if error}
	<p
		class="mb-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-800 dark:bg-red-950 dark:text-red-100"
		role="alert"
	>
		{error}
	</p>
{/if}
{#if notice}
	<p
		class="mb-4 rounded-lg bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:bg-emerald-950 dark:text-emerald-100"
		role="status"
	>
		{notice}
	</p>
{/if}

{#if loading}
	<p class="py-12 text-center text-slate-500">Loading…</p>
{:else if !shown.length}
	<div
		class="rounded-2xl border-2 border-dashed border-slate-300 py-12 text-center dark:border-slate-700"
	>
		<p class="font-medium">{onlyPending ? 'Every print has its spool.' : 'No prints yet.'}</p>
		{#if !onlyPending}
			<p class="mt-1 text-sm text-slate-500">
				Set up the slicer script above, or add a G-code file you printed.
			</p>
		{/if}
	</div>
{:else}
	<ul class="space-y-3">
		{#each shown as p (p.id)}
			<li class={card}>
				<div class="flex flex-wrap items-start justify-between gap-2">
					<div class="min-w-0">
						<h2 class="font-semibold break-words">{p.name}</h2>
						<p class="text-xs text-slate-500 dark:text-slate-400">
							{dateTime(p.created_at)}
							{#if p.slicer}· {p.slicer}{/if}
							{#if p.printer_preset}· {p.printer_preset}{/if}
						</p>
					</div>
					{#if confirmDelete === p.id}
						<div class="text-sm">
							Delete and put the filament back?
							<button
								type="button"
								class="ml-2 font-semibold text-red-600"
								disabled={busy === p.id}
								onclick={() => removePrint(p)}
							>
								Delete
							</button>
							<button
								type="button"
								class="ml-2 font-semibold"
								onclick={() => (confirmDelete = null)}
							>
								Keep
							</button>
						</div>
					{:else}
						<button
							type="button"
							class="text-xs font-medium text-slate-500 hover:text-red-600"
							onclick={() => (confirmDelete = p.id)}
						>
							Didn't print it…
						</button>
					{/if}
				</div>

				<ul class="mt-3 divide-y divide-slate-200 dark:divide-slate-800">
					{#each p.slots as s (s.slot)}
						{@const k = key(p, s)}
						<li class="py-2">
							<div class="flex items-center gap-3">
								<ColorSwatch hex={s.color_hex} size="sm" label={s.filament_type ?? 'Filament'} />
								<div class="min-w-0 flex-1">
									<p class="truncate text-sm font-medium">
										{s.profile_name ?? `Filament ${s.slot}`}
									</p>
									<p class="text-xs text-slate-500 dark:text-slate-400">
										Slot {s.slot}{#if s.filament_type}&nbsp;· {s.filament_type}{/if}
									</p>
								</div>
								<span class="text-sm font-semibold tabular-nums">{grams(s.grams)}</span>
							</div>

							{#if s.status === 'logged'}
								<div class="mt-1 flex flex-wrap items-center justify-between gap-2 pl-9 text-sm">
									{#if s.spool && s.spool_id}
										<a
											href={resolve('/spools/[id]', { id: s.spool_id })}
											class="min-w-0 truncate text-emerald-700 hover:underline dark:text-emerald-400"
										>
											✓ Taken from {spoolLabel(s.spool)}
										</a>
									{:else}
										<span class="text-emerald-700 dark:text-emerald-400">✓ Logged</span>
									{/if}
									<button
										type="button"
										class="text-xs font-medium text-slate-500 hover:text-red-600"
										disabled={busy === k}
										onclick={() => act(k, () => api.undoPrintSlot(p.id, s.slot))}
									>
										Undo
									</button>
								</div>
							{:else if s.status === 'skipped'}
								<div class="mt-1 flex items-center justify-between gap-2 pl-9 text-sm">
									<span class="text-slate-500">Skipped</span>
									<button
										type="button"
										class="text-xs font-medium text-slate-500"
										disabled={busy === k}
										onclick={() => act(k, () => api.undoPrintSlot(p.id, s.slot))}
									>
										Undo
									</button>
								</div>
							{:else}
								{@const choices = spoolChoices(s, spools)}
								<div class="mt-2 space-y-2 pl-9">
									{#if choices.length}
										<div class="flex flex-wrap gap-2">
											<select
												class={select}
												aria-label="Spool for slot {s.slot}"
												value={picked[k] ?? choices[0].id}
												onchange={(e) => (picked[k] = e.currentTarget.value)}
											>
												{#each choices as c (c.id)}
													<option value={c.id}>
														{spoolLabel(c)} ({grams(c.remaining_weight_g)} left)
													</option>
												{/each}
											</select>
											<button
												type="button"
												class="rounded-lg bg-orange-600 px-3 py-2 text-sm font-semibold text-white disabled:opacity-50"
												disabled={busy === k}
												onclick={() => log(p, s)}
											>
												Take {grams(s.grams)}
											</button>
											<button
												type="button"
												class="rounded-lg px-3 py-2 text-sm font-semibold ring-1 ring-slate-300 dark:ring-slate-700"
												disabled={busy === k}
												onclick={() =>
													act(k, () => api.assignPrintSlot(p.id, s.slot, { skip: true }))}
											>
												Skip
											</button>
										</div>
										{#if s.profile_name}
											<label
												class="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-300"
											>
												<input
													type="checkbox"
													class="size-4 accent-orange-600"
													checked={remember[k] ?? true}
													onchange={(e) => (remember[k] = e.currentTarget.checked)}
												/>
												Next time, use this spool's filament for “{s.profile_name}” by itself
											</label>
										{/if}
									{:else}
										<p class="text-sm text-slate-500">
											No open or sealed spools to take it from.
											<button
												type="button"
												class={small}
												onclick={() =>
													act(k, () => api.assignPrintSlot(p.id, s.slot, { skip: true }))}
											>
												Skip
											</button>
										</p>
									{/if}
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			</li>
		{/each}
	</ul>
{/if}
