import { describe, expect, it } from 'vitest';
import {
	acquiredLine,
	humidityLevel,
	storeName,
	contrastText,
	dateOnly,
	dayOf,
	eventLabel,
	fromLocalInput,
	grams,
	range,
	signedGrams,
	toLocalInput
} from './format';
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

	it('formats signed grams for the weight history', () => {
		expect(signedGrams(-12.5)).toBe('−12.5 g');
		expect(signedGrams(40)).toBe('+40 g');
		expect(signedGrams(0)).toBe('0 g');
	});

	it('labels weight events', () => {
		expect(eventLabel('weigh_in')).toBe('Weigh-in');
		expect(eventLabel('something_new')).toBe('something_new');
	});

	it('keeps plain dates on the same day', () => {
		expect(dateOnly('2026-03-01')).toBe('Mar 1, 2026');
		expect(dateOnly(null)).toBe('');
	});

	it('round-trips datetime-local values', () => {
		const iso = fromLocalInput('2026-11-27T12:30');
		expect(iso).not.toBeNull();
		expect(toLocalInput(iso)).toBe('2026-11-27T12:30');
		expect(fromLocalInput('')).toBeNull();
	});

	it('shows the local day of a timestamp', () => {
		expect(dayOf('2026-11-27T15:00:00Z')).toMatch(/^Nov 2[67], 2026$/);
		expect(dayOf(null)).toBe('');
	});

	it('describes how something was acquired', () => {
		const base = { acquisition: 'bought' as const, purchase_date: null, vendor: null };
		expect(acquiredLine(base)).toBeNull();
		expect(acquiredLine({ ...base, purchase_date: '2026-11-27', vendor: 'Micro Center' })).toBe(
			'Bought Nov 27, 2026 from Micro Center'
		);
		expect(acquiredLine({ ...base, acquisition: 'gift', vendor: 'Sam' })).toBe('Gift from Sam');
		expect(acquiredLine({ ...base, acquisition: 'other' })).toBe('Acquired');
	});

	it('names the store a link points at', () => {
		expect(storeName('https://www.amazon.com/dp/B0')).toBe('amazon.com');
		expect(storeName('https://us.store.bambulab.com/products/pla')).toBe('us.store.bambulab.com');
		expect(storeName(null)).toBe('');
		expect(storeName('not a url')).toBe('');
	});

	it('bands humidity readings', () => {
		expect(humidityLevel(12)).toBe('dry');
		expect(humidityLevel(20)).toBe('ok');
		expect(humidityLevel(40)).toBe('ok');
		expect(humidityLevel(40.1)).toBe('humid');
	});
});
