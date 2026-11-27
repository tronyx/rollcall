// Helpers for the printable spool labels on /labels.

import { encode } from 'uqr';
import type { Filament, Spool } from '../api/types.ts';
import { range } from './format.ts';

export interface LabelSize {
	id: string;
	name: string;
	width_mm: number;
	height_mm: number;
}

// Common sizes for thermal label printers. All are landscape: the QR code on
// the left, the text on the right.
export const LABEL_SIZES: LabelSize[] = [
	{ id: '40x30', name: '40 × 30 mm', width_mm: 40, height_mm: 30 },
	{ id: '50x30', name: '50 × 30 mm', width_mm: 50, height_mm: 30 },
	{ id: '62x29', name: '62 × 29 mm (Brother DK)', width_mm: 62, height_mm: 29 },
	{ id: '70x40', name: '70 × 40 mm', width_mm: 70, height_mm: 40 }
];

export type LabelLayout = 'printer' | 'sheet';

export function labelSize(id: string | null | undefined): LabelSize {
	return LABEL_SIZES.find((s) => s.id === id) ?? LABEL_SIZES[0];
}

export interface QrPath {
	/** Modules per side, including the quiet zone. */
	size: number;
	/** SVG path data for the dark modules, one unit per module. */
	d: string;
}

/** Encodes text as a QR code and returns one SVG path for its dark modules. */
export function qrPath(text: string, quietZone = 2): QrPath {
	const qr = encode(text, { ecc: 'M', border: 0 });
	let d = '';
	qr.data.forEach((row, y) => {
		// Join runs of dark modules on a row into one rectangle each.
		let x = 0;
		while (x < row.length) {
			if (!row[x]) {
				x++;
				continue;
			}
			const start = x;
			while (x < row.length && row[x]) x++;
			d += `M${start + quietZone} ${y + quietZone}h${x - start}v1h${start - x}z`;
		}
	});
	return { size: qr.size + quietZone * 2, d };
}

/** The address a label's QR code opens: the spool's page under `base`. */
export function spoolLink(base: string, path: string): string {
	return base.trim().replace(/\/+$/, '') + path;
}

/** Whether `base` is an http(s) address the QR codes can point at. */
export function validBase(base: string): boolean {
	try {
		const url = new URL(base.trim());
		return url.protocol === 'http:' || url.protocol === 'https:';
	} catch {
		return false;
	}
}

/** The short line of print temperatures, such as "210–230°C · bed 60°C". */
export function tempLine(filament: Filament | null | undefined): string | null {
	if (!filament) return null;
	const nozzle = range(filament.nozzle_temp_min_c, filament.nozzle_temp_max_c, '°C');
	const bed = range(filament.bed_temp_min_c, filament.bed_temp_max_c, '°C');
	const parts = [nozzle, bed && `bed ${bed}`].filter(Boolean);
	return parts.length ? parts.join(' · ') : null;
}

/** The material and product name, without repeating the material ("PLA Basic", not "PLA PLA Basic"). */
export function productLine(spool: Pick<Spool, 'material_code' | 'product_name'>): string {
	const product = spool.product_name?.trim();
	if (!product) return spool.material_code;
	const words = product.toLowerCase().split(/[\s-]+/);
	return words.includes(spool.material_code.toLowerCase())
		? product
		: `${spool.material_code} ${product}`;
}

/** Spools worth labeling by default: everything that isn't empty or archived. */
export function labelable(spools: Spool[]): Spool[] {
	return spools.filter((s) => s.status !== 'empty' && s.status !== 'archived');
}
