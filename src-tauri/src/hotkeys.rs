//! Global shortcuts (D-020). A combination that another app already holds is reported and
//! skipped, never taken over. While Settings records a new one, all of Panopin's are released
//! (D-061).

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::{
    error::{AppError, Result},
    settings::Shortcuts,
    AppState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    ToggleShelf,
    OpenShelfSearch,
    Snip,
    OpenLibrary,
    CopyLastImage,
    PauseCapture,
}

/// Registered shortcut ids mapped to their actions, and the ones another app holds.
#[derive(Default)]
pub struct HotkeyState(Mutex<Registered>);

#[derive(Default)]
struct Registered {
    actions: HashMap<u32, Action>,
    unavailable: Vec<Action>,
}

/// Every action with its accelerator; `None` = switched off.
fn active(shortcuts: &Shortcuts) -> [(Action, Option<&str>); 6] {
    [
        (Action::ToggleShelf, shortcuts.shelf.as_deref()),
        (Action::OpenShelfSearch, shortcuts.shelf_search.as_deref()),
        (Action::Snip, shortcuts.snip.as_deref()),
        (Action::OpenLibrary, shortcuts.library.as_deref()),
        (Action::CopyLastImage, shortcuts.copy_last_image.as_deref()),
        (Action::PauseCapture, shortcuts.pause_capture.as_deref()),
    ]
}

/// The same combination twice, or one that does not parse, is refused before saving.
pub fn validate(shortcuts: &Shortcuts) -> Result<()> {
    let mut seen = Vec::new();
    for (_, accelerator) in active(shortcuts) {
        let Some(accelerator) = accelerator else {
            continue;
        };
        let shortcut = accelerator
            .parse::<Shortcut>()
            .map_err(|_| AppError::InvalidSetting("unknown key combination"))?;
        if seen.contains(&shortcut.id()) {
            return Err(AppError::InvalidSetting(
                "two Panopin shortcuts use the same keys",
            ));
        }
        seen.push(shortcut.id());
    }
    Ok(())
}

/// Replaces all registrations with the given shortcuts. Returns the actions whose combination
/// another app holds.
pub fn register_all(app: &AppHandle, shortcuts: &Shortcuts) -> Vec<Action> {
    let manager = app.global_shortcut();
    if let Err(err) = manager.unregister_all() {
        log::warn!("could not release previous shortcuts: {err}");
    }
    let mut actions = HashMap::new();
    let mut unavailable = Vec::new();
    for (action, accelerator) in active(shortcuts) {
        let Some(accelerator) = accelerator else {
            continue;
        };
        let registered = accelerator
            .parse::<Shortcut>()
            .map_err(|err| err.to_string())
            .and_then(|shortcut| {
                manager
                    .register(shortcut)
                    .map(|()| shortcut)
                    .map_err(|err| err.to_string())
            });
        match registered {
            Ok(shortcut) => {
                actions.insert(shortcut.id(), action);
            }
            Err(err) => {
                log::warn!("shortcut {accelerator} for {action:?} is unavailable: {err}");
                unavailable.push(action);
            }
        }
    }
    let state = app.state::<HotkeyState>();
    *state.0.lock().unwrap_or_else(PoisonError::into_inner) = Registered {
        actions,
        unavailable: unavailable.clone(),
    };
    unavailable
}

/// The shortcut recorder releases every shortcut while it listens, and brings them back after.
pub fn suspend(app: &AppHandle, suspended: bool) {
    if suspended {
        if let Err(err) = app.global_shortcut().unregister_all() {
            log::warn!("could not release shortcuts for recording: {err}");
        }
        let state = app.state::<HotkeyState>();
        state
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .actions
            .clear();
    } else {
        register_all(app, &app.state::<AppState>().settings().shortcuts);
    }
}

pub fn unavailable(app: &AppHandle) -> Vec<Action> {
    let state = app.state::<HotkeyState>();
    let registered = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    registered.unavailable.clone()
}

pub fn action_for(app: &AppHandle, shortcut: &Shortcut) -> Option<Action> {
    let state = app.state::<HotkeyState>();
    let registered = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    registered.actions.get(&shortcut.id()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcuts_parse() {
        for (_, accelerator) in active(&Shortcuts::default()) {
            if let Some(accelerator) = accelerator {
                assert!(accelerator.parse::<Shortcut>().is_ok(), "{accelerator}");
            }
        }
        assert!(validate(&Shortcuts::default()).is_ok());
    }

    #[test]
    fn disabled_shortcuts_are_skipped() {
        let shortcuts = Shortcuts {
            shelf_search: None,
            ..Shortcuts::default()
        };
        assert_eq!(active(&shortcuts)[1], (Action::OpenShelfSearch, None));
    }

    #[test]
    fn duplicates_and_nonsense_are_refused() {
        let twice = Shortcuts {
            library: Some("Super+Alt+V".into()),
            ..Shortcuts::default()
        };
        assert!(validate(&twice).is_err());
        let nonsense = Shortcuts {
            library: Some("Super+Banana".into()),
            ..Shortcuts::default()
        };
        assert!(validate(&nonsense).is_err());
    }
}
