//! Turns clipboard and file image data into RGBA pixels (D-015).

use std::io::Cursor;

use image::{ImageFormat, RgbaImage};

use crate::error::{AppError, Result};

/// PNG, JPEG, BMP, GIF or WebP file bytes.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage> {
    Ok(image::load_from_memory(bytes)?.into_rgba8())
}

/// A `CF_DIB` / `CF_DIBV5` payload: a bitmap without the 14-byte file header, which is rebuilt
/// here so the regular BMP decoder can read it.
pub fn decode_dib(dib: &[u8]) -> Result<RgbaImage> {
    const BI_BITFIELDS: u32 = 3;
    const BI_ALPHABITFIELDS: u32 = 6;
    let u32_at = |at: usize| -> Option<u32> {
        dib.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let header_size = u32_at(0).ok_or(AppError::BadImage("truncated bitmap header"))?;
    let bit_count = dib
        .get(14..16)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(AppError::BadImage("truncated bitmap header"))?;
    let compression = u32_at(16).ok_or(AppError::BadImage("truncated bitmap header"))?;
    let colors_used = u32_at(32).ok_or(AppError::BadImage("truncated bitmap header"))?;
    if header_size < 40 {
        return Err(AppError::BadImage("unknown bitmap header"));
    }
    // A plain 40-byte header keeps its colour masks right after it.
    let masks = match (header_size, compression) {
        (40, BI_BITFIELDS) => 12,
        (40, BI_ALPHABITFIELDS) => 16,
        _ => 0,
    };
    let palette = match (bit_count, colors_used) {
        (1..=8, 0) => (1u32 << bit_count) * 4,
        (_, n) => n.saturating_mul(4),
    };
    let total =
        u32::try_from(dib.len() + 14).map_err(|_| AppError::BadImage("bitmap too large"))?;
    let pixels_at = 14 + header_size + masks + palette;

    let mut bmp = Vec::with_capacity(dib.len() + 14);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&total.to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&pixels_at.to_le_bytes());
    bmp.extend_from_slice(dib);

    let mut image = image::load_from_memory_with_format(&bmp, ImageFormat::Bmp)?.into_rgba8();
    // Many apps leave the alpha byte of 32-bit bitmaps at zero; such images are opaque.
    if bit_count == 32 && image.pixels().all(|p| p[3] == 0) {
        for pixel in image.pixels_mut() {
            pixel[3] = 255;
        }
    }
    Ok(image)
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    image.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?;
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::store::identity::image_identity;

    /// A bottom-up 32-bit `BITMAPINFOHEADER` DIB, pixels given top row first as BGRA.
    pub(crate) fn dib32(width: u32, height: u32, top_down_bgra: &[[u8; 4]]) -> Vec<u8> {
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&(width as i32).to_le_bytes());
        dib.extend_from_slice(&(height as i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&[0; 24]); // BI_RGB, sizes, resolution, palette counts
        for row in top_down_bgra.chunks(width as usize).rev() {
            for px in row {
                dib.extend_from_slice(px);
            }
        }
        dib
    }

    #[test]
    fn dib_with_empty_alpha_is_opaque() {
        let dib = dib32(2, 1, &[[0, 0, 255, 0], [255, 0, 0, 0]]);
        let image = decode_dib(&dib).unwrap();
        assert_eq!(image.dimensions(), (2, 1));
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(1, 0).0, [0, 0, 255, 255]);
    }

    #[test]
    fn same_pixels_from_dib_and_png_share_an_identity() {
        let dib = dib32(
            2,
            2,
            &[
                [10, 20, 30, 255],
                [40, 50, 60, 255],
                [70, 80, 90, 255],
                [1, 2, 3, 255],
            ],
        );
        let from_dib = decode_dib(&dib).unwrap();
        let from_png = decode(&encode_png(&from_dib).unwrap()).unwrap();
        assert_eq!(
            from_dib.get_pixel(0, 0).0,
            [30, 20, 10, 255],
            "top-left after bottom-up flip"
        );
        assert_eq!(
            image_identity(2, 2, from_dib.as_raw()),
            image_identity(2, 2, from_png.as_raw())
        );
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(decode_dib(&[1, 2, 3]).is_err());
        assert!(decode(b"not an image").is_err());
    }
}
