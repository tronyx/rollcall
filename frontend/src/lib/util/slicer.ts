// Tuned settings as an OrcaSlicer or Bambu Studio filament preset, for
// File > Import > Import Configs. The keys and formats follow the slicers'
// own preset files (resources/profiles in each repository).

import type { Filament, Printer, TunedValues } from '../api/types.ts';

export type Slicer = 'orca' | 'bambu';

export const SLICERS: { id: Slicer; name: string }[] = [
	{ id: 'orca', name: 'OrcaSlicer' },
	{ id: 'bambu', name: 'Bambu Studio' }
];

interface MaterialInfo {
	/** The slicer's filament_type. */
	type: string;
	/** OrcaSlicer's generic preset for it. */
	orca: string;
	/** Bambu Studio's generic preset name, without the "@BBL <printer>" part. */
	bambu: string;
	/** Bambu printers that have their own generic preset, as in "Generic PLA @BBL A1". */
	bambuPrinters: string[];
	/** Bambu Studio also has the plain name (for the X1, X1E and P1S). */
	bambuPlain: boolean;
	density: number;
}

const MOST = ['A1', 'A1M', 'A2L', 'H2C', 'H2D', 'H2DP', 'H2S', 'H3C', 'P1P', 'P2S', 'P3C', 'X2D'];
const ENG = ['A1', 'H2C', 'H2D', 'H2DP', 'P1P', 'P2S', 'X1E', 'X2D'];

// Rollcall's material codes (backend/migrations/0001_initial_schema.sql).
const MATERIALS: Record<string, MaterialInfo> = {
	PLA: {
		type: 'PLA',
		orca: 'Generic PLA @System',
		bambu: 'Generic PLA',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.24
	},
	'PLA+': {
		type: 'PLA',
		orca: 'Generic PLA @System',
		bambu: 'Generic PLA',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.23
	},
	'PLA-SILK': {
		type: 'PLA',
		orca: 'Generic PLA Silk @System',
		bambu: 'Generic PLA Silk',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.24
	},
	'PLA-MATTE': {
		type: 'PLA',
		orca: 'Generic PLA Matte @System',
		bambu: 'Generic PLA',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.3
	},
	'PLA-CF': {
		type: 'PLA-CF',
		orca: 'Generic PLA-CF @System',
		bambu: 'Generic PLA-CF',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.3
	},
	PETG: {
		type: 'PETG',
		orca: 'Generic PETG @System',
		bambu: 'Generic PETG',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.27
	},
	'PETG-CF': {
		type: 'PETG-CF',
		orca: 'Generic PETG-CF @System',
		bambu: 'Generic PETG-CF',
		bambuPrinters: [...MOST, 'X1C'],
		bambuPlain: false,
		density: 1.3
	},
	ABS: {
		type: 'ABS',
		orca: 'Generic ABS @System',
		bambu: 'Generic ABS',
		bambuPrinters: ENG,
		bambuPlain: true,
		density: 1.04
	},
	ASA: {
		type: 'ASA',
		orca: 'Generic ASA @System',
		bambu: 'Generic ASA',
		bambuPrinters: ENG,
		bambuPlain: true,
		density: 1.07
	},
	TPU: {
		type: 'TPU',
		orca: 'Generic TPU @System',
		bambu: 'Generic TPU',
		bambuPrinters: MOST,
		bambuPlain: true,
		density: 1.21
	},
	PA: {
		type: 'PA',
		orca: 'Generic PA @System',
		bambu: 'Generic PA',
		bambuPrinters: ['A1', 'H2C', 'H2D', 'H2DP', 'P1P', 'P2S', 'X2D'],
		bambuPlain: true,
		density: 1.14
	},
	'PA-CF': {
		type: 'PA-CF',
		orca: 'Generic PA-CF @System',
		bambu: 'Generic PA-CF',
		bambuPrinters: ENG,
		bambuPlain: true,
		density: 1.2
	},
	PC: {
		type: 'PC',
		orca: 'Generic PC @System',
		bambu: 'Generic PC',
		bambuPrinters: [...MOST.filter((p) => p !== 'A1M' && p !== 'A2L'), 'P1S', 'X1E'],
		bambuPlain: true,
		density: 1.2
	},
	HIPS: {
		type: 'HIPS',
		orca: 'Generic HIPS @System',
		bambu: 'Generic HIPS',
		bambuPrinters: ['A1', 'A1M', 'A2L', 'H2C', 'H2D', 'H2DP', 'P2S', 'X1C', 'X2D'],
		bambuPlain: false,
		density: 1.04
	},
	PVA: {
		type: 'PVA',
		orca: 'Generic PVA @System',
		bambu: 'Generic PVA',
		bambuPrinters: ['A1', 'A1M', 'A2L', 'H2C', 'H2D', 'H2DP', 'P1P', 'P2S', 'X2D'],
		bambuPlain: true,
		density: 1.23
	}
};

