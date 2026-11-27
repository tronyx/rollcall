import { describe, expect, it } from 'vitest';
import type { PrintSlot, Spool } from '../api/types';
import { colorDistance, gcodeSummary, spoolChoices, spoolLabel } from './prints';

describe('gcodeSummary', () => {
	it('keeps the header, the config and the totals', () => {
		const g = [
			'; HEADER_BLOCK_START',
			'; BambuStudio 02.04.00.70',
			'; HEADER_BLOCK_END',
			'G1 X1 Y1',
			';TYPE:Outer wall',
			'; FEATURE: Inner wall',
			'; CONFIG_BLOCK_START\r',
			'; filament_type = PLA',
			'; CONFIG_BLOCK_END',
			'G1 X2',
			'; filament used [g] = 1.2'
		].join('\n');
		expect(gcodeSummary(g)).toBe(
			[
				'; HEADER_BLOCK_START',
				'; BambuStudio 02.04.00.70',
				'; HEADER_BLOCK_END',
				'; CONFIG_BLOCK_START',
				'; filament_type = PLA',
				'; CONFIG_BLOCK_END',
				'; filament used [g] = 1.2',
				''
			].join('\n')
		);
	});
});

const spool = (id: string, o: Partial<Spool>) =>
	({
		id,
		filament_id: `f-${id}`,
		status: 'active',
		material_code: 'PLA',
		color_hex: '#000000',
		brand_name: 'B',
		product_name: null,
		color_name: id,
		label_code: null,
		...o
	}) as Spool;

const slot = (o: Partial<PrintSlot>) =>
	({
		slot: 1,
		grams: 1,
		profile_name: null,
		filament_type: 'PETG',
		color_hex: '#FF0000',
		status: 'pending',
		spool_id: null,
		weight_event_id: null,
		profile_filament_id: null,
		spool: null,
		...o
	}) as PrintSlot;

describe('spoolChoices', () => {
	const spools = [
		spool('pla-red', { color_hex: '#FF0000' }),
		spool('petg-black', { material_code: 'PETG' }),
		spool('petg-red', { material_code: 'PETG', color_hex: '#EE1111' }),
		spool('petg-red-sealed', { material_code: 'PETG', color_hex: '#FF0000', status: 'sealed' }),
		spool('empty', { material_code: 'PETG', color_hex: '#FF0000', status: 'empty' })
	];
	it('ranks material, then open over sealed, then color', () => {
		expect(spoolChoices(slot({}), spools).map((s) => s.id)).toEqual([
			'petg-red',
			'petg-black',
			'petg-red-sealed',
			'pla-red'
		]);
	});
	it('puts the known filament first', () => {
		expect(spoolChoices(slot({ profile_filament_id: 'f-pla-red' }), spools)[0].id).toBe('pla-red');
	});
});

describe('labels', () => {
	it('formats', () => {
		expect(
			spoolLabel({
				brand_name: 'Bambu Lab',
				material_code: 'PLA',
				product_name: 'PLA Basic',
				color_name: 'Jade White',
				label_code: 'R-0001'
			})
		).toBe('Bambu Lab PLA Basic · Jade White · #R-0001');
		expect(colorDistance('#000000', '#000000')).toBe(0);
		expect(colorDistance(null, '#000000')).toBe(500);
	});
});
