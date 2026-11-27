import type {
	AppSettings,
	Brand,
	CreateSpool,
	Equipment,
	EquipmentFilter,
	Filament,
	HumidityReading,
	InventoryStats,
	Location,
	Material,
	NewHumidityReading,
	NewLocation,
	NewPrinter,
	Photo,
	Print,
	AssignSlot,
	SlicerProfile,
	PhotoOwner,
	ParseResult,
	Printer,
	SaveEquipment,
	SaveTunedSettings,
	Spool,
	SpoolFilter,
	TunedSettings,
	UpdateFilament,
	UpdateSpool,
	WeightAdjustment,
	WeightChange,
	WeightEvent
} from './types';

export class ApiError extends Error {
	constructor(
		public status: number,
		public code: string,
		message: string
	) {
		super(message);
	}
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
	let res: Response;
	try {
		// FormData and Blob bodies carry their own content type.
		const form = body instanceof FormData || body instanceof Blob;
		res = await fetch(`/api${path}`, {
			method,
			headers: body === undefined || form ? undefined : { 'content-type': 'application/json' },
			body: body === undefined ? undefined : form ? body : JSON.stringify(body)
		});
	} catch {
		throw new ApiError(0, 'network', "Can't reach the Rollcall server.");
	}
	if (res.status === 204) return undefined as T;
	const data = await res.json().catch(() => null);
	if (!res.ok) {
		throw new ApiError(
			res.status,
			data?.error?.code ?? 'unknown',
			data?.error?.message ?? `Request failed (${res.status})`
		);
	}
	return data as T;
}

export function toQuery(params: object): string {
	const q = new URLSearchParams();
	for (const [k, v] of Object.entries(params)) {
		if (v === undefined || v === null || v === '' || v === false) continue;
		q.set(k, String(v));
	}
	const s = q.toString();
	return s ? `?${s}` : '';
}

export const api = {
	stats: () => request<InventoryStats>('GET', '/stats'),
	spools: (filter: SpoolFilter = {}) => request<Spool[]>('GET', `/spools${toQuery(filter)}`),
	spool: (id: string) => request<Spool>('GET', `/spools/${id}`),
	createSpool: (body: CreateSpool) => request<Spool>('POST', '/spools', body),
	updateSpool: (id: string, body: UpdateSpool) => request<Spool>('PUT', `/spools/${id}`, body),
	deleteSpool: (id: string) => request<void>('DELETE', `/spools/${id}`),
	adjustWeight: (id: string, body: WeightAdjustment) =>
		request<WeightChange>('POST', `/spools/${id}/weight`, body),
	weightEvents: (id: string, limit = 50) =>
		request<WeightEvent[]>('GET', `/spools/${id}/weight-events?limit=${limit}`),
	filament: (id: string) => request<Filament>('GET', `/filaments/${id}`),
	updateFilament: (id: string, body: UpdateFilament) =>
		request<Filament>('PUT', `/filaments/${id}`, body),
	parseUrl: (url: string) => request<ParseResult>('POST', '/filaments/parse-url', { url }),
	brands: () => request<Brand[]>('GET', '/brands'),
	materials: () => request<Material[]>('GET', '/materials'),
	locations: () => request<Location[]>('GET', '/locations'),
	createLocation: (body: NewLocation) => request<Location>('POST', '/locations', body),
	updateLocation: (id: string, body: NewLocation) =>
		request<Location>('PUT', `/locations/${id}`, body),
	deleteLocation: (id: string) => request<void>('DELETE', `/locations/${id}`),
	printers: () => request<Printer[]>('GET', '/printers'),
	createPrinter: (body: NewPrinter) => request<Printer>('POST', '/printers', body),
	updatePrinter: (id: string, body: NewPrinter) => request<Printer>('PUT', `/printers/${id}`, body),
	deletePrinter: (id: string) => request<void>('DELETE', `/printers/${id}`),
	equipment: (filter: EquipmentFilter = {}) =>
		request<Equipment[]>('GET', `/equipment${toQuery(filter)}`),
	createEquipment: (body: SaveEquipment) => request<Equipment>('POST', '/equipment', body),
	updateEquipment: (id: string, body: SaveEquipment) =>
		request<Equipment>('PUT', `/equipment/${id}`, body),
	deleteEquipment: (id: string) => request<void>('DELETE', `/equipment/${id}`),
	humidity: (spoolId: string) => request<HumidityReading[]>('GET', `/spools/${spoolId}/humidity`),
	addHumidity: (spoolId: string, body: NewHumidityReading) =>
		request<HumidityReading>('POST', `/spools/${spoolId}/humidity`, body),
	deleteHumidity: (id: string) => request<void>('DELETE', `/humidity/${id}`),
	photos: (owner: PhotoOwner) => request<Photo[]>('GET', `/photos${toQuery(owner)}`),
	uploadPhoto: (owner: PhotoOwner, file: File, caption?: string) => {
		const form = new FormData();
		for (const [k, v] of Object.entries(owner)) form.append(k, v);
		if (caption) form.append('caption', caption);
		form.append('file', file);
		return request<Photo>('POST', '/photos', form);
	},
	setPhotoCaption: (id: string, caption: string | null) =>
		request<Photo>('PATCH', `/photos/${id}`, { caption }),
	deletePhoto: (id: string) => request<void>('DELETE', `/photos/${id}`),
	prints: (filter: { pending?: boolean; limit?: number } = {}) =>
		request<Print[]>('GET', `/prints${toQuery(filter)}`),
	/** G-code, or just its comment lines (see `gcodeSummary`). */
	logPrint: (name: string, gcode: string) =>
		request<Print>(
			'POST',
			`/prints${toQuery({ name })}`,
			new Blob([gcode], { type: 'text/plain' })
		),
	assignPrintSlot: (id: string, slot: number, body: AssignSlot) =>
		request<Print>('PUT', `/prints/${id}/slots/${slot}`, body),
	undoPrintSlot: (id: string, slot: number) =>
		request<Print>('POST', `/prints/${id}/slots/${slot}/undo`),
	deletePrint: (id: string) => request<void>('DELETE', `/prints/${id}`),
	slicerProfiles: (filamentId?: string) =>
		request<SlicerProfile[]>('GET', `/slicer-profiles${toQuery({ filament_id: filamentId })}`),
	saveSlicerProfile: (body: SlicerProfile) =>
		request<SlicerProfile>('PUT', '/slicer-profiles', body),
	tuned: (filamentId: string) => request<TunedSettings[]>('GET', `/filaments/${filamentId}/tuned`),
	saveTuned: (filamentId: string, body: SaveTunedSettings) =>
		request<TunedSettings>('PUT', `/filaments/${filamentId}/tuned`, body),
	deleteTuned: (id: string) => request<void>('DELETE', `/tuned/${id}`),
	settings: () => request<AppSettings>('GET', '/settings'),
	updateSettings: (body: AppSettings) => request<AppSettings>('PUT', '/settings', body)
};
