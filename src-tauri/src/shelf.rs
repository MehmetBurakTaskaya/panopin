//! The shelf overlay window (D-004): shown on the monitor under the cursor, animated by the
//! webview, hidden only when the matching close finishes.

use std::{
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::error::Result;

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
}

impl ShelfGate {
    pub fn open(&mut self) -> u64 {
        self.generation += 1;
        self.open = true;
        self.generation
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
}

#[derive(Clone, Serialize)]
struct CloseEvent {
    generation: u64,
}

pub fn toggle(app: &AppHandle) -> Result<()> {
    if with_gate(app, |gate| gate.is_open()) {
        request_close(app)
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
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let generation = with_gate(app, ShelfGate::open);
    cover_cursor_monitor(&window)?;
    window.show()?;
    window.set_focus()?;
    app.emit_to(LABEL, "shelf:open", OpenEvent { generation, search })?;
    Ok(())
}

pub fn request_close(app: &AppHandle) -> Result<()> {
    if let Some(generation) = with_gate(app, |gate| gate.close(Instant::now())) {
        app.emit_to(LABEL, "shelf:close", CloseEvent { generation })?;
    }
    Ok(())
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
    fn reopen_guard_expires() {
        let mut gate = ShelfGate::default();
        let start = Instant::now();
        gate.open();
        gate.close(start);
        assert!(gate.closed_recently(start + Duration::from_millis(100)));
        assert!(!gate.closed_recently(start + REOPEN_GUARD));
    }
}
