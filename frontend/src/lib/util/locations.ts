import type { Location, Spool } from '#lib/api/types.ts';

/** A location's name, with its owner's when someone shared it with you. */
export function locationLabel(l: Location): string {
	return l.owner_username ? `${l.name} (${l.owner_username})` : l.name;
}

/**
 * The locations a spool can go in. A new spool: yours, or one shared with you
 * to edit (the spool is then its owner's). An existing spool stays with its
 * owner: yours go in your locations, a shared one in its owner's locations
 * that you can edit.
 */
export function placeable(
	locations: Location[],
	spool?: Pick<Spool, 'owner_username'>
): Location[] {
	if (!spool) return locations.filter((l) => l.access !== 'view');
	if (!spool.owner_username) return locations.filter((l) => l.access === 'owner');
	return locations.filter((l) => l.access === 'edit' && l.owner_username === spool.owner_username);
}
