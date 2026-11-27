import { describe, expect, it } from 'vitest';
import type { Equipment } from '../api/types';
import { filterEquipment, KIND_SUGGESTIONS, makerLine, usedIds } from './equipment';

describe('equipment', () => {
	it('suggests kinds for each category', () => {
		expect(KIND_SUGGESTIONS.equipment).toContain('Nozzle');
		expect(KIND_SUGGESTIONS.accessory).toContain('Build plate');
	});

	it('joins brand and model', () => {
		expect(makerLine({ brand_name: 'Bambu Lab', model: 'H2D-NZ-04' })).toBe(
			'Bambu Lab · H2D-NZ-04'
		);
		expect(makerLine({ brand_name: 'Prusa', model: null })).toBe('Prusa');
		expect(makerLine({ brand_name: null, model: null })).toBeNull();
	});

	it('collects the brands and printers in use', () => {
		const ids = usedIds([
			{ brand_id: 'b1', printer_id: null },
			{ brand_id: null, printer_id: 'p1' },
			{ brand_id: 'b1', printer_id: 'p2' }
		]);
		expect([...ids.brands]).toEqual(['b1']);
		expect([...ids.printers].sort()).toEqual(['p1', 'p2']);
	});

	it('filters like the API', () => {
		const item = (o: Partial<Equipment>) =>
			({
				category: 'equipment',
				brand_id: null,
				printer_id: null,
				retired: false,
				kind: null,
				model: null,
				brand_name: null,
				notes: null,
				name: '',
				...o
			}) as Equipment;
		const items = [
			item({ name: 'Nozzle', brand_id: 'b1', printer_id: 'p1' }),
			item({ name: 'PEI sheet', category: 'accessory', printer_id: 'p2' }),
			item({ name: 'Cutters', category: 'accessory', notes: 'Hakko' }),
			item({ name: 'Old nozzle', printer_id: 'p1', retired: true })
		];
		const names = (f: object) => filterEquipment(items, f).map((e) => e.name);
		expect(names({})).toEqual(['Nozzle', 'PEI sheet', 'Cutters']);
		expect(names({ category: 'accessory' })).toEqual(['PEI sheet', 'Cutters']);
		expect(names({ brand_id: 'b1' })).toEqual(['Nozzle']);
		expect(names({ printer: 'p1' })).toEqual(['Nozzle']);
		expect(names({ printer: 'p1', include_retired: true })).toEqual(['Nozzle', 'Old nozzle']);
		expect(names({ printer: 'none' })).toEqual(['Cutters']);
		expect(names({ q: 'hakko' })).toEqual(['Cutters']);
	});
});
