// Mirrors the Rust DTOs in backend/src/models and backend/src/scraper.

export type SpoolStatus = 'sealed' | 'active' | 'drying' | 'empty' | 'archived';
export type WeightEventKind = 'initial' | 'print' | 'quick_adjust' | 'weigh_in' | 'correction';
export type LocationKind = 'dry_box' | 'ams_slot' | 'printer' | 'dryer' | 'shelf' | 'other';
export type SpecSource = 'manual' | 'scraped' | 'llm';
export type AcquisitionKind = 'bought' | 'gift' | 'other';

export interface Spool {
	id: string;
	label_code: string | null;
	status: SpoolStatus;
	initial_weight_g: number;
	remaining_weight_g: number;
	tare_weight_g: number | null;
	remaining_pct: number;
	is_low_stock: boolean;
	purchase_price: number | null;
	currency: string;
	purchase_date: string | null;
	vendor: string | null;
	acquisition: AcquisitionKind;
	discounted: boolean;
	discount_note: string | null;
	lot_number: string | null;
	opened_at: string | null;
	last_dried_at: string | null;
	emptied_at: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
	filament_id: string;
	product_name: string | null;
	color_name: string;
	color_hex: string | null;
	color_hex_secondary: string | null;
	diameter_mm: number;
	image_url: string | null;
	brand_id: string;
	brand_name: string;
	material_id: string;
	material_code: string;
	location_id: string | null;
	location_name: string | null;
	location_kind: LocationKind | null;
}

export interface SpoolFilter {
	q?: string;
	material?: string;
	material_family?: string;
	brand_id?: string;
	location_id?: string;
	status?: SpoolStatus;
	color?: string;
	low_stock?: boolean;
	include_empty?: boolean;
}

export interface FilamentSpecs {
	nozzle_temp_min_c: number | null;
	nozzle_temp_max_c: number | null;
	bed_temp_min_c: number | null;
	bed_temp_max_c: number | null;
	print_speed_min_mm_s: number | null;
	print_speed_max_mm_s: number | null;
	drying_temp_c: number | null;
	drying_time_min: number | null;
	density_g_cm3?: number | null;
	td_value?: number | null;
}

export interface NewFilament extends FilamentSpecs {
	brand_name: string;
	material_code: string;
	product_name: string | null;
	color_name: string;
	color_hex: string | null;
	diameter_mm: number;
	nominal_weight_g: number | null;
	source_url: string | null;
	image_url: string | null;
	spec_source: SpecSource;
	scrape_raw?: unknown;
}

/** A filament product, as `GET /api/filaments/{id}` returns it. */
export interface Filament extends FilamentSpecs {
	id: string;
	brand_id: string;
	brand_name: string;
	material_id: string;
	material_code: string;
	product_name: string | null;
	color_name: string;
	color_hex: string | null;
	color_hex_secondary: string | null;
	diameter_mm: number;
	nominal_weight_g: number | null;
	typical_tare_g: number | null;
	source_url: string | null;
	image_url: string | null;
	spec_source: SpecSource;
	scraped_at: string | null;
	created_at: string;
	updated_at: string;
}

/** `PUT /api/filaments/{id}`: replaces every field, so send them all. */
export interface UpdateFilament extends FilamentSpecs {
	brand_name: string;
	material_code: string;
	product_name: string | null;
	color_name: string;
	color_hex: string | null;
	color_hex_secondary: string | null;
	diameter_mm: number;
	nominal_weight_g: number | null;
	typical_tare_g: number | null;
	source_url: string | null;
	image_url: string | null;
}

/** `PUT /api/spools/{id}`: replaces the editable fields. Remaining weight
 * only changes through the weight endpoint, so every change is logged. */
export interface UpdateSpool {
	filament_id: string;
	label_code: string | null;
	location_id: string | null;
	status: SpoolStatus;
	initial_weight_g: number;
	tare_weight_g: number | null;
	purchase_price: number | null;
	currency: string;
	purchase_date: string | null;
	vendor: string | null;
	acquisition: AcquisitionKind;
	discounted: boolean;
	discount_note: string | null;
	lot_number: string | null;
	last_dried_at: string | null;
	notes: string | null;
}

export interface CreateSpool {
	filament_id?: string;
	filament?: NewFilament;
	label_code?: string | null;
	location_id?: string | null;
	status?: SpoolStatus;
	initial_weight_g: number;
	remaining_weight_g?: number | null;
	tare_weight_g?: number | null;
	purchase_price?: number | null;
	currency?: string | null;
	purchase_date?: string | null;
	vendor?: string | null;
	acquisition?: AcquisitionKind;
	discounted?: boolean;
	discount_note?: string | null;
	notes?: string | null;
}

export interface WeightAdjustment {
	kind: Exclude<WeightEventKind, 'initial'>;
	delta_g?: number;
	gross_reading_g?: number;
	print_name?: string;
	note?: string;
}

export interface WeightEvent {
	id: number;
	spool_id: string;
	kind: WeightEventKind;
	delta_g: number;
	weight_after_g: number;
	gross_reading_g: number | null;
	print_name: string | null;
	note: string | null;
	created_at: string;
}

