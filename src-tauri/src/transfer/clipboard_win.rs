//! Writes a copy to the Windows clipboard. Every copy carries the `Panopin.OwnCopy` marker, so
//! the clipboard listener never captures it again (D-040).

use std::{ptr, thread, time::Duration};

use windows::{
    core::w,
    Win32::{
        Foundation::{GlobalFree, HANDLE, HWND},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
                SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
        },
    },
};

use super::Formats;
use crate::{
    capture::clipboard_win::OWN_FORMAT,
    error::{AppError, Result},
};

pub(super) const CF_DIB: u32 = 8;
pub(super) const CF_UNICODETEXT: u32 = 13;
pub(super) const CF_HDROP: u32 = 15;

/// `owner` must be a window of ours: with no owner, Windows rejects the data after emptying.
pub fn write(owner: isize, formats: &Formats) -> Result<()> {
    // SAFETY: the clipboard is opened and closed on this thread, and every handle given to
    // SetClipboardData is a fresh movable global block that the system takes over.
    unsafe {
        open(HWND(owner as *mut _))?;
        let result = fill(formats);
        let _ = CloseClipboard();
        result
    }
}

/// Another process may hold the clipboard for a moment; retry a few times, briefly.
pub(super) unsafe fn open(owner: HWND) -> Result<()> {
    for attempt in 1..=5 {
        if OpenClipboard(Some(owner)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10 * attempt));
    }
    Err(AppError::ClipboardBusy)
}

unsafe fn fill(formats: &Formats) -> Result<()> {
    EmptyClipboard()?;
    if let Some(text) = &formats.text {
        let bytes: Vec<u8> = text.iter().flat_map(|unit| unit.to_le_bytes()).collect();
        put(CF_UNICODETEXT, &bytes)?;
    }
    if let Some(png) = &formats.png {
        put(RegisterClipboardFormatW(w!("PNG")), png)?;
    }
    if let Some(dib) = &formats.dib {
        put(CF_DIB, dib)?;
    }
    if let Some(files) = &formats.files {
        put(CF_HDROP, files)?;
    }
    put(RegisterClipboardFormatW(OWN_FORMAT), &1u32.to_le_bytes())
}

pub(super) unsafe fn put(format: u32, bytes: &[u8]) -> Result<()> {
    let global = GlobalAlloc(GMEM_MOVEABLE, bytes.len())?;
    let target = GlobalLock(global).cast::<u8>();
    if target.is_null() {
        let err = windows::core::Error::from_thread();
        let _ = GlobalFree(Some(global));
        return Err(err.into());
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
    // Returns "false" once the lock count reaches zero, which is the expected outcome here.
    let _ = GlobalUnlock(global);
    if let Err(err) = SetClipboardData(format, Some(HANDLE(global.0))) {
        let _ = GlobalFree(Some(global));
        return Err(err.into());
    }
    Ok(())
}
