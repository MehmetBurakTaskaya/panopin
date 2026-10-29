//! Card thumbnails (D-025): longest side 512 px, JPEG for opaque images, PNG when alpha is used.

use std::io::Cursor;

use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageFormat, RgbaImage,
};

use crate::error::Result;

pub const MAX_SIDE: u32 = 512;

pub struct Thumbnail {
    pub bytes: Vec<u8>,
    pub ext: &'static str,
}

pub fn make(image: &RgbaImage) -> Result<Thumbnail> {
    let (width, height) = image.dimensions();
    let longest = width.max(height).max(1);
    let small = if longest > MAX_SIDE {
        let scale =
            |side: u32| (u64::from(side) * u64::from(MAX_SIDE) / u64::from(longest)).max(1) as u32;
        image::imageops::resize(image, scale(width), scale(height), FilterType::Triangle)
    } else {
        image.clone()
    };
    let mut bytes = Vec::new();
    if small.pixels().any(|p| p[3] < 255) {
        small.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)?;
        Ok(Thumbnail { bytes, ext: "png" })
    } else {
        let rgb = DynamicImage::ImageRgba8(small).into_rgb8();
        JpegEncoder::new_with_quality(&mut bytes, 85).encode_image(&rgb)?;
        Ok(Thumbnail { bytes, ext: "jpg" })
    }
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use super::*;
    use crate::media::decode::decode;

    #[test]
    fn large_images_shrink_to_the_card_size() {
        let image = RgbaImage::from_pixel(2048, 1024, Rgba([200, 100, 50, 255]));
        let thumb = make(&image).unwrap();
        assert_eq!(thumb.ext, "jpg");
        assert_eq!(decode(&thumb.bytes).unwrap().dimensions(), (512, 256));
    }

    #[test]
    fn small_images_keep_their_size_and_alpha() {
        let image = RgbaImage::from_pixel(40, 30, Rgba([0, 0, 0, 128]));
        let thumb = make(&image).unwrap();
        assert_eq!(thumb.ext, "png");
        let decoded = decode(&thumb.bytes).unwrap();
        assert_eq!(decoded.dimensions(), (40, 30));
        assert_eq!(decoded.get_pixel(0, 0)[3], 128);
    }
}
