use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A photo you uploaded. Exactly one of the three owner ids is set.
#[derive(Debug, Clone, Serialize)]
pub struct Photo {
    pub id: Uuid,
    pub spool_id: Option<Uuid>,
    pub filament_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub content_type: String,
    pub size_bytes: i32,
    pub original_name: Option<String>,
    pub caption: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Where the image is served: `/api/photos/{id}/file`.
    pub url: String,
}

/// `GET /api/photos`: give exactly one owner.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PhotoOwner {
    pub spool_id: Option<Uuid>,
    pub filament_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
}

impl PhotoOwner {
    pub fn count(&self) -> usize {
        [self.spool_id, self.filament_id, self.equipment_id]
            .iter()
            .filter(|o| o.is_some())
            .count()
    }
}

/// The image formats browsers show, recognised from a file's first bytes
/// rather than its name or the browser's claim.
pub fn sniff_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some(("image/jpeg", "jpg")),
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some(("image/png", "png")),
        [b'G', b'I', b'F', b'8', ..] => Some(("image/gif", "gif")),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some(("image/webp", "webp")),
        [
            _,
            _,
            _,
            _,
            b'f',
            b't',
            b'y',
            b'p',
            b'a',
            b'v',
            b'i',
            b'f',
            ..,
        ] => Some(("image/avif", "avif")),
        _ => None,
    }
}

impl Photo {
    /// The spool, filament or equipment this photo belongs to.
    pub fn owner(&self) -> PhotoOwner {
        PhotoOwner {
            spool_id: self.spool_id,
            filament_id: self.filament_id,
            equipment_id: self.equipment_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sniff_image;

    #[test]
    fn recognises_image_formats() {
        assert_eq!(
            sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0, 0]).unwrap().0,
            "image/jpeg"
        );
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\n....").unwrap().1, "png");
        assert_eq!(sniff_image(b"GIF89a").unwrap().1, "gif");
        assert_eq!(sniff_image(b"RIFF\0\0\0\0WEBPVP8 ").unwrap().1, "webp");
        assert_eq!(sniff_image(b"\0\0\0\x1cftypavif").unwrap().1, "avif");
        assert!(sniff_image(b"<svg xmlns=").is_none());
        assert!(sniff_image(b"%PDF-1.7").is_none());
        assert!(sniff_image(b"").is_none());
    }
}
