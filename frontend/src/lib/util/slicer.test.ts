import { describe, expect, it } from 'vitest';
import type { Filament, Printer } from '../api/types';
import { emptyValues } from './tuned';
import {
	baseOptions,
	bambuPrinterCode,
	buildProfile,
	profileFileName,
	profileName,
	profileValues,
	suggestBase
} from './slicer';

const filament = {
	id: 'f1',
	brand_name: 'Bambu Lab',
	material_code: 'PLA',
	product_name: 'PLA Basic',
	color_name: 'Jade White',
	color_hex: '#f5f5dc',
	diameter_mm: 1.75,
	nozzle_temp_min_c: 190,
	nozzle_temp_max_c: 230,
	bed_temp_min_c: 55,
	bed_temp_max_c: 65,
	density_g_cm3: null
} as unknown as Filament;

const printer = (name: string, model: string | null = null) =>
	({ id: 'p', name, model, nozzle_diameter_mm: 0.4, is_default: false, notes: null }) as Printer;

describe('Bambu printer codes', () => {
	it('reads names and models', () => {
		expect(bambuPrinterCode('Bambu Lab A1 mini')).toBe('A1M');
		expect(bambuPrinterCode('A1')).toBe('A1');
		expect(bambuPrinterCode('X1 Carbon')).toBe('X1C');
		expect(bambuPrinterCode('my x1c')).toBe('X1C');
		expect(bambuPrinterCode('H2D Pro')).toBe('H2DP');
		expect(bambuPrinterCode('Prusa MK4')).toBeNull();
	});
});

describe('base presets', () => {
	it('suggests the generic preset for the material and printer', () => {
		expect(suggestBase('orca', 'PLA', null)).toBe('Generic PLA @System');
		expect(suggestBase('orca', 'pla-silk', null)).toBe('Generic PLA Silk @System');
		expect(suggestBase('bambu', 'PLA', printer('X1C'))).toBe('Generic PLA');
		expect(suggestBase('bambu', 'PETG', printer('Kitchen', 'Bambu Lab A1 mini'))).toBe(
			'Generic PETG @BBL A1M'
		);
		expect(suggestBase('bambu', 'ABS', printer('A1 mini'))).toBe('');
		expect(suggestBase('bambu', 'PLA', printer('MK4'))).toBe('');
		expect(suggestBase('orca', 'WOOD', null)).toBe('');
	});
	it('lists the choices', () => {
		expect(baseOptions('orca', 'PETG')).toEqual(['Generic PETG @System']);
		expect(baseOptions('bambu', 'PLA')).toContain('Generic PLA @BBL H2D');
		expect(baseOptions('bambu', 'PLA')[0]).toBe('Generic PLA');
	});
});

describe('names', () => {
	it('names the preset after the filament and printer', () => {
		expect(profileName(filament, printer('X1C'))).toBe(
			'Bambu Lab PLA Basic Jade White @X1C (Rollcall)'
		);
		expect(profileName(filament, null)).toBe('Bambu Lab PLA Basic Jade White (Rollcall)');
		expect(profileFileName('A/B: "C" (Rollcall)')).toBe('A B C (Rollcall).json');
	});
});

describe('buildProfile', () => {
	const tuned = {
		...emptyValues(),
		nozzle_temp_c: 215,
		flow_ratio: 0.98,
		pressure_advance: 0.025,
		fan_speed_pct: 80
	};

	it('writes an OrcaSlicer preset with a value for each extruder type', () => {
		const p = buildProfile({
			slicer: 'orca',
			name: 'N',
			base: 'Generic PLA @System',
			filament,
			tuned,
			notes: 'Hi'
		});
		expect(p).toMatchObject({
			type: 'filament',
			name: 'N',
			inherits: 'Generic PLA @System',
			from: 'User',
			filament_settings_id: ['N'],
			filament_vendor: ['Bambu Lab'],
			filament_type: ['PLA'],
			filament_colour: ['#F5F5DC'],
			filament_diameter: ['1.75'],
			filament_density: ['1.24'],
			nozzle_temperature: ['215'],
			nozzle_temperature_initial_layer: ['215'],
			nozzle_temperature_range_low: ['190'],
			nozzle_temperature_range_high: ['230'],
			textured_plate_temp: ['60'],
			pressure_advance: ['0.025'],
			enable_pressure_advance: ['1'],
			fan_min_speed: ['80'],
			fan_max_speed: ['80'],
			filament_notes: ['Hi']
		});
		expect(p.filament_flow_ratio).toEqual(Array(6).fill('0.98'));
		expect((p.filament_extruder_variant as string[]).length).toBe(6);
		expect(p).not.toHaveProperty('filament_retraction_length');
		expect(p).not.toHaveProperty('filament_max_volumetric_speed');
		expect(typeof p.version).toBe('string');
	});

	it('writes a Bambu Studio preset with single values', () => {
		const p = buildProfile({ slicer: 'bambu', name: 'N', base: '', filament, tuned, notes: 'Hi' });
		expect(p.inherits).toBe('');
		expect(p.filament_flow_ratio).toEqual(['0.98']);
		expect(p).not.toHaveProperty('filament_extruder_variant');
		expect(p.filament_notes).toBe('Hi');
	});

	it('falls back to the maker values', () => {
		const p = buildProfile({ slicer: 'orca', name: 'N', base: '', filament, tuned: emptyValues() });
		expect(p.nozzle_temperature).toEqual(['210']);
		expect(p.hot_plate_temp).toEqual(['60']);
		expect(p).not.toHaveProperty('pressure_advance');
		expect(p).not.toHaveProperty('filament_extruder_variant');
		const v = profileValues(filament, emptyValues());
		expect(v.nozzle).toEqual({ label: 'Nozzle', value: '210°C', from: 'maker' });
		expect(profileValues(filament, tuned).nozzle.from).toBe('tuned');
	});
});
