// The fields of the tuned settings form, and the manufacturer values each
// one falls back to.

import type { Filament, TunedSettings, TunedValues } from '../api/types.ts';
import { range } from './format.ts';

export type TunedKey = keyof TunedValues;

export interface TunedField {
	key: TunedKey;
	label: string;
	unit: string;
	step: number;
	min: number;
	max?: number;
	/** The filament's value this one overrides, as text, if it has one. */
	fallback?: (f: Filament) => string | null;
}

export const TUNED_FIELDS: TunedField[] = [
	{
		key: 'nozzle_temp_c',
		label: 'Nozzle',
		unit: '°C',
		step: 1,
		min: 150,
		max: 500,
		fallback: (f) => range(f.nozzle_temp_min_c, f.nozzle_temp_max_c, '°C')
	},
	{
		key: 'first_layer_nozzle_temp_c',
		label: 'First layer nozzle',
		unit: '°C',
		step: 1,
		min: 150,
		max: 500
	},
	{
		key: 'bed_temp_c',
		label: 'Bed',
		unit: '°C',
		step: 1,
		min: 0,
		max: 150,
		fallback: (f) => range(f.bed_temp_min_c, f.bed_temp_max_c, '°C')
	},
	{
		key: 'print_speed_mm_s',
		label: 'Speed',
		unit: 'mm/s',
		step: 1,
		min: 1,
		fallback: (f) => range(f.print_speed_min_mm_s, f.print_speed_max_mm_s, ' mm/s')
	},
	{
		key: 'max_volumetric_speed_mm3_s',
		label: 'Max volumetric speed',
		unit: 'mm³/s',
		step: 0.1,
		min: 0.1
	},
	{ key: 'flow_ratio', label: 'Flow ratio', unit: '', step: 0.001, min: 0.5, max: 1.5 },
	{ key: 'pressure_advance', label: 'Pressure advance', unit: '', step: 0.0001, min: 0, max: 2 },
	{ key: 'retraction_length_mm', label: 'Retraction length', unit: 'mm', step: 0.01, min: 0 },
	{ key: 'retraction_speed_mm_s', label: 'Retraction speed', unit: 'mm/s', step: 1, min: 1 },
	{ key: 'fan_speed_pct', label: 'Fan', unit: '%', step: 1, min: 0, max: 100 },
	{
		key: 'drying_temp_c',
		label: 'Drying temperature',
		unit: '°C',
		step: 1,
		min: 30,
		max: 150,
		fallback: (f) => (f.drying_temp_c != null ? `${f.drying_temp_c}°C` : null)
	},
	{
		key: 'drying_time_min',
		label: 'Drying time',
		unit: 'min',
		step: 1,
		min: 1,
		fallback: (f) => (f.drying_time_min != null ? `${f.drying_time_min} min` : null)
	}
];

export function emptyValues(): TunedValues {
	return Object.fromEntries(TUNED_FIELDS.map((f) => [f.key, null])) as unknown as TunedValues;
}

/** A value with its unit, such as "215°C", "0.98" or "0.025". */
export function formatValue(field: TunedField, value: number): string {
	if (!field.unit) return String(value);
	return field.unit === '°C' || field.unit === '%'
		? `${value}${field.unit}`
		: `${value} ${field.unit}`;
}

/** The fields a row actually sets, in form order. */
export function setFields(t: TunedValues): TunedField[] {
	return TUNED_FIELDS.filter((f) => t[f.key] != null);
}

/** Printers that don't have settings yet, as options for "Add". */
export function addableTargets<P extends { id: string }>(
	printers: P[],
	rows: Pick<TunedSettings, 'printer_id'>[]
): { anyPrinter: boolean; printers: P[] } {
	const used = new Set(rows.map((r) => r.printer_id));
	return { anyPrinter: !used.has(null), printers: printers.filter((p) => !used.has(p.id)) };
}
