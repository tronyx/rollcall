import type {
	AppSettings,
	Brand,
	CreateSpool,
	Filament,
	InventoryStats,
	Location,
	Material,
	NewLocation,
	NewPrinter,
	ParseResult,
	Printer,
	Spool,
	SpoolFilter,
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
		res = await fetch(`/api${path}`, {
			method,
			headers: body === undefined ? undefined : { 'content-type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
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
	settings: () => request<AppSettings>('GET', '/settings'),
	updateSettings: (body: AppSettings) => request<AppSettings>('PUT', '/settings', body)
};
