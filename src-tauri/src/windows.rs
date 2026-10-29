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
pub fn supports_mica() -> bool {
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

// Mica arrives with macOS vibrancy in Phase 8.
#[cfg(not(windows))]
#[allow(dead_code)]
pub fn supports_mica() -> bool {
    false
}

/// Windows that sit on Mica when Windows 11 has it and the setting allows it (D-005, D-060).
const MATERIAL_WINDOWS: [&str; 2] = [LIBRARY, SETTINGS];

#[cfg_attr(not(windows), allow(dead_code))]
fn material_on(app: &AppHandle) -> bool {
    supports_mica()
        && app
            .state::<crate::AppState>()
            .settings()
            .appearance
            .material
}

#[cfg(windows)]
fn mica() -> tauri::utils::config::WindowEffectsConfig {
    tauri::window::EffectsBuilder::new()
        .effect(tauri::window::Effect::Mica)
        .build()
}

/// A window that can show Mica is created transparent, so the material can be switched on and
/// off while it is open; the page paints its own background whenever Mica is off.
fn with_material<'a>(
    app: &AppHandle,
    builder: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>,
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    #[cfg(windows)]
    if supports_mica() {
        let builder = builder.transparent(true);
        return if material_on(app) {
            builder
                .effects(mica())
                .initialization_script("document.documentElement.dataset.material = 'mica';")
        } else {
            builder
        };
    }
    let _ = app;
    builder
}

fn open_with_material(
    app: &AppHandle,
    label: &str,
    title_key: &str,
    size: (f64, f64),
    min: (f64, f64),
) -> Result<()> {
    if let Some(window) = app.get_webview_window(label) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(i18n::t(crate::current_lang(app), title_key))
        .inner_size(size.0, size.1)
        .min_inner_size(min.0, min.1)
        .center()
        .theme(window_theme(app))
        // Cards are dragged onto folders with HTML drag and drop, which Tauri's file drop would
        // otherwise swallow on Windows.
        .disable_drag_drop_handler();
    with_material(app, builder).build()?;
    Ok(())
}

/// The Library: a normal window with Mica on Windows 11 (D-005, D-053).
pub fn open_library(app: &AppHandle) -> Result<()> {
    open_with_material(
        app,
        LIBRARY,
        "library.title",
        (1180.0, 760.0),
        (720.0, 480.0),
    )
}

/// Follows the theme setting in the windows that draw a system material.
pub fn apply_theme(app: &AppHandle) -> Result<()> {
    for label in MATERIAL_WINDOWS {
        if let Some(window) = app.get_webview_window(label) {
            window.set_theme(window_theme(app))?;
        }
    }
    Ok(())
}

/// Switches Mica on or off in the open windows; the pages follow through `settings:changed`.
pub fn apply_material(app: &AppHandle) -> Result<()> {
    #[cfg(windows)]
    for label in MATERIAL_WINDOWS {
        if let Some(window) = app.get_webview_window(label) {
            window.set_effects(material_on(app).then(mica))?;
        }
    }
    #[cfg(not(windows))]
    let _ = app;
    Ok(())
}

/// Settings: tabs on the left, Mica like the Library.
pub fn open_settings(app: &AppHandle) -> Result<()> {
    open_with_material(
        app,
        SETTINGS,
        "settings.title",
        (820.0, 640.0),
        (640.0, 480.0),
    )
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
