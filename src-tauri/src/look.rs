//! What the system says about the look (D-060, D-073): whether Mica is available and the accent
//! colour.
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

/// The accent chosen in System Settings → Appearance (D-073). Windows materials have no macOS
/// counterpart here yet, and the accent is read again whenever a window opens.
#[cfg(target_os = "macos")]
pub fn current() -> SystemLook {
    use objc2_app_kit::{NSColor, NSColorSpace};
    let accent = NSColor::controlAccentColor()
        .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
        .map(|color| {
            [
                color.redComponent(),
                color.greenComponent(),
                color.blueComponent(),
            ]
        });
    SystemLook {
        mica: false,
        // As on Windows: a little darker on light surfaces, lighter on dark ones.
        accent_light: accent.map(|rgb| shade(rgb, 0.0, 0.12)),
        accent_dark: accent.map(|rgb| shade(rgb, 1.0, 0.25)),
    }
}

/// `rgb` (0–1) moved `amount` of the way to `toward` (0 = black, 1 = white), as `#rrggbb`.
#[cfg(any(test, target_os = "macos"))]
fn shade(rgb: [f64; 3], toward: f64, amount: f64) -> String {
    let [r, g, b] = rgb.map(|channel| {
        let channel = channel.clamp(0.0, 1.0);
        ((channel + (toward - channel) * amount) * 255.0).round() as u8
    });
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(not(any(windows, target_os = "macos")))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shades_move_towards_black_or_white() {
        assert_eq!(shade([0.0, 0.478, 1.0], 0.0, 0.0), "#007aff");
        assert_eq!(shade([1.0, 1.0, 1.0], 0.0, 0.5), "#808080");
        assert_eq!(shade([0.0, 0.0, 0.0], 1.0, 0.25), "#404040");
        assert_eq!(shade([1.2, -0.1, 0.5], 0.0, 0.0), "#ff0080");
    }
}