/** The Bambu Studio printer code in "@BBL <code>" for a printer's name or model. */
export function bambuPrinterCode(text: string): string | null {
	const t = text.toLowerCase().replace(/[\s_-]+/g, ' ');
	const rules: [RegExp, string][] = [
		[/\ba1 ?mini\b|\ba1m\b/, 'A1M'],
		[/\ba1\b/, 'A1'],
		[/\ba2l\b/, 'A2L'],
		[/\bh2d ?pro\b|\bh2dp\b/, 'H2DP'],
		[/\bh2d\b/, 'H2D'],
		[/\bh2s\b/, 'H2S'],
		[/\bh2c\b/, 'H2C'],
		[/\bh3c\b/, 'H3C'],
		[/\bp1p\b/, 'P1P'],
		[/\bp1s\b/, 'P1S'],
		[/\bp2s\b/, 'P2S'],
		[/\bp3c\b/, 'P3C'],
		[/\bx1e\b/, 'X1E'],
		[/\bx1 ?c(arbon)?\b|\bx1\b/, 'X1C'],
		[/\bx2d\b/, 'X2D']
	];
	return rules.find(([re]) => re.test(t))?.[1] ?? null;
}

/** The slicer's generic preset to build on, or '' when there's no good match. */
export function suggestBase(slicer: Slicer, materialCode: string, printer: Printer | null): string {
	const m = MATERIALS[materialCode.toUpperCase()];
	if (!m) return '';
	if (slicer === 'orca') return m.orca;
	const code = printer ? bambuPrinterCode(`${printer.model ?? ''} ${printer.name}`) : null;
	if (code && m.bambuPrinters.includes(code)) return `${m.bambu} @BBL ${code}`;
	// The plain generic presets are for the X1, X1E and P1S. For "any printer",
	// they're the closest thing to a default.
	if (m.bambuPlain && (!printer || (code && ['X1C', 'X1E', 'P1S'].includes(code)))) return m.bambu;
	return '';
}

/** Every generic preset we know of for this slicer and material, for the base picker. */
export function baseOptions(slicer: Slicer, materialCode: string): string[] {
	const m = MATERIALS[materialCode.toUpperCase()];
	if (!m) return [];
	if (slicer === 'orca') return [m.orca];
	return [
		...(m.bambuPlain ? [m.bambu] : []),
		...m.bambuPrinters.map((p) => `${m.bambu} @BBL ${p}`)
	];
}

/** "Bambu Lab PLA Basic Jade White @X1C (Rollcall)". */
export function profileName(filament: Filament, printer: Printer | null): string {
	const product = filament.product_name ?? filament.material_code;
	const parts = [filament.brand_name, product, filament.color_name].filter(Boolean).join(' ');
	return `${parts}${printer ? ` @${printer.name}` : ''} (Rollcall)`;
}

