//! Text in images on macOS (D-073): Apple Vision's accurate recogniser, in Turkish when the
//! system offers it, else English, else its first language. Vision needs no main thread.

use std::io::Cursor;

use image::{ImageFormat, RgbaImage};
use objc2::{rc::autoreleasepool, runtime::AnyObject, AnyThread};
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};

use super::OcrLanguage;
use crate::error::{AppError, Result};

/// The reading thread runs at background priority, like on Windows.
pub fn init_thread(background: bool) {
    if background {
        // SAFETY: changes only the scheduling class of the calling thread.
        unsafe {
            libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_BACKGROUND, 0);
        }
    }
}

fn accurate_request() -> objc2::rc::Retained<VNRecognizeTextRequest> {
    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    request
}

/// Language tags such as `tr-TR` and `en-US` that the accurate recogniser reads.
fn supported() -> Vec<String> {
    autoreleasepool(|_| {
        let request = accurate_request();
        // SAFETY: only asks Vision for its list of languages.
        match unsafe { request.supportedRecognitionLanguagesAndReturnError() } {
            Ok(list) => list.to_vec().iter().map(|tag| tag.to_string()).collect(),
            Err(_) => Vec::new(),
        }
    })
}

pub fn languages() -> Result<Vec<OcrLanguage>> {
    Ok(supported()
        .into_iter()
        .map(|tag| OcrLanguage {
            name: tag.clone(),
            tag,
        })
        .collect())
}

pub struct Engine {
    pub language: String,
}

impl Engine {
    pub fn new() -> Option<Engine> {
        let supported = supported();
        let pick = |prefix: &str| {
            supported
                .iter()
                .find(|tag| tag.to_ascii_lowercase().starts_with(prefix))
                .cloned()
        };
        let language = pick("tr")
            .or_else(|| pick("en"))
            .or_else(|| supported.first().cloned())?;
        Some(Engine { language })
    }

    /// The text of the picture, one recognised line per line.
    pub fn recognize(&self, pixels: &RgbaImage) -> Result<String> {
        let mut png = Cursor::new(Vec::new());
        pixels.write_to(&mut png, ImageFormat::Png)?;
        let png = png.into_inner();
        autoreleasepool(|_| {
            let data = NSData::with_bytes(&png);
            let options = NSDictionary::<VNImageOption, AnyObject>::new();
            let handler = VNImageRequestHandler::initWithData_options(
                VNImageRequestHandler::alloc(),
                &data,
                &options,
            );
            let request = accurate_request();
            request.setUsesLanguageCorrection(true);
            let languages = NSArray::from_retained_slice(&[NSString::from_str(&self.language)]);
            request.setRecognitionLanguages(&languages);
            let as_request: &VNRequest = &request;
            handler
                .performRequests_error(&NSArray::from_slice(&[as_request]))
                .map_err(|err| AppError::Mac(err.localizedDescription().to_string()))?;
            let mut lines = Vec::new();
            if let Some(results) = request.results() {
                for observation in results.to_vec() {
                    if let Some(best) = observation.topCandidates(1).firstObject() {
                        lines.push(best.string().to_string());
                    }
                }
            }
            Ok(lines.join("\n"))
        })
    }
}
