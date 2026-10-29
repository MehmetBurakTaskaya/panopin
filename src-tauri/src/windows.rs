//! Normal windows, created on demand and destroyed on close to keep memory low.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::{error::Result, i18n};

pub const SETTINGS: &str = "settings";
pub const ONBOARDING: &str = "onboarding";

fn open(app: &AppHandle, label: &str, title_key: &str, size: (f64, f64)) -> Result<()> {
    if let Some(window) = app.get_webview_window(label) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(i18n::t(crate::current_lang(app), title_key))
        .inner_size(size.0, size.1)
        .min_inner_size(440.0, 320.0)
        .center()
        .build()?;
    Ok(())
}

pub const LIBRARY: &str = "library";

/// Mica needs Windows 11 (build 22000); elsewhere the Library paints its own opaque background.
#[cfg(windows)]
fn supports_mica() -> bool {
    use windows::{
        core::w,
        Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ},
    };
    let mut buffer = [0u16; 16];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: the buffer and its size in bytes are passed together.
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    let len = (size as usize / 2).saturating_sub(1).min(buffer.len());
    read.is_ok()
        && String::from_utf16_lossy(&buffer[..len])
            .parse::<u32>()
            .is_ok_and(|build| build >= 22000)
}

fn window_theme(app: &AppHandle) -> Option<tauri::Theme> {
    match app.state::<crate::AppState>().settings().theme {
        crate::settings::Theme::Light => Some(tauri::Theme::Light),
        crate::settings::Theme::Dark => Some(tauri::Theme::Dark),
        crate::settings::Theme::System => None,
    }
}

/// The Library: a normal window with Mica on Windows 11 (D-005, D-053).
pub fn open_library(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(LIBRARY) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(app, LIBRARY, WebviewUrl::App("index.html".into()))
        .title(i18n::t(crate::current_lang(app), "library.title"))
        .inner_size(1180.0, 760.0)
        .min_inner_size(720.0, 480.0)
        .center()
        .theme(window_theme(app))
        // Cards are dragged onto folders with HTML drag and drop, which Tauri's file drop would
        // otherwise swallow on Windows.
        .disable_drag_drop_handler();
    #[cfg(windows)]
    let builder = if supports_mica() {
        builder
            .transparent(true)
            .effects(
                tauri::window::EffectsBuilder::new()
                    .effect(tauri::window::Effect::Mica)
                    .build(),
            )
            .initialization_script("document.documentElement.dataset.material = 'mica';")
    } else {
        builder
    };
    builder.build()?;
    Ok(())
}

/// Follows the theme setting in the windows that draw a system material.
pub fn apply_theme(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(LIBRARY) {
        window.set_theme(window_theme(app))?;
    }
    Ok(())
}

pub fn open_settings(app: &AppHandle) -> Result<()> {
    open(app, SETTINGS, "settings.title", (560.0, 640.0))
}

/// First-run screen: nothing is captured until it is confirmed.
pub fn open_onboarding(app: &AppHandle) -> Result<()> {
    open(app, ONBOARDING, "onboarding.windowTitle", (600.0, 720.0))
}

pub fn close_onboarding(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(ONBOARDING) {
        window.close()?;
    }
    Ok(())
}

/// Keeps window titles in the selected language.
pub fn retitle(app: &AppHandle, lang: i18n::Lang) -> Result<()> {
    for (label, key) in [
        (SETTINGS, "settings.title"),
        (ONBOARDING, "onboarding.windowTitle"),
        (LIBRARY, "library.title"),
    ] {
        if let Some(window) = app.get_webview_window(label) {
            window.set_title(&i18n::t(lang, key))?;
        }
    }
    Ok(())
}
