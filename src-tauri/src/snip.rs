//! The screen-snip shortcut: opens the Windows Snipping Tool overlay
//! (`ms-screenclip:`); the next picture it puts on the clipboard opens on the shelf in the crop
//! editor (D-062).
#![cfg_attr(not(windows), allow(dead_code))]

use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, Manager};

use crate::{capture::now_ms, error::Result};

/// How long a snip is waited for after the shortcut.
const WAIT_MS: i64 = 2 * 60 * 1000;

#[derive(Default)]
pub struct SnipState(Mutex<Option<i64>>);

pub fn start(app: &AppHandle) -> Result<()> {
    *app.state::<SnipState>()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(now_ms() + WAIT_MS);
    open_overlay()
}

/// Whether a picture that just arrived is the snip; a snip is taken once.
pub fn take(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<SnipState>() else {
        return false;
    };
    let mut until = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    let waiting = until.is_some_and(|until| until > now_ms());
    *until = None;
    waiting
}

#[cfg(windows)]
fn open_overlay() -> Result<()> {
    use windows::{
        core::w,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    // SAFETY: constant strings; the protocol handler belongs to Windows.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("ms-screenclip:"),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(windows::core::Error::from_thread().into())
    }
}

#[cfg(not(windows))]
fn open_overlay() -> Result<()> {
    Err(crate::error::AppError::Unsupported)
}
