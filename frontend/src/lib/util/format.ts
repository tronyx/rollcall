/** 1234.5 → "1,235 g"; 12.3 → "12.3 g" (decimals only below 100 g). */
export function grams(g: number): string {
	const digits = Math.abs(g) < 100 && g % 1 !== 0 ? 1 : 0;
	return `${g.toLocaleString('en-US', { maximumFractionDigits: digits, minimumFractionDigits: digits })} g`;
}

export function kg(value: number): string {
	return `${value.toLocaleString('en-US', { maximumFractionDigits: 2 })} kg`;
}

export function range(min: number | null, max: number | null, unit: string): string | null {
	if (min == null && max == null) return null;
	if (min == null || max == null || min === max) return `${min ?? max}${unit}`;
	return `${min}–${max}${unit}`;
}

/** Text color that stays readable on top of a swatch. */
export function contrastText(hex: string | null): 'black' | 'white' {
	if (!hex || !/^#[0-9a-f]{6}$/i.test(hex)) return 'black';
	const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
	const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
	return lum > 0.55 ? 'black' : 'white';
}

/** Bar color for remaining filament. */
export function levelColor(pct: number, lowStock: boolean): string {
	if (lowStock || pct < 20) return 'bg-red-500';
	if (pct < 50) return 'bg-amber-500';
	return 'bg-emerald-500';
}

/** "2026-11-27T17:55:00Z" → "Nov 27, 2026, 12:55 PM" in the viewer's time zone. */
export function dateTime(iso: string | null): string {
	if (!iso) return '';
	return new Date(iso).toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' });
}

/** A timestamp's local date: "Nov 27, 2026". */
export function dayOf(iso: string | null): string {
	if (!iso) return '';
	return new Date(iso).toLocaleDateString('en-US', { dateStyle: 'medium' });
}

/** "2026-11-27" (a date with no time) → "Nov 27, 2026", without a time-zone shift. */
export function dateOnly(ymd: string | null): string {
	if (!ymd) return '';
	const [y, m, d] = ymd.split('-').map(Number);
	return new Date(y, m - 1, d).toLocaleDateString('en-US', { dateStyle: 'medium' });
}

const eventLabels: Record<string, string> = {
	initial: 'Added',
	print: 'Print',
	quick_adjust: 'Quick adjust',
	weigh_in: 'Weigh-in',
	correction: 'Correction'
};

export function eventLabel(kind: string): string {
	return eventLabels[kind] ?? kind;
}

/** −12.5 → "−12.5 g", 40 → "+40 g", 0 → "0 g". */
export function signedGrams(g: number): string {
	if (g === 0) return grams(0);
	return `${g > 0 ? '+' : '−'}${grams(Math.abs(g))}`;
}

/** An ISO timestamp → the value a `datetime-local` input expects, in local time. */
export function toLocalInput(iso: string | null): string {
	if (!iso) return '';
	const d = new Date(iso);
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** A `datetime-local` value (local time) → ISO timestamp, or null when empty. */
export function fromLocalInput(value: string): string | null {
	return value ? new Date(value).toISOString() : null;
}

/** "Bought Nov 27, 2026 from Micro Center", "Gift from Sam", or null if nothing is known. */
export function acquiredLine(x: {
	acquisition: 'bought' | 'gift' | 'other';
	purchase_date: string | null;
	vendor: string | null;
}): string | null {
	if (!x.purchase_date && !x.vendor && x.acquisition === 'bought') return null;
	const verb = { bought: 'Bought', gift: 'Gift', other: 'Acquired' }[x.acquisition];
	return [verb, x.purchase_date && dateOnly(x.purchase_date), x.vendor && `from ${x.vendor}`]
		.filter(Boolean)
		.join(' ');
}

/** The site a product link points at, as a store name: "amazon.com". */
export function storeName(url: string | null | undefined): string {
	if (!url) return '';
	try {
		return new URL(url).hostname.replace(/^www\./, '');
	} catch {
		return '';
	}
}
