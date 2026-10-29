//! Screen recordings (D-024, D-058): the poster frame comes from the Windows shell thumbnail
//! provider and the length and frame size from the property system, so no ffmpeg ships with
//! Panopin. The file itself is only read.

use std::path::Path;

use image::RgbaImage;

/// What the shell knows about a recording; any part may be missing.
#[derive(Debug, Default)]
pub struct VideoFacts {
    pub poster: Option<RgbaImage>,
    pub duration_ms: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[cfg(windows)]
pub fn probe(path: &Path) -> VideoFacts {
    use windows::{
        core::{Interface, HSTRING},
        Win32::{
            Foundation::SIZE,
            Storage::EnhancedStorage::{
                PKEY_Media_Duration, PKEY_Video_FrameHeight, PKEY_Video_FrameWidth,
            },
            System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
            UI::Shell::{
                IShellItem2, IShellItemImageFactory, SHCreateItemFromParsingName,
                SIIGBF_BIGGERSIZEOK, SIIGBF_THUMBNAILONLY,
            },
        },
    };

    // SAFETY: COM is initialised on this thread first; every interface is released on drop and
    // the bitmap is deleted after it was copied.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        // The shell parses only backslash paths.
        let name = HSTRING::from(path.to_string_lossy().replace('/', "\\"));
        let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem2>(&name, None) else {
            return VideoFacts::default();
        };
        // The property system counts in 100-nanosecond steps.
        let duration_ms = item
            .GetUInt64(&PKEY_Media_Duration)
            .ok()
            .and_then(|d| i64::try_from(d / 10_000).ok());
        let width = item
            .GetUInt32(&PKEY_Video_FrameWidth)
            .ok()
            .filter(|w| *w > 0);
        let height = item
            .GetUInt32(&PKEY_Video_FrameHeight)
            .ok()
            .filter(|h| *h > 0);
        let poster = item
            .cast::<IShellItemImageFactory>()
            .ok()
            .and_then(|factory| {
                factory
                    .GetImage(
                        SIZE { cx: 512, cy: 512 },
                        SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK,
                    )
                    .ok()
            })
            .and_then(|bitmap| {
                let pixels = bitmap_pixels(bitmap);
                let _ = windows::Win32::Graphics::Gdi::DeleteObject(bitmap.into());
                pixels
            });
        VideoFacts {
            poster,
            duration_ms,
            width,
            height,
        }
    }
}

/// Copies a shell bitmap into RGBA pixels (top-down 32-bit; an all-zero alpha means opaque).
#[cfg(windows)]
unsafe fn bitmap_pixels(bitmap: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<RgbaImage> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS,
    };

    let mut info = BITMAP::default();
    let read = GetObjectW(
        bitmap.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(std::ptr::from_mut(&mut info).cast()),
    );
    if read == 0 || info.bmWidth <= 0 || info.bmHeight == 0 {
        return None;
    }
    let (width, height) = (info.bmWidth, info.bmHeight.abs());
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bgra = vec![0u8; width as usize * height as usize * 4];
    let dc = CreateCompatibleDC(None);
    let lines = GetDIBits(
        dc,
        bitmap,
        0,
        height as u32,
        Some(bgra.as_mut_ptr().cast()),
        &mut header,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);
    if lines == 0 {
        return None;
    }
    let opaque = bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 0);
    for pixel in bgra.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        if opaque {
            pixel[3] = 255;
        }
    }
    RgbaImage::from_raw(width as u32, height as u32, bgra)
}

#[cfg(not(windows))]
pub fn probe(_path: &Path) -> VideoFacts {
    // AVFoundation arrives in Phase 8.
    VideoFacts::default()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// Probes a recording named by `PANOPIN_VIDEO_SAMPLE`; runs on request only:
    /// `cargo test -- --ignored probes_a_sample`.
    #[test]
    #[ignore]
    fn probes_a_sample_recording() {
        let path = std::env::var("PANOPIN_VIDEO_SAMPLE").expect("PANOPIN_VIDEO_SAMPLE");
        let facts = probe(Path::new(&path));
        println!(
            "poster {:?}, length {:?} ms, frame {:?}×{:?}",
            facts.poster.as_ref().map(RgbaImage::dimensions),
            facts.duration_ms,
            facts.width,
            facts.height
        );
        assert!(facts.poster.is_some() && facts.duration_ms.is_some());
    }
}
