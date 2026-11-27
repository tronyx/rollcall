/** The largest photo the server takes (backend/src/routes/photos.rs). */
export const MAX_PHOTO_BYTES = 20 * 1024 * 1024;

const SHOWN = ['image/jpeg', 'image/png', 'image/webp', 'image/gif', 'image/avif'];

/** Why this file can't be uploaded, or null if it can. */
export function photoProblem(file: { name: string; type: string; size: number }): string | null {
	if (file.size === 0) return `${file.name} is empty.`;
	if (file.size > MAX_PHOTO_BYTES) return `${file.name} is over 20 MB.`;
	// The browser's type can be blank; the server checks the bytes either way.
	if (file.type && !SHOWN.includes(file.type)) {
		return file.type === 'image/heic' || file.type === 'image/heif'
			? `${file.name} is HEIC. Export it as JPEG, or set your camera to "Most compatible".`
			: `${file.name} isn't a JPEG, PNG, WebP, GIF or AVIF image.`;
	}
	return null;
}

/** "2.4 MB", "830 KB". */
export function fileSize(bytes: number): string {
	if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
	return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