/** A name that's safe as a file name on every system. */
export function profileFileName(name: string): string {
	const safe = name
		.replace(/[\\/:*?"<>|]+/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
	return `${safe || 'filament'}.json`;
}

// The slicers keep some filament values per extruder type ("variant"). OrcaSlicer's
// generic presets list six, and a preset must give a value for each.
const ORCA_VARIANTS = [
	'Direct Drive Standard',
	'Direct Drive High Flow',
	'Direct Drive Extra High Flow',
	'Bowden Standard',
	'Bowden High Flow',
	'Bowden Extra High Flow'
];

const mid = (a: number | null, b: number | null): number | null =>
	a != null && b != null ? Math.round((a + b) / 2) : (a ?? b);

const str = (v: number) => String(Number(v.toFixed(4)));

export interface ProfileInput {
	slicer: Slicer;
	name: string;
	/** The preset it inherits from, or '' for a standalone preset. */
	base: string;
	filament: Filament;
	/** The tuned values; empty ones fall back to the filament's. */
	tuned: TunedValues;
	notes?: string | null;
}

/** What the preset sets, for showing before the download. */
export interface ProfileValue {
	label: string;
	value: string;
	/** Where it came from. */
	from: 'tuned' | 'maker';
}

/** The preset's values: tuned first, then the filament's maker values. */
export function profileValues(
	filament: Filament,
	tuned: TunedValues
): Record<string, ProfileValue> {
	const out: Record<string, ProfileValue> = {};
	const put = (key: string, label: string, t: number | null, m: number | null, unit = '') => {
		const v = t ?? m;
		if (v != null)
			out[key] = { label, value: `${str(v)}${unit}`, from: t != null ? 'tuned' : 'maker' };
	};
	const nozzle = mid(filament.nozzle_temp_min_c, filament.nozzle_temp_max_c);
	put('nozzle', 'Nozzle', tuned.nozzle_temp_c, nozzle, '°C');
	put(
		'first_layer',
		'First layer nozzle',
		tuned.first_layer_nozzle_temp_c ?? tuned.nozzle_temp_c,
		nozzle,
		'°C'
	);
	put(
		'bed',
		'Bed (textured and smooth PEI)',
		tuned.bed_temp_c,
		mid(filament.bed_temp_min_c, filament.bed_temp_max_c),
		'°C'
	);
	put('volumetric', 'Max volumetric speed', tuned.max_volumetric_speed_mm3_s, null, ' mm³/s');
	put('flow', 'Flow ratio', tuned.flow_ratio, null);
	put('pa', 'Pressure advance', tuned.pressure_advance, null);
	put('retract_len', 'Retraction length', tuned.retraction_length_mm, null, ' mm');
	put('retract_speed', 'Retraction speed', tuned.retraction_speed_mm_s, null, ' mm/s');
	put('fan', 'Fan', tuned.fan_speed_pct, null, '%');
	return out;
}

/** The preset as the slicer's JSON. Every value is a list of strings, as in the slicer's own files. */
export function buildProfile(input: ProfileInput): Record<string, unknown> {
	const { slicer, filament: f, tuned: t } = input;
	const m = MATERIALS[f.material_code.toUpperCase()];
	const p: Record<string, unknown> = {
		type: 'filament',
		name: input.name,
		inherits: input.base,
		from: 'User',
		instantiation: 'true',
		version: '1.0.0.0',
		filament_settings_id: [input.name],
		filament_vendor: [f.brand_name]
	};
	if (m) p.filament_type = [m.type];
	if (f.color_hex) p.filament_colour = [f.color_hex.toUpperCase()];
	p.filament_diameter = [str(f.diameter_mm)];
	const density = f.density_g_cm3 ?? m?.density;
	if (density) p.filament_density = [str(density)];

	const one = (v: number | null) => (v == null ? undefined : [str(v)]);
	// Values the slicer keeps per extruder type.
	let variants = false;
	const perVariant = (v: number | null) => {
		if (v == null) return undefined;
		if (slicer === 'bambu') return [str(v)];
		variants = true;
		return ORCA_VARIANTS.map(() => str(v));
	};

	const nozzle = t.nozzle_temp_c ?? mid(f.nozzle_temp_min_c, f.nozzle_temp_max_c);
	const first = t.first_layer_nozzle_temp_c ?? nozzle;
	const bed = t.bed_temp_c ?? mid(f.bed_temp_min_c, f.bed_temp_max_c);
	const values: Record<string, string[] | undefined> = {
		nozzle_temperature: one(nozzle),
		nozzle_temperature_initial_layer: one(first),
		nozzle_temperature_range_low: one(f.nozzle_temp_min_c),
		nozzle_temperature_range_high: one(f.nozzle_temp_max_c),
		hot_plate_temp: one(bed),
		hot_plate_temp_initial_layer: one(bed),
		textured_plate_temp: one(bed),
		textured_plate_temp_initial_layer: one(bed),
		filament_max_volumetric_speed: perVariant(t.max_volumetric_speed_mm3_s),
		filament_flow_ratio: perVariant(t.flow_ratio),
		pressure_advance: one(t.pressure_advance),
		enable_pressure_advance: t.pressure_advance != null ? ['1'] : undefined,
		filament_retraction_length: perVariant(t.retraction_length_mm),
		filament_retraction_speed: perVariant(t.retraction_speed_mm_s),
		fan_min_speed: one(t.fan_speed_pct),
		fan_max_speed: one(t.fan_speed_pct)
	};
	for (const [k, v] of Object.entries(values)) if (v) p[k] = v;
	if (variants) p.filament_extruder_variant = ORCA_VARIANTS;

	const notes = input.notes?.trim();
	if (notes) p.filament_notes = slicer === 'orca' ? [notes] : notes;
	return p;
}

/** Saves a preset as a .json download. */
export function downloadProfile(profile: Record<string, unknown>, name: string) {
	const blob = new Blob([JSON.stringify(profile, null, 4) + '\n'], { type: 'application/json' });
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = profileFileName(name);
	document.body.append(a);
	a.click();
	a.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}
