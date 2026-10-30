//! Writes a copy to the macOS pasteboard (D-073): text, a PNG, file URLs, and Panopin's own type,
//! so the listener never captures the copy again.

use objc2::{rc::autoreleasepool, runtime::ProtocolObject};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSData, NSString, NSURL};

use super::Formats;
use crate::{
    capture::clipboard_mac::OWN_TYPE,
    error::{AppError, Result},
};

pub fn write(formats: &Formats) -> Result<()> {
    autoreleasepool(|_| {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        // Files first: each becomes a pasteboard item, and the rest joins the first one.
        if !formats.paths.is_empty() {
            let urls: Vec<_> = formats
                .paths
                .iter()
                .map(|path| {
                    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
                    ProtocolObject::<dyn NSPasteboardWriting>::from_retained(url)
                })
                .collect();
            if !pasteboard.writeObjects(&NSArray::from_retained_slice(&urls)) {
                return Err(AppError::Mac(
                    "files could not be put on the pasteboard".into(),
                ));
            }
        }
        // SAFETY: the pasteboard type constants are immutable statics provided by AppKit.
        let (string, png) = unsafe { (NSPasteboardTypeString, NSPasteboardTypePNG) };
        if let Some(text) = &formats.text {
            let units = text.strip_suffix(&[0]).unwrap_or(text);
            let text = String::from_utf16_lossy(units);
            pasteboard.setString_forType(&NSString::from_str(&text), string);
        }
        if let Some(bytes) = &formats.png {
            pasteboard.setData_forType(Some(&NSData::with_bytes(bytes)), png);
        }
        pasteboard.setData_forType(
            Some(&NSData::with_bytes(&[1])),
            &NSString::from_str(OWN_TYPE),
        );
        Ok(())
    })
}
