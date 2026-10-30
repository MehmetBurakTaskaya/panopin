//! The shelf overlay window (D-004): shown on the monitor under the cursor, animated by the
//! webview, hidden only when the matching close finishes.

use std::{
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::{
    error::Result,
    focus::{self, Target},
    system_state,
};

pub const LABEL: &str = "shelf";

/// A tray click right after a focus-loss close is the same gesture, not a request to reopen.
const REOPEN_GUARD: Duration = Duration::from_millis(400);

/// Every open and close bumps the generation, so a slow close animation can never hide a
/// shelf that was reopened in the meantime.
#[derive(Debug, Default)]
pub struct ShelfGate {
    generation: u64,
    open: bool,
    closed_at: Option<Instant>,
    /// The window that was in front before the shelf opened.
    previous: Option<Target>,
    /// Losing focus closes the shelf only after it really had focus: a shelf shown by a
    /// background launch may never get it.
    focused: bool,
    /// Open dialogs of the shelf's own (the export folder picker) that have taken the focus.
    held: u32,
}

impl ShelfGate {
    pub fn open(&mut self) -> u64 {
        self.generation += 1;
        if !self.open {
            self.focused = false;
        }
        self.open = true;
        self.generation
    }

    /// Undoes an opening that failed half-way, so the next shortcut opens again.
    pub fn abort(&mut self) {
        if self.open {
            self.generation += 1;
            self.open = false;
        }
    }

    pub fn close(&mut self, now: Instant) -> Option<u64> {
        if !self.open {
            return None;
        }
        self.generation += 1;
        self.open = false;
        self.closed_at = Some(now);
        Some(self.generation)
    }

    pub fn may_hide(&self, generation: u64) -> bool {
        !self.open && generation == self.generation
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn closed_recently(&self, now: Instant) -> bool {
        self.closed_at
            .is_some_and(|at| now.saturating_duration_since(at) < REOPEN_GUARD)
    }

    /// Notes a focus change; true when losing focus should close the shelf.
    pub fn focus(&mut self, focused: bool) -> bool {
        if focused && self.open {
            self.focused = true;
        }
        !focused && self.open && self.focused && self.held == 0
    }
}

/// Focus events from a modal dialog can arrive after it has closed; the hold outlives it by this.
const HOLD_GRACE: Duration = Duration::from_millis(600);

/// Keeps the shelf open while one of its own dialogs has the focus. Dropping it lets focus
/// loss close the shelf again, a moment later.
pub struct HoldOpen(AppHandle);

impl Drop for HoldOpen {
    fn drop(&mut self) {
        let app = self.0.clone();
        std::thread::spawn(move || {
            std::thread::sleep(HOLD_GRACE);
            with_gate(&app, |gate| gate.held = gate.held.saturating_sub(1));
        });
    }
}

pub fn hold_open(app: &AppHandle) -> HoldOpen {
    with_gate(app, |gate| gate.held += 1);
    HoldOpen(app.clone())
}

#[derive(Default)]
pub struct ShelfState(Mutex<ShelfGate>);

fn with_gate<T>(app: &AppHandle, f: impl FnOnce(&mut ShelfGate) -> T) -> T {
    let state = app.state::<ShelfState>();
    let mut gate = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    f(&mut gate)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenEvent {
    generation: u64,
    search: bool,
    /// Sounds stay off while a full-screen app, presentation or focus session is active.
    muted: bool,
    /// A message key the board shows, e.g. why a paste did not happen.
    notice: Option<&'static str>,
    /// A card to put the cursor on, and the editor to open on it (a new snip: crop).
    focus: Option<i64>,
    editor: Option<&'static str>,
}

#[derive(Clone, Serialize)]
struct CloseEvent {
    generation: u64,
}

pub fn toggle(app: &AppHandle) -> Result<()> {
    if with_gate(app, |gate| gate.is_open()) {
        request_close(app, true)?;
        // The shortcut is still held: releasing its Alt must not open the menu bar of the app
        // that just got focus back.
        focus::mask_alt_release();
        Ok(())
    } else {
        open(app, false)
    }
}

pub fn toggle_from_tray(app: &AppHandle) -> Result<()> {
    if with_gate(app, |gate| {
        !gate.is_open() && gate.closed_recently(Instant::now())
    }) {
        return Ok(());
    }
    toggle(app)
}

pub fn open(app: &AppHandle, search: bool) -> Result<()> {
    show(app, search, None, None)
}

/// Opens the shelf on one card with its editor; used for a new snip (D-062). No shortcut gave
/// Panopin the right to come forward, so it claims it first.
pub fn open_on(app: &AppHandle, id: i64, editor: &'static str) -> Result<()> {
    focus::claim_foreground();
    show(app, false, None, Some((id, editor)))
}

/// Opens the shelf with a message on the board, e.g. after a refused paste.
// Only the Windows paste path refuses after closing so far; macOS joins in Phase 8.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn open_with_notice(app: &AppHandle, notice: &'static str) -> Result<()> {
    show(app, false, Some(notice), None)
}

fn show(
    app: &AppHandle,
    search: bool,
    notice: Option<&'static str>,
    on: Option<(i64, &'static str)>,
) -> Result<()> {
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    // Recorded before anything is shown; the shelf itself never counts as the previous window.
    let own = own_hwnd(&window);
    let previous = focus::foreground().filter(|target| Some(target.hwnd) != own);
    let muted = system_state::wants_quiet();
    // An Alt still held from the shortcut must not open the menu bar of the app left behind.
    focus::mask_alt_release();
    let generation = with_gate(app, |gate| {
        if previous.is_some() {
            gate.previous = previous;
        }
        gate.open()
    });
    let shown = cover_cursor_monitor(&window)
        .and_then(|()| Ok(window.show()?))
        .and_then(|()| Ok(window.set_focus()?));
    if let Err(err) = shown {
        with_gate(app, ShelfGate::abort);
        let _ = window.hide();
        return Err(err);
    }
    // The focus event follows later; a shelf that is already in front counts as focused now.
    if window.is_focused().unwrap_or(false) {
        with_gate(app, |gate| gate.focused = true);
    }
    let event = OpenEvent {
        generation,
        search,
        muted,
        notice,
        focus: on.map(|(id, _)| id),
        editor: on.map(|(_, editor)| editor),
    };
    app.emit_to(LABEL, "shelf:open", event)?;
    Ok(())
}

#[cfg(windows)]
fn own_hwnd(window: &WebviewWindow) -> Option<isize> {
    window.hwnd().ok().map(|hwnd| hwnd.0 as isize)
}

#[cfg(not(windows))]
fn own_hwnd(_window: &WebviewWindow) -> Option<isize> {
    None
}

/// The overlay on macOS (D-073). Since macOS 14 an app in the background may only ask to come
/// forward, so a shelf opened by its shortcut would show without the keyboard. The window becomes
/// a non-activating panel instead: it takes the keyboard while the app in front stays active,
/// which is where Enter and Cmd+Enter return anyway. It also opens over full-screen apps and stays
/// out of Cmd+`; Tauri only lets it join every Space. Runs once in setup, on the main thread.
#[cfg(target_os = "macos")]
pub fn prepare_overlay(app: &AppHandle) {
    use objc2::{runtime::AnyObject, ClassType};
    use objc2_app_kit::{NSPanel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
    let Some(Ok(raw)) = app
        .get_webview_window(LABEL)
        .map(|window| window.ns_window())
    else {
        return;
    };
    // SAFETY: Tauri hands out the live NSWindow of the shelf, and setup runs on the main thread.
    let Some(ns_window) = (unsafe { raw.cast::<NSWindow>().as_ref() }) else {
        return;
    };
    ns_window.setCollectionBehavior(
        ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    // The window keeps Tauri's own state right after the NSWindow fields, so it can only take
    // the panel class while a panel adds no fields of its own.
    if NSPanel::class().instance_size() != NSWindow::class().instance_size() {
        log::warn!("the shelf stays a regular window: panels have fields of their own here");
        return;
    }
    // SAFETY: on the main thread; ShelfPanel is an NSPanel without fields, as large as the window
    // it replaces, and it answers the key and main window questions as Tauri's class did.
    unsafe { AnyObject::set_class(ns_window.as_ref(), mac_panel::ShelfPanel::class()) };
    ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
    // Panels hide when their app deactivates; the shelf belongs to the app in front instead.
    ns_window.setHidesOnDeactivate(false);
}

#[cfg(target_os = "macos")]
mod mac_panel {
    use objc2::{define_class, runtime::NSObject, MainThreadOnly};
    use objc2_app_kit::{NSPanel, NSResponder, NSWindow};

    define_class!(
        /// The shelf's class on macOS: a panel that takes the keyboard without activating Panopin.
        #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "PanopinShelfPanel"]
        pub(super) struct ShelfPanel;

        impl ShelfPanel {
            #[unsafe(method(canBecomeKeyWindow))]
            fn can_become_key_window(&self) -> bool {
                true
            }

            #[unsafe(method(canBecomeMainWindow))]
            fn can_become_main_window(&self) -> bool {
                false
            }
        }
    );
}

/// Starts the slide-out. `restore` hands focus back to the window that was in front before:
/// right for Esc, Enter and the shortcut, wrong when the user clicked another window.
pub fn request_close(app: &AppHandle, restore: bool) -> Result<()> {
    let closed = with_gate(app, |gate| {
        gate.close(Instant::now())
            .map(|generation| (generation, gate.previous))
    });
    if let Some((generation, previous)) = closed {
        if let (true, Some(target)) = (restore, previous) {
            focus::restore(target);
        }
        app.emit_to(LABEL, "shelf:close", CloseEvent { generation })?;
    }
    Ok(())
}

pub fn previous_window(app: &AppHandle) -> Option<Target> {
    with_gate(app, |gate| gate.previous)
}

/// Clicking anywhere outside Panopin closes the shelf, once it has had focus.
pub fn focus_changed(app: &AppHandle, focused: bool) -> Result<()> {
    if with_gate(app, |gate| gate.focus(focused)) {
        request_close(app, false)
    } else {
        Ok(())
    }
}

/// Called by the webview once its slide-out animation has finished.
pub fn finish_close(app: &AppHandle, generation: u64) -> Result<()> {
    if with_gate(app, |gate| gate.may_hide(generation)) {
        if let Some(window) = app.get_webview_window(LABEL) {
            window.hide()?;
        }
    }
    Ok(())
}

/// Sizes the overlay to the work area of the monitor under the cursor (physical pixels, so
/// per-monitor DPI is respected).
fn cover_cursor_monitor(window: &WebviewWindow) -> Result<()> {
    let cursor = window.cursor_position()?;
    let monitor = match window.monitor_from_point(cursor.x, cursor.y)? {
        Some(monitor) => Some(monitor),
        None => window.primary_monitor()?,
    };
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        window.set_position(area.position)?;
        window.set_size(area.size)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_after_open_may_hide() {
        let mut gate = ShelfGate::default();
        gate.open();
        let generation = gate.close(Instant::now()).unwrap();
        assert!(gate.may_hide(generation));
    }

    #[test]
    fn stale_close_never_hides_a_reopened_shelf() {
        let mut gate = ShelfGate::default();
        gate.open();
        let stale = gate.close(Instant::now()).unwrap();
        gate.open();
        assert!(!gate.may_hide(stale));
        let current = gate.close(Instant::now()).unwrap();
        assert!(!gate.may_hide(stale));
        assert!(gate.may_hide(current));
    }

    #[test]
    fn closing_twice_is_a_no_op() {
        let mut gate = ShelfGate::default();
        assert_eq!(gate.close(Instant::now()), None);
        gate.open();
        assert!(gate.close(Instant::now()).is_some());
        assert_eq!(gate.close(Instant::now()), None);
    }

    #[test]
    fn a_failed_opening_is_undone() {
        let mut gate = ShelfGate::default();
        let generation = gate.open();
        gate.abort();
        assert!(!gate.is_open());
        assert!(gate.close(Instant::now()).is_none());
        assert!(gate.open() > generation);
    }

    #[test]
    fn reopen_guard_expires() {
        let mut gate = ShelfGate::default();
        let start = Instant::now();
        gate.open();
        gate.close(start);
        assert!(gate.closed_recently(start + Duration::from_millis(100)));
        assert!(!gate.closed_recently(start + REOPEN_GUARD));
    }

    #[test]
    fn focus_loss_closes_only_after_focus_and_not_during_own_dialogs() {
        let mut gate = ShelfGate::default();
        gate.open();
        // Shown without ever getting focus: losing it means nothing.
        assert!(!gate.focus(false));
        assert!(!gate.focus(true));
        gate.held = 1;
        assert!(!gate.focus(false));
        gate.held = 0;
        assert!(gate.focus(false));
    }
}
