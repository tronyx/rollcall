import { describe, expect, it } from 'vitest';
import { fileSize, MAX_PHOTO_BYTES, photoProblem } from './photos';

const file = (name: string, type: string, size = 1000) => ({ name, type, size });

describe('photoProblem', () => {
	it('accepts images browsers show', () => {
		expect(photoProblem(file('a.jpg', 'image/jpeg'))).toBeNull();
		expect(photoProblem(file('a.webp', 'image/webp'))).toBeNull();
		expect(photoProblem(file('a', ''))).toBeNull();
	});
	it('rejects empty, large and other files', () => {
		expect(photoProblem(file('a.jpg', 'image/jpeg', 0))).toMatch(/empty/);
		expect(photoProblem(file('a.jpg', 'image/jpeg', MAX_PHOTO_BYTES + 1))).toMatch(/20 MB/);
		expect(photoProblem(file('a.pdf', 'application/pdf'))).toMatch(/isn't/);
		expect(photoProblem(file('a.svg', 'image/svg+xml'))).toMatch(/isn't/);
		expect(photoProblem(file('IMG.HEIC', 'image/heic'))).toMatch(/HEIC/);
	});
});

describe('fileSize', () => {
	it('uses KB and MB', () => {
		expect(fileSize(500)).toBe('1 KB');
		expect(fileSize(850_000)).toBe('830 KB');
		expect(fileSize(2.5 * 1024 * 1024)).toBe('2.5 MB');
	});
});
