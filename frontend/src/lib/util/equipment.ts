// Suggestions and summaries for the equipment page.

import type { Equipment, EquipmentCategory, EquipmentFilter } from '../api/types.ts';

/** Kinds offered while typing; anything else can be typed too. */
export const KIND_SUGGESTIONS: Record<EquipmentCategory, string[]> = {
	equipment: [
		'Nozzle',
		'Hotend',
		'AMS',
		'Filament dryer',
		'Dry box',
		'Enclosure',
		'Camera',
		'Upgrade'
	],
	accessory: ['Build plate', 'Tool', 'Spare part', 'Lubricant', 'Glue', 'Cleaning', 'Storage']
};

/** "Bambu Lab · H2D-NZ-04" or null. */
export function makerLine(e: Pick<Equipment, 'brand_name' | 'model'>): string | null {
	return [e.brand_name, e.model].filter(Boolean).join(' · ') || null;
}

/** Brands and printers that the given items use, for filters that only offer useful choices. */
export function usedIds(items: Pick<Equipment, 'brand_id' | 'printer_id'>[]) {
	return {
		brands: new Set(items.map((i) => i.brand_id).filter((x): x is string => !!x)),
		printers: new Set(items.map((i) => i.printer_id).filter((x): x is string => !!x))
	};
}

/** The same filters as `GET /api/equipment`, applied to a loaded list. */
export function filterEquipment(items: Equipment[], f: EquipmentFilter): Equipment[] {
	const needle = f.q?.trim().toLowerCase();
	return items.filter(
		(e) =>
			(!f.category || e.category === f.category) &&
			(!f.brand_id || e.brand_id === f.brand_id) &&
			(!f.printer || (f.printer === 'none' ? !e.printer_id : e.printer_id === f.printer)) &&
			(f.include_retired || !e.retired) &&
			(!needle ||
				[e.name, e.kind, e.model, e.brand_name, e.notes].some((v) =>
					v?.toLowerCase().includes(needle)
				))
	);
}
