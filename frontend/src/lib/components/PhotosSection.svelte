<script lang="ts">
	import { api, ApiError } from '#lib/api/client.ts';
	import type { Photo, PhotoOwner } from '#lib/api/types.ts';
	import { dayOf } from '#lib/util/format.ts';
	import { fileSize, photoProblem } from '#lib/util/photos.ts';

	let {
		owner,
		title = 'Photos',
		hint = '',
		card = '',
		readonly = false,
		onchange
	}: {
		owner: PhotoOwner;
		title?: string;
		/** Shown under the title, e.g. who else sees these photos. */
		hint?: string;
		/** Card classes; leave empty to render without a card (inside a form). */
		card?: string;
		/** Show the photos without upload, caption or delete. */
		readonly?: boolean;
		/** Called with the photos after each upload or delete. */
		onchange?: (photos: Photo[]) => void;
	} = $props();

	let photos = $state<Photo[]>([]);
	let loaded = $state(false);
	let error = $state<string | null>(null);
	let progress = $state<string | null>(null);
	let fileInput = $state<HTMLInputElement>();

	// The photo open in the viewer, by index.
	let open = $state<number | null>(null);
	let dialog = $state<HTMLDialogElement>();
	let caption = $state('');
	let confirmDelete = $state(false);
	let busy = $state(false);

	const key = $derived(JSON.stringify(owner));

	async function load(o: PhotoOwner) {
		loaded = false;
		error = null;
		try {
			photos = await api.photos(o);
			loaded = true;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load photos.';
		}
	}

	$effect(() => {
		void key;
		load(owner);
	});

	function changed() {
		onchange?.(photos);
	}

	async function upload(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const files = [...(input.files ?? [])];
		input.value = '';
		error = null;
		const problems: string[] = [];
		const ok = files.filter((f) => {
			const p = photoProblem(f);
			if (p) problems.push(p);
			return !p;
		});
		for (const [i, f] of ok.entries()) {
			progress = ok.length > 1 ? `Uploading ${i + 1} of ${ok.length}…` : 'Uploading…';
			try {
				photos = [...photos, await api.uploadPhoto(owner, f)];
			} catch (err) {
				problems.push(`${f.name}: ${err instanceof ApiError ? err.message : 'upload failed'}.`);
			}
		}
		progress = null;
		if (problems.length) error = problems.join(' ');
		if (ok.length) changed();
	}

	function show(i: number) {
		open = i;
		caption = photos[i].caption ?? '';
		confirmDelete = false;
		dialog?.showModal();
	}

	function step(by: number) {
		if (open === null || photos.length < 2) return;
		show((open + by + photos.length) % photos.length);
	}

	function keydown(e: KeyboardEvent) {
		if (e.target instanceof HTMLInputElement) return;
		if (e.key === 'ArrowLeft') step(-1);
		if (e.key === 'ArrowRight') step(1);
	}

	const current = $derived(open === null ? null : (photos[open] ?? null));

	// Not a <form>: this can sit inside another form, and forms can't nest.
	async function saveCaption() {
		if (!current) return;
		busy = true;
		try {
			const p = await api.setPhotoCaption(current.id, caption.trim() || null);
			photos = photos.map((x) => (x.id === p.id ? p : x));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the caption.';
		} finally {
			busy = false;
		}
	}

	async function remove() {
		if (!current) return;
		busy = true;
		try {
			await api.deletePhoto(current.id);
			photos = photos.filter((x) => x.id !== current.id);
			dialog?.close();
			changed();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not delete the photo.';
		} finally {
			busy = false;
		}
	}

	const link = 'text-sm font-semibold text-orange-700 dark:text-orange-400';
</script>