export interface WeightChange {
	spool: Spool;
	event: WeightEvent;
}

export interface InventoryStats {
	spools_in_stock: number;
	spools_sealed: number;
	spools_active: number;
	spools_drying: number;
	spools_empty: number;
	remaining_kg: number;
	low_stock_count: number;
	low_stock_threshold_g: number;
}

export interface Brand {
	id: string;
	name: string;
	website: string | null;
}

export interface Material {
	id: string;
	code: string;
	display_name: string;
	family: string;
	is_abrasive: boolean;
}

export interface Location {
	id: string;
	name: string;
	kind: LocationKind;
	description: string | null;
	sort_order: number;
}

export interface NewLocation {
	name: string;
	kind: LocationKind;
	description: string | null;
	sort_order: number;
}

export interface Printer {
	id: string;
	name: string;
	model: string | null;
	nozzle_diameter_mm: number;
	is_default: boolean;
	notes: string | null;
}

export type NewPrinter = Omit<Printer, 'id'>;

/** The values you can tune per printer. Empty ones fall back to the filament's. */
export interface TunedValues {
	nozzle_temp_c: number | null;
	first_layer_nozzle_temp_c: number | null;
	bed_temp_c: number | null;
	print_speed_mm_s: number | null;
	max_volumetric_speed_mm3_s: number | null;
	flow_ratio: number | null;
	pressure_advance: number | null;
	retraction_length_mm: number | null;
	retraction_speed_mm_s: number | null;
	fan_speed_pct: number | null;
	drying_temp_c: number | null;
	drying_time_min: number | null;
}

export interface TunedSettings extends TunedValues {
	id: string;
	filament_id: string;
	/** Null for the settings that apply to any printer. */
	printer_id: string | null;
	printer_name: string | null;
	notes: string | null;
	verified_at: string | null;
	updated_at: string;
}

export interface SaveTunedSettings extends TunedValues {
	printer_id: string | null;
	notes: string | null;
	verified: boolean;
}

export interface AppSettings {
	low_stock_threshold_g: number;
	default_currency: string;
}

export type FieldSource =
	| 'shopify_json'
	| 'json_ld'
	| 'site_adapter'
	| 'spec_table'
	| 'meta_tags'
	| 'title'
	| 'page_text'
	| 'llm'
	| 'url'
	| 'derived';

export interface ParseDraft extends FilamentSpecs {
	title: string | null;
	brand_name: string | null;
	material_code: string | null;
	product_name: string | null;
	color_name: string | null;
	color_hex: string | null;
	image_url: string | null;
	diameter_mm: number | null;
	nominal_weight_g: number | null;
	price: number | null;
	currency: string | null;
}

export interface ParseResult {
	url: string;
	final_url: string | null;
	fetched: boolean;
	blocked: boolean;
	complete: boolean;
	draft: ParseDraft;
	sources: Partial<Record<keyof ParseDraft, FieldSource>>;
	missing: (keyof ParseDraft)[];
	warnings: string[];
}

export interface HumidityReading {
	id: string;
	spool_id: string;
	measured_at: string;
	humidity_pct: number;
	temperature_c: number | null;
	note: string | null;
}

export interface NewHumidityReading {
	measured_at?: string | null;
	humidity_pct: number;
	temperature_c?: number | null;
	note?: string | null;
}

export type EquipmentCategory = 'equipment' | 'accessory';

/** A piece of equipment (nozzle, AMS, dryer) or an accessory (build plate, tool). */
export interface Equipment {
	id: string;
	category: EquipmentCategory;
	kind: string | null;
	name: string;
	brand_id: string | null;
	brand_name: string | null;
	printer_id: string | null;
	printer_name: string | null;
	model: string | null;
	quantity: number;
	retired: boolean;
	acquisition: AcquisitionKind;
	acquired_on: string | null;
	acquired_from: string | null;
	price: number | null;
	currency: string;
	discounted: boolean;
	discount_note: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
	/** The first photo, shown on the card. */
	photo_url: string | null;
}

export interface SaveEquipment {
	category: EquipmentCategory;
	kind: string | null;
	name: string;
	brand_name: string | null;
	printer_id: string | null;
	model: string | null;
	quantity: number;
	retired: boolean;
	acquisition: AcquisitionKind;
	acquired_on: string | null;
	acquired_from: string | null;
	price: number | null;
	currency: string | null;
	discounted: boolean;
	discount_note: string | null;
	notes: string | null;
}

export interface EquipmentFilter {
	category?: EquipmentCategory | '';
	brand_id?: string;
	/** A printer id, or "none" for items not tied to a printer. */
	printer?: string;
	q?: string;
	include_retired?: boolean;
}

/** Whose photo it is: exactly one of these. */
export type PhotoOwner = { spool_id: string } | { filament_id: string } | { equipment_id: string };

export interface Photo {
	id: string;
	spool_id: string | null;
	filament_id: string | null;
	equipment_id: string | null;
	content_type: string;
	size_bytes: number;
	original_name: string | null;
	caption: string | null;
	created_at: string;
	/** Where the image is served: `/api/photos/{id}/file`. */
	url: string;
}
