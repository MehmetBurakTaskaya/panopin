//! Edge opening (D-022): resting the pointer on the shelf's screen edge for 300 ms opens it. A
//! low-level mouse hook exists only while the (default-off) setting is on, on a thread of its own
//! that sleeps in its message loop; full-screen apps, presentations and focus sessions suppress it.
#![cfg_attr(not(windows), allow(dead_code))]

use std::sync::{Mutex, PoisonError};

use tauri::AppHandle;

use crate::settings::ShelfEdge;

/// The running hook thread, if the setting is on.
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

struct Running {
    thread_id: u32,
    handle: std::thread::JoinHandle<()>,
}

/// Starts, restarts or stops the watcher to match the settings.
pub fn configure(app: &AppHandle, enabled: bool, edge: ShelfEdge) {
    let mut running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(old) = running.take() {
        imp::stop(old.thread_id);
        let _ = old.handle.join();
    }
    if enabled {
        *running = imp::start(app.clone(), edge);
    }
}

#[cfg(windows)]
mod imp {
    use std::{
        cell::Cell,
        sync::{mpsc, OnceLock},
    };

    use tauri::AppHandle;
    use windows::Win32::{
        Foundation::{LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST},
        System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, KillTimer,
            PostThreadMessageW, SetTimer, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
            MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_MOUSEMOVE, WM_QUIT, WM_TIMER,
        },
    };

    use super::Running;
    use crate::settings::ShelfEdge;

    const DWELL_MS: u32 = 300;

    static APP: OnceLock<AppHandle> = OnceLock::new();

    thread_local! {
        static TOP: Cell<bool> = const { Cell::new(true) };
        /// The dwell timer that is running, if any.
        static TIMER: Cell<usize> = const { Cell::new(0) };
        /// The shelf opened for this visit; it waits until the pointer has left the edge.
        static FIRED: Cell<bool> = const { Cell::new(false) };
    }

    fn at_edge(point: POINT, top: bool) -> bool {
        // SAFETY: plain queries with a stack-allocated info block of the right size.
        unsafe {
            let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return false;
            }
            let rect = info.rcMonitor;
            if top {
                point.y <= rect.top + 1
            } else {
                point.y >= rect.bottom - 2
            }
        }
    }

    unsafe extern "system" fn on_mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && wparam.0 as u32 == WM_MOUSEMOVE {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let top = TOP.with(Cell::get);
            if at_edge(info.pt, top) {
                if TIMER.with(Cell::get) == 0 && !FIRED.with(Cell::get) {
                    TIMER.with(|t| t.set(SetTimer(None, 0, DWELL_MS, None)));
                }
            } else {
                let timer = TIMER.with(|t| t.replace(0));
                if timer != 0 {
                    let _ = KillTimer(None, timer);
                }
                FIRED.with(|f| f.set(false));
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    fn dwell_done() {
        let timer = TIMER.with(|t| t.replace(0));
        if timer != 0 {
            // SAFETY: the timer belongs to this thread.
            let _ = unsafe { KillTimer(None, timer) };
        }
        let mut point = POINT::default();
        // SAFETY: writes into a stack value.
        let still =
            unsafe { GetCursorPos(&mut point) }.is_ok() && at_edge(point, TOP.with(Cell::get));
        if !still || crate::system_state::wants_quiet() {
            return;
        }
        FIRED.with(|f| f.set(true));
        if let Some(app) = APP.get() {
            crate::focus::claim_foreground();
            if let Err(err) = crate::shelf::open(app, false) {
                log::warn!("edge opening failed: {err}");
            }
        }
    }

    pub(super) fn start(app: AppHandle, edge: ShelfEdge) -> Option<Running> {
        let _ = APP.set(app);
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("edge".into())
            .spawn(move || {
                TOP.with(|t| t.set(edge == ShelfEdge::Top));
                // SAFETY: the hook procedure lives as long as the program; the hook is removed
                // before the thread ends. The message loop only reads messages of this thread.
                unsafe {
                    let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(on_mouse), None, 0) {
                        Ok(hook) => hook,
                        Err(err) => {
                            log::warn!("edge opening is unavailable: {err}");
                            let _ = tx.send(None);
                            return;
                        }
                    };
                    let _ = tx.send(Some(GetCurrentThreadId()));
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        if msg.message == WM_TIMER && msg.hwnd.is_invalid() {
                            dwell_done();
                            continue;
                        }
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    let _ = UnhookWindowsHookEx(hook);
                }
            })
            .ok()?;
        match rx.recv() {
            Ok(Some(thread_id)) => Some(Running { thread_id, handle }),
            _ => {
                let _ = handle.join();
                None
            }
        }
    }

    pub(super) fn stop(thread_id: u32) {
        // SAFETY: posting WM_QUIT ends that thread's message loop.
        let _ = unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
    }
}

#[cfg(not(windows))]
mod imp {
    use tauri::AppHandle;

    use super::Running;
    use crate::settings::ShelfEdge;

    pub(super) fn start(_app: AppHandle, _edge: ShelfEdge) -> Option<Running> {
        None
    }

    pub(super) fn stop(_thread_id: u32) {}
}
