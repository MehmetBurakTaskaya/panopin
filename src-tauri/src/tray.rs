//! Tray icon and menu. Items appear as their features land; Phase 1 has show, settings, quit.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Wry,
};

use crate::{
    error::Result,
    i18n::{self, Lang},
    shelf, windows,
};

const TRAY_ID: &str = "panopin";

pub fn create(app: &AppHandle, lang: Lang) -> Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(i18n::t(lang, "tray.tooltip"))
        .menu(&menu(app, lang)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let result = match event.id().as_ref() {
                "show" => shelf::open(app, false),
                "settings" => windows::open_settings(app),
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(err) = result {
                log::error!("tray action failed: {err}");
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(err) = shelf::toggle_from_tray(tray.app_handle()) {
                    log::error!("could not toggle the shelf: {err}");
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Rebuilds the menu after a language change.
pub fn relabel(app: &AppHandle, lang: Lang) -> Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu(app, lang)?))?;
        tray.set_tooltip(Some(i18n::t(lang, "tray.tooltip")))?;
    }
    Ok(())
}

fn menu(app: &AppHandle, lang: Lang) -> Result<Menu<Wry>> {
    let show = MenuItem::with_id(
        app,
        "show",
        i18n::t(lang, "tray.showShelf"),
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        i18n::t(lang, "tray.settings"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", i18n::t(lang, "tray.quit"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    Ok(Menu::with_items(
        app,
        &[&show, &settings, &separator, &quit],
    )?)
}
