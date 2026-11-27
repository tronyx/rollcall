<script lang="ts">
	let {
		hex,
		secondary = null,
		size = 'md',
		label
	}: {
		hex: string | null;
		secondary?: string | null;
		size?: 'sm' | 'md' | 'lg';
		label: string;
	} = $props();

	const sizes = { sm: 'size-6', md: 'size-12', lg: 'size-16' };
	// Two-tone filaments get a diagonal split; unknown colors get a checker pattern.
	const fill = $derived(
		!hex
			? 'repeating-conic-gradient(#cbd5e1 0 25%, #f1f5f9 0 50%) 50% / 10px 10px'
			: secondary
				? `linear-gradient(135deg, ${hex} 50%, ${secondary} 50%)`
				: hex
	);
</script>

<!-- A spool seen from the side: colored ring around a hub. -->
<span
	class="relative inline-flex shrink-0 items-center justify-center rounded-full shadow-inner ring-1 ring-black/10 dark:ring-white/15 {sizes[
		size
	]}"
	style:background={fill}
	role="img"
	aria-label={label}
	title={label}
>
	{#if size !== 'sm'}
		<span class="size-1/3 rounded-full bg-slate-50 ring-1 ring-black/10 dark:bg-slate-900"></span>
	{/if}
</span>
