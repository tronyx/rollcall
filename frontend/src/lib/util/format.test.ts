import { describe, expect, it } from 'vitest';
import { contrastText, grams, range } from './format';
import { toQuery } from '../api/client';

describe('format', () => {
	it('formats grams', () => {
		expect(grams(1234.5)).toBe('1,235 g');
		expect(grams(12.3)).toBe('12.3 g');
		expect(grams(0)).toBe('0 g');
	});

	it('formats ranges', () => {
		expect(range(190, 230, '°C')).toBe('190–230°C');
		expect(range(null, 200, ' mm/s')).toBe('200 mm/s');
		expect(range(null, null, '°C')).toBeNull();
	});

	it('picks readable text on swatches', () => {
		expect(contrastText('#FFFFFF')).toBe('black');
		expect(contrastText('#000000')).toBe('white');
		expect(contrastText(null)).toBe('black');
	});

	it('drops empty query params', () => {
		expect(toQuery({ q: 'pla', low_stock: false, material: '', include_empty: true })).toBe(
			'?q=pla&include_empty=true'
		);
		expect(toQuery({})).toBe('');
	});
});