<section class={card}>
	<div class="mb-2 flex items-center justify-between gap-2">
		<div>
			<h2 class={card ? 'font-semibold' : 'text-sm font-semibold'}>{title}</h2>
			{#if hint}<p class="text-xs text-slate-500 dark:text-slate-400">{hint}</p>{/if}
		</div>
		{#if loaded && !readonly}
			<button
				type="button"
				class={link}
				disabled={progress !== null}
				onclick={() => fileInput?.click()}
			>
				{progress ?? '+ Add photos'}
			</button>
			<input
				bind:this={fileInput}
				type="file"
				accept="image/jpeg,image/png,image/webp,image/gif,image/avif"
				multiple
				class="hidden"
				onchange={upload}
				aria-label="Add photos"
			/>
		{/if}
	</div>

	{#if error}<p class="mb-2 text-sm text-red-600" role="alert">{error}</p>{/if}

	{#if !loaded && !error}
		<p class="text-sm text-slate-500">Loading…</p>
	{:else if loaded && photos.length === 0}
		<p class="text-sm text-slate-600 dark:text-slate-400">
			No photos yet. JPEG, PNG, WebP, GIF or AVIF, up to 20 MB each.
		</p>
	{:else}
		<ul class="grid grid-cols-3 gap-2 sm:grid-cols-4">
			{#each photos as p, i (p.id)}
				<li>
					<button
						type="button"
						class="block aspect-square w-full overflow-hidden rounded-lg bg-slate-100 ring-1 ring-slate-200 hover:ring-2 hover:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700"
						onclick={() => show(i)}
						aria-label={p.caption ?? `Photo ${i + 1}`}
					>
						<img src={p.url} alt={p.caption ?? ''} loading="lazy" class="size-full object-cover" />
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<dialog
	bind:this={dialog}
	onclose={() => (open = null)}
	onkeydown={keydown}
	class="m-auto w-[min(56rem,calc(100vw-2rem))] rounded-2xl bg-white p-0 text-slate-900 shadow-xl backdrop:bg-black/70 dark:bg-slate-900 dark:text-slate-100"
>
	{#if current}
		<div class="relative bg-black">
			<img
				src={current.url}
				alt={current.caption ?? ''}
				class="mx-auto max-h-[70vh] w-auto object-contain"
			/>
			{#if photos.length > 1}
				<button
					type="button"
					class="absolute top-1/2 left-2 -translate-y-1/2 rounded-full bg-black/50 px-3 py-2 text-white"
					onclick={() => step(-1)}
					aria-label="Previous photo"
				>
					‹
				</button>
				<button
					type="button"
					class="absolute top-1/2 right-2 -translate-y-1/2 rounded-full bg-black/50 px-3 py-2 text-white"
					onclick={() => step(1)}
					aria-label="Next photo"
				>
					›
				</button>
			{/if}
		</div>
		<div class="space-y-3 p-4">
			<div class="flex gap-2">
				<input
					type="text"
					bind:value={caption}
					placeholder="Add a caption"
					aria-label="Caption"
					maxlength="500"
					onkeydown={(e) => {
						if (e.key === 'Enter') {
							e.preventDefault();
							saveCaption();
						}
					}}
					class="min-w-0 flex-1 rounded-lg border-0 bg-white px-3 py-2 text-sm shadow-sm ring-1 ring-slate-300 focus:ring-2 focus:ring-orange-500 dark:bg-slate-800 dark:ring-slate-700"
				/>
				<button
					type="button"
					onclick={saveCaption}
					disabled={busy || caption.trim() === (current.caption ?? '')}
					class="rounded-lg bg-orange-600 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
				>
					Save
				</button>
			</div>
			<div class="flex flex-wrap items-center justify-between gap-2 text-sm">
				<p class="text-xs text-slate-500 dark:text-slate-400">
					{#if photos.length > 1}{(open ?? 0) + 1} of {photos.length} ·{/if}
					Added {dayOf(current.created_at)} · {fileSize(current.size_bytes)}
					{#if current.original_name}· {current.original_name}{/if}
				</p>
				<div class="flex items-center gap-3">
					{#if confirmDelete}
						<span>Delete this photo?</span>
						<button
							type="button"
							class="font-semibold text-red-600"
							disabled={busy}
							onclick={remove}
						>
							Delete
						</button>
						<button type="button" class="font-semibold" onclick={() => (confirmDelete = false)}>
							Keep
						</button>
					{:else}
						<a href={current.url} target="_blank" rel="noopener" class={link}>Full size</a>
						{#if !readonly}
							<button
								type="button"
								class="font-medium text-slate-500 hover:text-red-600"
								onclick={() => (confirmDelete = true)}
							>
								Delete…
							</button>
						{/if}
					{/if}
					<button type="button" class={link} onclick={() => dialog?.close()}>Close</button>
				</div>
			</div>
		</div>
	{/if}
</dialog>
