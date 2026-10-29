//! What Windows says about the look (D-060): whether Mica is available and the accent colour.
//! A change of the accent colour in Windows reaches every window at once.

use serde::Serialize;
use tauri::AppHandle;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemLook {
    pub mica: bool,
    /// The accent for light backgrounds and the one for dark backgrounds, as `#rrggbb`.
    pub accent_light: Option<String>,
    pub accent_dark: Option<String>,
}

#[cfg(windows)]
pub fn current() -> SystemLook {
    use windows::UI::ViewManagement::{UIColorType, UISettings};
    let hex = |settings: &UISettings, kind: UIColorType| {
        settings
            .GetColorValue(kind)
            .ok()
            .map(|c| format!("#{:02x}{:02x}{:02x}", c.R, c.G, c.B))
    };
    let settings = UISettings::new().ok();
    SystemLook {
        mica: crate::windows::supports_mica(),
        // Darker shades read well on light surfaces, lighter ones on dark surfaces.
        accent_light: settings
            .as_ref()
            .and_then(|s| hex(s, UIColorType::AccentDark1)),
        accent_dark: settings
            .as_ref()
            .and_then(|s| hex(s, UIColorType::AccentLight2)),
    }
}

#[cfg(not(windows))]
pub fn current() -> SystemLook {
    SystemLook::default()
}

/// Keeps the Windows settings object alive, so its change event keeps firing.
#[cfg(windows)]
pub struct LookWatch(#[allow(dead_code)] windows::UI::ViewManagement::UISettings);

#[cfg(windows)]
pub fn watch(app: &AppHandle) {
    use tauri::{Emitter, Manager};
    use windows::{Foundation::TypedEventHandler, UI::ViewManagement::UISettings};
    let Ok(settings) = UISettings::new() else {
        return;
    };
    let notify = app.clone();
    let handler = TypedEventHandler::new(move |_, _| {
        if let Err(err) = notify.emit("look:changed", current()) {
            log::warn!("could not announce the new accent colour: {err}");
        }
        Ok(())
    });
    match settings.ColorValuesChanged(&handler) {
        Ok(_) => {
            app.manage(LookWatch(settings));
        }
        Err(err) => log::warn!("accent colour changes will not be followed: {err}"),
    }
}

#[cfg(not(windows))]
pub fn watch(_app: &AppHandle) {}
