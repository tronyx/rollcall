import { describe, expect, it } from 'vitest';
import type { Filament, Spool } from '../api/types';
import {
	labelable,
	labelSize,
	LABEL_SIZES,
	productLine,
	qrPath,
	spoolLink,
	tempLine,
	validBase
} from './label';

describe('labels', () => {
	it('draws a QR code with its quiet zone', () => {
		const qr = qrPath('https://rollcall.example/spools/0193b0c4-8a7e-7c1e-9d2f-5b1a2c3d4e5f');
		// The 68-character link needs version 5 (37 modules) at level M, plus 2 each side.
		expect(qr.size).toBe(41);
		// The top-left finder pattern starts with a run of 7 dark modules.
		expect(qr.d.startsWith('M2 2h7v1h-7z')).toBe(true);
		expect(qr.d).not.toContain('NaN');
	});

	it('is the same code for the same text', () => {
		expect(qrPath('abc').d).toBe(qrPath('abc').d);
		expect(qrPath('abc').d).not.toBe(qrPath('abd').d);
	});

	it('builds links from a base address', () => {
		expect(spoolLink('http://nas.local:8080/', '/spools/1')).toBe('http://nas.local:8080/spools/1');
		expect(spoolLink(' https://rollcall.example ', '/spools/1')).toBe(
			'https://rollcall.example/spools/1'
		);
		expect(validBase('http://192.168.1.20:8080')).toBe(true);
		expect(validBase('rollcall.local')).toBe(false);
		expect(validBase('ftp://x')).toBe(false);
	});

	it('falls back to the first size', () => {
		expect(labelSize('62x29').width_mm).toBe(62);
		expect(labelSize('nope')).toBe(LABEL_SIZES[0]);
		expect(labelSize(null)).toBe(LABEL_SIZES[0]);
	});

	it('summarizes print temperatures', () => {
		const f = {
			nozzle_temp_min_c: 210,
			nozzle_temp_max_c: 230,
			bed_temp_min_c: 60,
			bed_temp_max_c: 60
		} as Filament;
		expect(tempLine(f)).toBe('210–230°C · bed 60°C');
		expect(tempLine({ ...f, bed_temp_min_c: null, bed_temp_max_c: null })).toBe('210–230°C');
		expect(
			tempLine({
				...f,
				nozzle_temp_min_c: null,
				nozzle_temp_max_c: null,
				bed_temp_min_c: null,
				bed_temp_max_c: null
			})
		).toBeNull();
		expect(tempLine(null)).toBeNull();
	});

	it('names the product without repeating the material', () => {
		expect(productLine({ material_code: 'PLA', product_name: 'PLA Basic' })).toBe('PLA Basic');
		expect(productLine({ material_code: 'PETG', product_name: 'PolyLite' })).toBe('PETG PolyLite');
		expect(productLine({ material_code: 'PLA', product_name: 'Silk-PLA' })).toBe('Silk-PLA');
		expect(productLine({ material_code: 'PLA+', product_name: null })).toBe('PLA+');
		expect(productLine({ material_code: 'PLA', product_name: 'PLAyful' })).toBe('PLA PLAyful');
	});

	it('skips empty and archived spools by default', () => {
		const spools = ['sealed', 'active', 'drying', 'empty', 'archived'].map(
			(status, i) => ({ id: String(i), status }) as Spool
		);
		expect(labelable(spools).map((s) => s.status)).toEqual(['sealed', 'active', 'drying']);
	});
});
