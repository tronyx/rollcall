import { describe, expect, it } from 'vitest';
import { safeNext } from './auth.ts';

describe('safeNext', () => {
	it('keeps paths on this site', () => {
		expect(safeNext('/spools/abc?tab=1')).toBe('/spools/abc?tab=1');
		expect(safeNext('/')).toBe('/');
	});
	it('refuses other sites and junk', () => {
		expect(safeNext(null)).toBe('/');
		expect(safeNext('')).toBe('/');
		expect(safeNext('https://evil.example')).toBe('/');
		expect(safeNext('//evil.example')).toBe('/');
		expect(safeNext('/\\evil.example')).toBe('/');
		expect(safeNext('javascript:alert(1)')).toBe('/');
	});
});
