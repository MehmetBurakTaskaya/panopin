//! `Windows.Media.Ocr`: free, offline and built in, so no model ships with Panopin.

use image::{imageops::FilterType, RgbaImage};
use windows::{
    core::HSTRING,
    Globalization::Language,
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::System::{
        Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN},
        WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
    },
};

use super::OcrLanguage;
use crate::error::Result;

/// WinRT needs an apartment on every thread that uses it; the queue thread also drops to
/// background priority (CPU, disk and memory), so reading never slows the user down.
pub fn init_thread(background: bool) {
    // SAFETY: plain calls on the current thread; an apartment that already exists is fine.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
        if background {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
        }
    }
}

pub fn languages() -> Result<Vec<OcrLanguage>> {
    let mut out = Vec::new();
    for language in OcrEngine::AvailableRecognizerLanguages()? {
        out.push(OcrLanguage {
            tag: language.LanguageTag()?.to_string(),
            name: language.NativeName()?.to_string(),
        });
    }
    Ok(out)
}

pub struct Engine {
    inner: OcrEngine,
    pub language: String,
}

impl Engine {
    /// Turkish first, then English, then the user's own languages.
    pub fn new() -> Option<Engine> {
        for tag in ["tr", "en-US", "en"] {
            let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag)) else {
                continue;
            };
            if OcrEngine::IsLanguageSupported(&language).unwrap_or(false) {
                if let Ok(inner) = OcrEngine::TryCreateFromLanguage(&language) {
                    return Some(Engine {
                        inner,
                        language: tag.to_owned(),
                    });
                }
            }
        }
        let inner = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;
        let language = inner
            .RecognizerLanguage()
            .and_then(|l| l.LanguageTag())
            .map(|tag| tag.to_string())
            .unwrap_or_default();
        Some(Engine { inner, language })
    }

    /// The text of the picture, one line per recognised line.
    pub fn recognize(&self, pixels: &RgbaImage) -> Result<String> {
        let limit = OcrEngine::MaxImageDimension()?.max(1);
        let (width, height) = pixels.dimensions();
        let scaled;
        let pixels = if width > limit || height > limit {
            let factor = f64::from(limit) / f64::from(width.max(height));
            let w = ((f64::from(width) * factor).floor() as u32).max(1);
            let h = ((f64::from(height) * factor).floor() as u32).max(1);
            scaled = image::imageops::resize(pixels, w, h, FilterType::Triangle);
            &scaled
        } else {
            pixels
        };
        let (width, height) = pixels.dimensions();
        // BGRA laid on white: transparent areas read as paper, not as black.
        let mut bgra = Vec::with_capacity(width as usize * height as usize * 4);
        for pixel in pixels.pixels() {
            let [r, g, b, a] = pixel.0;
            let on_white =
                |c: u8| ((u32::from(c) * u32::from(a) + 255 * (255 - u32::from(a))) / 255) as u8;
            bgra.extend_from_slice(&[on_white(b), on_white(g), on_white(r), 255]);
        }
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        let buffer = writer.DetachBuffer()?;
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
            BitmapAlphaMode::Ignore,
        )?;
        let result = self.inner.RecognizeAsync(&bitmap)?.join()?;
        let mut text = String::new();
        for line in result.Lines()? {
            let line = line.Text()?.to_string();
            let line = line.trim();
            if !line.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(line);
            }
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads a picture named by `PANOPIN_OCR_SAMPLE`; needs an OCR language, so it runs on
    /// request only: `cargo test -- --ignored ocr_reads`.
    #[test]
    #[ignore]
    fn ocr_reads_a_sample_picture() {
        let path = std::env::var("PANOPIN_OCR_SAMPLE").expect("PANOPIN_OCR_SAMPLE");
        let pixels = image::open(path).unwrap().to_rgba8();
        init_thread(false);
        let engine = Engine::new().expect("an OCR language");
        println!("languages: {:?}", languages().unwrap());
        println!("reading in {}", engine.language);
        println!("text: {:?}", engine.recognize(&pixels).unwrap());
    }
}
