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
