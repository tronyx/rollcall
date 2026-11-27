import { describe, expect, it } from 'vitest';
import type { Filament, TunedValues } from '../api/types';
import { addableTargets, emptyValues, formatValue, setFields, TUNED_FIELDS } from './tuned';

const field = (key: string) => TUNED_FIELDS.find((f) => f.key === key)!;

describe('tuned settings', () => {
	it('formats values with their units', () => {
		expect(formatValue(field('nozzle_temp_c'), 215)).toBe('215°C');
		expect(formatValue(field('fan_speed_pct'), 40)).toBe('40%');
		expect(formatValue(field('print_speed_mm_s'), 200)).toBe('200 mm/s');
		expect(formatValue(field('flow_ratio'), 0.98)).toBe('0.98');
	});

	it('lists only the fields a row sets', () => {
		const t: TunedValues = { ...emptyValues(), flow_ratio: 0.98, nozzle_temp_c: 215 };
		expect(setFields(t).map((f) => f.key)).toEqual(['nozzle_temp_c', 'flow_ratio']);
		expect(setFields(emptyValues())).toEqual([]);
	});

	it('shows the manufacturer value each field overrides', () => {
		const f = {
			nozzle_temp_min_c: 190,
			nozzle_temp_max_c: 230,
			bed_temp_min_c: null,
			bed_temp_max_c: null,
			drying_temp_c: 55,
			drying_time_min: null
		} as Filament;
		expect(field('nozzle_temp_c').fallback!(f)).toBe('190–230°C');
		expect(field('bed_temp_c').fallback!(f)).toBeNull();
		expect(field('drying_temp_c').fallback!(f)).toBe('55°C');
		expect(field('flow_ratio').fallback).toBeUndefined();
	});

	it('offers printers that have no settings yet', () => {
		const printers = [{ id: 'a' }, { id: 'b' }];
		expect(addableTargets(printers, [])).toEqual({ anyPrinter: true, printers });
		expect(addableTargets(printers, [{ printer_id: null }, { printer_id: 'a' }])).toEqual({
			anyPrinter: false,
			printers: [{ id: 'b' }]
		});
	});
});
