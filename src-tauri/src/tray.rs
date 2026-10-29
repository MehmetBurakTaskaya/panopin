//! Tray icon, menu and tooltip. The tooltip shows item counts per kind; the icon and menu
//! change while capture is paused.

use tauri::{
    image::Image,
    menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::{
    capture::{self, now_ms},
    error::Result,
    i18n,
    settings::PAUSED_UNTIL_RESUMED,
    shelf,
    store::items::Counts,
    windows, AppState,
};

const TRAY_ID: &str = "panopin";
const PAUSED_ICON: &[u8] = include_bytes!("../icons/tray-paused.png");

pub fn create(app: &AppHandle) -> Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu)
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
        })
        .build(app)?;
    refresh(app)
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let minutes = |n: i64| Some(now_ms() + n * 60_000);
    let result = match event.id().as_ref() {
        "show" => shelf::open(app, false),
        "pause-5m" => capture::set_pause(app, minutes(5)),
        "pause-1h" => capture::set_pause(app, minutes(60)),
        "pause-forever" => capture::set_pause(app, Some(PAUSED_UNTIL_RESUMED)),
        "resume" => capture::set_pause(app, None),
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
}

fn is_paused(app: &AppHandle) -> bool {
    app.state::<AppState>().settings().is_paused(now_ms())
}

/// Rebuilds menu, icon and tooltip from the language, the pause state and the counts.
pub fn refresh(app: &AppHandle) -> Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(menu(app)?))?;
    let icon = if is_paused(app) {
        Some(Image::from_bytes(PAUSED_ICON)?)
    } else {
        app.default_window_icon().cloned()
    };
    tray.set_icon(icon)?;
    tray.set_tooltip(Some(tooltip(app, capture::counts(app))))?;
    Ok(())
}

pub fn show_counts(app: &AppHandle, counts: Counts) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Err(err) = tray.set_tooltip(Some(tooltip(app, counts))) {
            log::warn!("could not update the tray tooltip: {err}");
        }
    }
}

fn tooltip(app: &AppHandle, counts: Counts) -> String {
    let lang = crate::current_lang(app);
    let mut text = format!(
        "Panopin\n{}",
        i18n::t_with(
            lang,
            "tray.status",
            &[
                ("images", counts.image),
                ("texts", counts.text),
                ("links", counts.link)
            ]
        )
    );
    if is_paused(app) {
        text.push('\n');
        text.push_str(&i18n::t(lang, "tray.paused"));
    }
    text
}

fn menu(app: &AppHandle) -> Result<Menu<Wry>> {
    let lang = crate::current_lang(app);
    let item =
        |id: &str, key: &str| MenuItem::with_id(app, id, i18n::t(lang, key), true, None::<&str>);
    let show = item("show", "tray.showShelf")?;
    let pause: Box<dyn IsMenuItem<Wry>> = if is_paused(app) {
        Box::new(item("resume", "tray.resume")?)
    } else {
        Box::new(Submenu::with_items(
            app,
            i18n::t(lang, "tray.pause"),
            true,
            &[
                &item("pause-5m", "tray.pause.5m")?,
                &item("pause-1h", "tray.pause.1h")?,
                &item("pause-forever", "tray.pause.untilResumed")?,
            ],
        )?)
    };
    let settings = item("settings", "tray.settings")?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = item("quit", "tray.quit")?;
    Ok(Menu::with_items(
        app,
        &[&show, pause.as_ref(), &settings, &separator, &quit],
    )?)
}
