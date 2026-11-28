import { describe, expect, it } from 'vitest';
import type { Location } from '#lib/api/types.ts';
import { locationLabel, placeable } from './locations.ts';

const loc = (name: string, access: Location['access'], owner: string | null = null): Location => ({
	id: name,
	name,
	kind: 'shelf',
	description: null,
	sort_order: 0,
	access,
	owner_username: owner,
	share_count: 0
});
const all = [
	loc('Mine', 'owner'),
	loc('Alice edit', 'edit', 'alice'),
	loc('Alice view', 'view', 'alice'),
	loc('Sam edit', 'edit', 'sam')
];
const names = (l: Location[]) => l.map((x) => x.name);

describe('locations', () => {
	it('labels shared locations with their owner', () => {
		expect(locationLabel(all[0])).toBe('Mine');
		expect(locationLabel(all[1])).toBe('Alice edit (alice)');
	});
	it('puts a new spool anywhere you can add to', () => {
		expect(names(placeable(all))).toEqual(['Mine', 'Alice edit', 'Sam edit']);
	});
	it('keeps a spool with its owner', () => {
		expect(names(placeable(all, { owner_username: null }))).toEqual(['Mine']);
		expect(names(placeable(all, { owner_username: 'alice' }))).toEqual(['Alice edit']);
	});
});
