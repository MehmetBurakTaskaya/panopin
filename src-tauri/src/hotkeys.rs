//! Global shortcuts (D-020). A combination that another app already holds is reported and
//! skipped, never taken over.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::settings::Shortcuts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ToggleShelf,
    OpenShelfSearch,
}

/// Registered shortcut ids mapped to their actions.
#[derive(Default)]
pub struct HotkeyState(Mutex<HashMap<u32, Action>>);

/// Shortcuts whose feature exists in this build. The others (snip, library, ...) are
/// registered by the phase that implements them.
fn active(shortcuts: &Shortcuts) -> [(Action, Option<&str>); 2] {
    [
        (Action::ToggleShelf, shortcuts.shelf.as_deref()),
        (Action::OpenShelfSearch, shortcuts.shelf_search.as_deref()),
    ]
}

/// Replaces all registrations with the given shortcuts. Returns the ones that failed.
pub fn register_all(app: &AppHandle, shortcuts: &Shortcuts) -> Vec<String> {
    let manager = app.global_shortcut();
    if let Err(err) = manager.unregister_all() {
        log::warn!("could not release previous shortcuts: {err}");
    }
    let mut actions = HashMap::new();
    let mut failed = Vec::new();
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
                failed.push(accelerator.to_owned());
            }
        }
    }
    let state = app.state::<HotkeyState>();
    *state.0.lock().unwrap_or_else(PoisonError::into_inner) = actions;
    failed
}

pub fn action_for(app: &AppHandle, shortcut: &Shortcut) -> Option<Action> {
    let state = app.state::<HotkeyState>();
    let actions = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    actions.get(&shortcut.id()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcuts_parse() {
        for (_, accelerator) in active(&Shortcuts::default()) {
            let accelerator = accelerator.unwrap();
            assert!(accelerator.parse::<Shortcut>().is_ok(), "{accelerator}");
        }
    }

    #[test]
    fn disabled_shortcuts_are_skipped() {
        let shortcuts = Shortcuts {
            shelf_search: None,
            ..Shortcuts::default()
        };
        assert_eq!(active(&shortcuts)[1], (Action::OpenShelfSearch, None));
    }
}
