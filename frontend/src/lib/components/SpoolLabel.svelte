<script lang="ts">
	import type { Filament, Spool } from '#lib/api/types.ts';
	import { productLine, qrPath, tempLine, type LabelSize } from '#lib/util/label.ts';

	let {
		spool,
		filament = null,
		link,
		size
	}: {
		spool: Spool;
		filament?: Filament | null;
		link: string;
		size: LabelSize;
	} = $props();

	const qr = $derived(qrPath(link));
	const temps = $derived(tempLine(filament));
	// Text scales with the label's height so every size keeps the same look.
	const unit = $derived(size.height_mm / 30);
	// The QR code fills the height, but leaves narrow labels room for the text.
	const qrMm = $derived(Math.min(size.height_mm - 3 * unit, size.width_mm * 0.45));
	const swatch = $derived(
		!spool.color_hex
			? 'repeating-conic-gradient(#cbd5e1 0 25%, #fff 0 50%) 50% / 2mm 2mm'
			: spool.color_hex_secondary
				? `linear-gradient(135deg, ${spool.color_hex} 50%, ${spool.color_hex_secondary} 50%)`
				: spool.color_hex
	);
</script>

<!-- Printed in black on white whatever the screen's theme, at the label's real size. -->
<article
	class="spool-label flex overflow-hidden bg-white text-black"
	style:width="{size.width_mm}mm"
	style:height="{size.height_mm}mm"
	style:padding="{1.5 * unit}mm"
	style:gap="{1.5 * unit}mm"
	style:font-size="{2.6 * unit}mm"
	aria-label="Label for {spool.color_name} {spool.material_code}"
>
	<svg
		viewBox="0 0 {qr.size} {qr.size}"
		class="shrink-0 self-center"
		style:width="{qrMm}mm"
		style:height="{qrMm}mm"
		shape-rendering="crispEdges"
		role="img"
		aria-label="QR code linking to this spool"
	>
		<rect width={qr.size} height={qr.size} fill="#fff" />
		<path d={qr.d} fill="#000" />
	</svg>
	<div class="flex min-w-0 flex-1 flex-col justify-between leading-tight">
		<div class="min-w-0">
			<p class="line-clamp-2 font-bold break-words" style:font-size="{3.4 * unit}mm">
				{spool.color_name}
			</p>
			<p class="truncate">{spool.brand_name}</p>
			<p class="truncate font-semibold">
				{productLine(spool)}
			</p>
			{#if temps}<p class="truncate">{temps}</p>{/if}
		</div>
		<div class="flex items-center justify-between gap-1">
			<span
				class="shrink-0 rounded-full border border-black"
				style:width="{3.2 * unit}mm"
				style:height="{3.2 * unit}mm"
				style:background={swatch}
				style:print-color-adjust="exact"
			></span>
			<span class="truncate font-mono font-bold">
				{spool.label_code ? `#${spool.label_code}` : `${spool.diameter_mm} mm`}
			</span>
		</div>
	</div>
</article>
