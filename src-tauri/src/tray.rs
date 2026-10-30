//! Tray icon, menu and tooltip. The tooltip shows item counts per kind; the icon and menu
//! change while capture is paused and when an update is on offer.

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
    updates, windows, AppState,
};

const TRAY_ID: &str = "panopin";

/// One tray picture per small-icon size from 100 % to 200 % scaling, drawn from the SVG at that
/// size so Windows never shrinks a larger one (D-067). `npm run icons` makes them.
#[cfg(not(target_os = "macos"))]
type Pictures = [(u32, &'static [u8]); 4];
#[cfg(not(target_os = "macos"))]
macro_rules! pictures {
    ($name:literal) => {
        [
            (
                16,
                include_bytes!(concat!("../icons/tray/", $name, "-16.png")),
            ),
            (
                20,
                include_bytes!(concat!("../icons/tray/", $name, "-20.png")),
            ),
            (
                24,
                include_bytes!(concat!("../icons/tray/", $name, "-24.png")),
            ),
            (
                32,
                include_bytes!(concat!("../icons/tray/", $name, "-32.png")),
            ),
        ]
    };
}
/// The picture for the tray's state: whether capture is paused and an update is on offer.
#[cfg(not(target_os = "macos"))]
fn state_picture(paused: bool, update: bool) -> &'static [u8] {
    const NORMAL: Pictures = pictures!("normal");
    const PAUSED: Pictures = pictures!("paused");
    const UPDATE: Pictures = pictures!("update");
    const PAUSED_UPDATE: Pictures = pictures!("paused-update");
    let set = match (paused, update) {
        (false, false) => &NORMAL,
        (true, false) => &PAUSED,
        (false, true) => &UPDATE,
        (true, true) => &PAUSED_UPDATE,
    };
    // The smallest picture at least as large as Windows' small icons, else the largest.
    let want = small_icon_size();
    set.iter()
        .find(|(size, _)| *size >= want)
        .unwrap_or(&set[set.len() - 1])
        .1
}

/// The macOS menu bar gets template pictures, black on transparent, which macOS tints for light
/// and dark menu bars (D-073).
#[cfg(target_os = "macos")]
fn state_picture(paused: bool, update: bool) -> &'static [u8] {
    match (paused, update) {
        (false, false) => include_bytes!("../icons/tray/mac-normal.png"),
        (true, false) => include_bytes!("../icons/tray/mac-paused.png"),
        (false, true) => include_bytes!("../icons/tray/mac-update.png"),
        (true, true) => include_bytes!("../icons/tray/mac-paused-update.png"),
    }
}

#[cfg(windows)]
fn small_icon_size() -> u32 {
    use ::windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
    // SAFETY: no pointers; the size follows the display scaling Panopin started with.
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) };
    u32::try_from(size).unwrap_or(16)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn small_icon_size() -> u32 {
    32
}

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
        "library" => windows::open_library(app),
        "pause-5m" => capture::set_pause(app, minutes(5)),
        "pause-1h" => capture::set_pause(app, minutes(60)),
        "pause-forever" => capture::set_pause(app, Some(PAUSED_UNTIL_RESUMED)),
        "resume" => capture::set_pause(app, None),
        "settings" => windows::open_settings(app),
        // Settings shows the result; a check runs unless an update is already on offer.
        "updates" => {
            if updates::offered(app).is_none() {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { updates::check(&app).await });
            }
            windows::open_settings_on(app, "updates")
        }
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

/// Rebuilds menu, icon and tooltip from the language, the pause state, the update on offer and
/// the counts.
pub fn refresh(app: &AppHandle) -> Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(menu(app)?))?;
    let picture = state_picture(is_paused(app), updates::offered(app).is_some());
    tray.set_icon(Some(Image::from_bytes(picture)?))?;
    #[cfg(target_os = "macos")]
    tray.set_icon_as_template(true)?;
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
    if let Some(version) = updates::offered(app) {
        text.push('\n');
        text.push_str(&i18n::t(lang, "tray.updateAvailable").replace("{version}", &version));
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
    let library = item("library", "tray.library")?;
    let settings = item("settings", "tray.settings")?;
    let updates = match updates::offered(app) {
        Some(version) => MenuItem::with_id(
            app,
            "updates",
            i18n::t(lang, "tray.updateTo").replace("{version}", &version),
            true,
            None::<&str>,
        )?,
        None => item("updates", "tray.checkUpdates")?,
    };
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = item("quit", "tray.quit")?;
    Ok(Menu::with_items(
        app,
        &[
            &show,
            &library,
            pause.as_ref(),
            &settings,
            &updates,
            &separator,
            &quit,
        ],
    )?)
}
