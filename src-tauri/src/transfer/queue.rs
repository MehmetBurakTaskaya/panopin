//! Paste one by one (D-084): several items go on the clipboard in turn, the next one after each
//! paste. The platform side (`queue_win`, `queue_mac`) sees a paste because the system asks
//! Panopin for the data only then; this module keeps the order and tells the windows.

use std::{
    collections::VecDeque,
    sync::{Mutex, MutexGuard, OnceLock},
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::Formats;

/// How long after an app took an item the next one replaces it. A paste reads all its formats
/// well within this time.
pub const SETTLE: Duration = Duration::from_millis(300);

/// What the next paste gives: item `next` of `total`, counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct QueueStatus {
    pub next: usize,
    pub total: usize,
}

/// The items of one queue and how far it got.
#[derive(Debug)]
pub struct Turns {
    current: Formats,
    rest: VecDeque<Formats>,
    total: usize,
    /// An app has taken the current item at least once.
    taken: bool,
    /// The last item went out: it stays on the clipboard, and nothing follows it.
    done: bool,
}

impl Turns {
    /// A queue of the items that have something to paste; `None` when none has.
    pub fn new(items: Vec<Formats>) -> Option<Self> {
        let mut rest: VecDeque<_> = items.into_iter().filter(|f| !f.is_empty()).collect();
        let total = rest.len();
        let current = rest.pop_front()?;
        Some(Self {
            current,
            rest,
            total,
            taken: false,
            done: false,
        })
    }

    pub fn current(&self) -> &Formats {
        &self.current
    }

    /// An app asked for the current item's data.
    pub fn take(&mut self) {
        self.taken = true;
    }

    /// Whether the queue still waits for a paste to move on.
    pub fn running(&self) -> bool {
        !self.done
    }

    /// After a paste: the next item becomes current (`true`), or the last one stays and the queue
    /// is done. Nothing moves while the current item was not taken.
    pub fn advance(&mut self) -> bool {
        if !self.taken || self.done {
            return false;
        }
        match self.rest.pop_front() {
            Some(next) => {
                self.current = next;
                self.taken = false;
                true
            }
            None => {
                self.done = true;
                false
            }
        }
    }

    pub fn status(&self) -> Option<QueueStatus> {
        (!self.done).then(|| QueueStatus {
            next: self.total - self.rest.len(),
            total: self.total,
        })
    }
}

static STATUS: Mutex<Option<QueueStatus>> = Mutex::new(None);
static APP: OnceLock<AppHandle> = OnceLock::new();

/// Remembers the app, so that the platform side can tell the windows and use its main thread.
pub fn attach(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

#[cfg(target_os = "macos")]
pub fn app() -> Option<&'static AppHandle> {
    APP.get()
}

fn status_lock() -> MutexGuard<'static, Option<QueueStatus>> {
    STATUS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Where the running queue is; `None` when none runs.
pub fn status() -> Option<QueueStatus> {
    *status_lock()
}

pub fn is_running() -> bool {
    status().is_some()
}

/// Records the queue's state and tells every window (`queue:changed`).
pub fn publish(status: Option<QueueStatus>) {
    {
        let mut current = status_lock();
        if *current == status {
            return;
        }
        *current = status;
    }
    if let Some(app) = APP.get() {
        if let Err(err) = app.emit("queue:changed", status) {
            log::warn!("the queue's state could not be sent: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(body: &str) -> Formats {
        Formats {
            text: Some(super::super::utf16z(body)),
            ..Formats::default()
        }
    }

    fn body(turns: &Turns) -> String {
        let units = turns.current().text.as_deref().unwrap_or_default();
        String::from_utf16_lossy(units.strip_suffix(&[0]).unwrap_or(units))
    }

    #[test]
    fn hands_out_the_items_in_turn() {
        let mut turns =
            Turns::new(vec![text("a"), Formats::default(), text("b"), text("c")]).unwrap();
        assert_eq!(
            turns.status(),
            Some(QueueStatus { next: 1, total: 3 }),
            "empty items are left out"
        );
        assert_eq!(body(&turns), "a");
        assert!(!turns.advance(), "nothing moves before a paste");
        turns.take();
        turns.take();
        assert!(turns.advance());
        assert_eq!(body(&turns), "b");
        assert_eq!(turns.status(), Some(QueueStatus { next: 2, total: 3 }));
        assert!(!turns.advance(), "b was not pasted yet");
        turns.take();
        assert!(turns.advance());
        assert_eq!(body(&turns), "c");
        turns.take();
        assert!(!turns.advance(), "c is the last");
        assert_eq!(body(&turns), "c", "the last item stays");
        assert!(!turns.running());
        assert_eq!(turns.status(), None);
        turns.take();
        assert!(
            !turns.advance(),
            "pasting the last item again moves nothing"
        );
    }

    #[test]
    fn a_queue_needs_something_to_paste() {
        assert!(Turns::new(Vec::new()).is_none());
        assert!(Turns::new(vec![Formats::default()]).is_none());
    }
}
