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

pub fn open_settings(app: &AppHandle) -> Result<()> {
    open(app, SETTINGS, "settings.title", (560.0, 420.0))
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
    ] {
        if let Some(window) = app.get_webview_window(label) {
            window.set_title(&i18n::t(lang, key))?;
        }
    }
    Ok(())
}
