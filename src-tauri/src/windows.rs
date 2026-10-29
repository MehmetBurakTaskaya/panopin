//! Normal windows, created on demand and destroyed on close to keep memory low.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::{error::Result, i18n};

pub const SETTINGS: &str = "settings";

pub fn open_settings(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title(i18n::t(crate::current_lang(app), "settings.title"))
        .inner_size(560.0, 420.0)
        .min_inner_size(440.0, 320.0)
        .center()
        .build()?;
    Ok(())
}

/// Keeps window titles in the selected language.
pub fn retitle(app: &AppHandle, lang: i18n::Lang) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        window.set_title(&i18n::t(lang, "settings.title"))?;
    }
    Ok(())
}
