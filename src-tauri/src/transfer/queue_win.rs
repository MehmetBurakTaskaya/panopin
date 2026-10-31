//! Paste one by one on Windows (D-084). A hidden window of Panopin owns the clipboard and puts the
//! current item there with delayed rendering: the formats are named, the data is held back.
//! Windows asks for the data (`WM_RENDERFORMAT`) only when an app pastes; shortly after, the next
//! item takes its place. No keys are watched, so a menu or Shift+Insert paste counts too.

use std::{
    cell::{Cell, RefCell},
    sync::{mpsc, Mutex, MutexGuard},
    thread,
};

use tauri::AppHandle;
use windows::{
    core::{w, BOOL},
    Win32::{
        Foundation::{SetLastError, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardOwner, GetOpenClipboardWindow,
                OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
            },
            LibraryLoader::GetModuleHandleW,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, EnumChildWindows,
            GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, KillTimer, RegisterClassW,
            SendMessageW, SetTimer, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
            WM_DESTROYCLIPBOARD, WM_RENDERALLFORMATS, WM_RENDERFORMAT, WM_TIMER, WNDCLASSW,
        },
    },
};

use super::{
    clipboard_win::{open, put, CF_DIB, CF_HDROP, CF_UNICODETEXT},
    queue::{self, Turns, SETTLE},
    Formats,
};
use crate::{
    capture::clipboard_win::OWN_FORMAT,
    error::{AppError, Result},
};

const WM_START: u32 = WM_APP + 1;
/// Panopin quits: the item waiting is written in full, so the clipboard keeps it.
const WM_FINISH: u32 = WM_APP + 2;
const ADVANCE: usize = 1;

/// The queue window, made on first use.
static WINDOW: Mutex<Option<isize>> = Mutex::new(None);
/// The items of a queue on their way to the window.
static INBOX: Mutex<Option<Vec<Formats>>> = Mutex::new(None);

thread_local! {
    static TURNS: RefCell<Option<Turns>> = const { RefCell::new(None) };
    /// Set while the window empties the clipboard itself, which Windows reports to its owner too.
    static REPLACING: Cell<bool> = const { Cell::new(false) };
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Puts the first item on the clipboard; the others follow one paste at a time.
pub fn start(app: &AppHandle, items: Vec<Formats>) -> Result<()> {
    queue::attach(app);
    start_items(items)
}

fn start_items(items: Vec<Formats>) -> Result<()> {
    let hwnd = window()?;
    *lock(&INBOX) = Some(items);
    // SAFETY: the window lives as long as Panopin; the message is handled on its own thread.
    let started = unsafe { SendMessageW(HWND(hwnd as *mut _), WM_START, None, None) };
    if started.0 == 0 {
        return Err(AppError::ClipboardBusy);
    }
    Ok(())
}

/// Called as Panopin quits.
pub fn finish() {
    // The lock is let go first: the window's thread may need it while this message waits.
    let window = *lock(&WINDOW);
    if let Some(hwnd) = window {
        // SAFETY: as in `start`.
        unsafe { SendMessageW(HWND(hwnd as *mut _), WM_FINISH, None, None) };
    }
}

fn window() -> Result<isize> {
    let mut slot = lock(&WINDOW);
    if let Some(hwnd) = *slot {
        return Ok(hwnd);
    }
    let (ready, made) = mpsc::channel();
    thread::Builder::new()
        .name("paste-queue".into())
        .spawn(move || run(&ready))?;
    let hwnd = made.recv().map_err(|_| AppError::Interrupted)??;
    *slot = Some(hwnd);
    Ok(hwnd)
}

fn run(ready: &mpsc::Sender<Result<isize>>) {
    // SAFETY: a message-only window and its message loop, all on this thread.
    unsafe {
        let made = (|| -> windows::core::Result<HWND> {
            let instance = GetModuleHandleW(None)?;
            let class = w!("PanopinPasteQueue");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                w!(""),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
        })();
        match made {
            Ok(hwnd) => {
                let _ = ready.send(Ok(hwnd.0 as isize));
            }
            Err(err) => {
                let _ = ready.send(Err(err.into()));
                return;
            }
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_START => LRESULT(isize::from(begin(hwnd))),
        WM_RENDERFORMAT => {
            render(hwnd, wparam.0 as u32);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == ADVANCE => {
            // SAFETY: the timer belongs to this window.
            unsafe {
                let _ = KillTimer(Some(hwnd), ADVANCE);
            }
            advance(hwnd);
            LRESULT(0)
        }
        WM_DESTROYCLIPBOARD => {
            // Something else was copied: the queue is over.
            if !REPLACING.get() && TURNS.with(|turns| turns.borrow_mut().take()).is_some() {
                announce_status(None);
            }
            LRESULT(0)
        }
        WM_RENDERALLFORMATS | WM_FINISH => {
            render_all(hwnd);
            LRESULT(0)
        }
        // SAFETY: the default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn begin(hwnd: HWND) -> bool {
    let Some(turns) = lock(&INBOX).take().and_then(Turns::new) else {
        return false;
    };
    // SAFETY: the clipboard is opened and closed by this window's thread.
    if let Err(err) = unsafe { offer(hwnd, turns.current()) } {
        log::warn!("a queue could not start: {err}");
        // A queue that was running may have lost its item on the way.
        if TURNS.with(|slot| slot.borrow_mut().take()).is_some() {
            announce_status(None);
        }
        return false;
    }
    let status = turns.status();
    TURNS.with(|slot| *slot.borrow_mut() = Some(turns));
    announce_status(status);
    true
}

/// An app pastes: it gets the current item's data in this format, and the next item follows
/// once the paste has settled.
fn render(hwnd: HWND, format: u32) {
    if !asked_from_the_front() {
        log::debug!("a queued item was asked for from the background and held back");
        return;
    }
    let rendered = TURNS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let turns = slot.as_mut()?;
        if let Some(bytes) = bytes_for(turns.current(), format) {
            // SAFETY: during WM_RENDERFORMAT the owner sets the data without opening the clipboard.
            if let Err(err) = unsafe { put(format, &bytes) } {
                log::warn!("a queued item could not be handed out: {err}");
            }
        }
        turns.take();
        Some(turns.running())
    });
    if rendered == Some(true) {
        // SAFETY: a timer on this thread's own window; a new paste starts it again.
        unsafe { SetTimer(Some(hwnd), ADVANCE, SETTLE.as_millis() as u32, None) };
    }
}

/// A paste comes from the app in front. Anything else that asks, such as the remote desktop client
/// that hands the clipboard on to WSL's Linux apps as soon as it changes, gets nothing yet: the
/// data stays held back, so Windows asks Panopin again when the paste comes.
fn asked_from_the_front() -> bool {
    // SAFETY: window and process queries with no preconditions.
    unsafe {
        // Opened without a window, the reader cannot be told apart, so it counts as a paste.
        let Ok(reader) = GetOpenClipboardWindow() else {
            return true;
        };
        // With no window in front, nobody is pasting.
        let front = GetForegroundWindow();
        if front.is_invalid() {
            return false;
        }
        let reader = process_of(reader);
        if reader == 0 || reader == process_of(front) {
            return true;
        }
        // A Store app draws inside a frame window that another process owns.
        let mut found = (reader, false);
        let _ = EnumChildWindows(
            Some(front),
            Some(child_of),
            LPARAM(&mut found as *mut (u32, bool) as isize),
        );
        found.1
    }
}

unsafe fn process_of(window: HWND) -> u32 {
    let mut pid = 0u32;
    GetWindowThreadProcessId(window, Some(&mut pid));
    pid
}

unsafe extern "system" fn child_of(child: HWND, lparam: LPARAM) -> BOOL {
    let found = &mut *(lparam.0 as *mut (u32, bool));
    found.1 = process_of(child) == found.0;
    BOOL::from(!found.1)
}

fn advance(hwnd: HWND) {
    let status = TURNS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let turns = slot.as_mut()?;
        if turns.advance() {
            // SAFETY: as in `begin`.
            if let Err(err) = unsafe { offer(hwnd, turns.current()) } {
                log::warn!("the next queued item could not go on the clipboard: {err}");
                *slot = None;
                return Some(None);
            }
        }
        Some(turns.status())
    });
    if let Some(status) = status {
        announce_status(status);
    }
}

/// Names the item's formats on the clipboard without their data, with the marks that keep
/// clipboard history and other clipboard tools from reading it before a paste does.
unsafe fn offer(hwnd: HWND, formats: &Formats) -> Result<()> {
    open(hwnd)?;
    REPLACING.set(true);
    let emptied = EmptyClipboard();
    REPLACING.set(false);
    let result = emptied.map_err(AppError::from).and_then(|()| {
        for format in delayed(formats) {
            // Delayed rendering returns no handle, so success shows as an error without a code.
            SetLastError(WIN32_ERROR(0));
            if let Err(err) = SetClipboardData(format, None) {
                if err.code().is_err() {
                    return Err(err.into());
                }
            }
        }
        put(RegisterClipboardFormatW(OWN_FORMAT), &1u32.to_le_bytes())?;
        put(
            RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
            &[0],
        )?;
        put(
            RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")),
            &0u32.to_le_bytes(),
        )?;
        put(
            RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")),
            &0u32.to_le_bytes(),
        )
    });
    let _ = CloseClipboard();
    result
}

/// Panopin quits, or the window goes: whatever the clipboard still lacks of the item is written.
fn render_all(hwnd: HWND) {
    TURNS.with(|slot| {
        let slot = slot.borrow();
        let Some(turns) = slot.as_ref() else {
            return;
        };
        // SAFETY: the owner opens the clipboard to render the rest, as WM_RENDERALLFORMATS asks.
        unsafe {
            if OpenClipboard(Some(hwnd)).is_err() {
                return;
            }
            if GetClipboardOwner().ok() == Some(hwnd) {
                for format in delayed(turns.current()) {
                    if let Some(bytes) = bytes_for(turns.current(), format) {
                        let _ = put(format, &bytes);
                    }
                }
            }
            let _ = CloseClipboard();
        }
    });
}

fn delayed(formats: &Formats) -> Vec<u32> {
    let mut list = Vec::new();
    if formats.text.is_some() {
        list.push(CF_UNICODETEXT);
    }
    if formats.png.is_some() {
        // SAFETY: registering a format name has no preconditions.
        list.push(unsafe { RegisterClipboardFormatW(w!("PNG")) });
    }
    if formats.dib.is_some() {
        list.push(CF_DIB);
    }
    if formats.files.is_some() {
        list.push(CF_HDROP);
    }
    list
}

fn bytes_for(formats: &Formats, format: u32) -> Option<Vec<u8>> {
    match format {
        CF_UNICODETEXT => formats
            .text
            .as_ref()
            .map(|text| text.iter().flat_map(|unit| unit.to_le_bytes()).collect()),
        CF_DIB => formats.dib.clone(),
        CF_HDROP => formats.files.clone(),
        // SAFETY: as in `delayed`.
        _ if format == unsafe { RegisterClipboardFormatW(w!("PNG")) } => formats.png.clone(),
        _ => None,
    }
}

fn announce_status(status: Option<queue::QueueStatus>) {
    queue::publish(status);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::HGLOBAL,
            System::{
                DataExchange::{GetClipboardData, IsClipboardFormatAvailable},
                Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            },
        },
    };

    use super::*;
    use crate::transfer::{clipboard_win, queue::QueueStatus, utf16z};

    fn text(body: &str) -> Formats {
        Formats {
            text: Some(utf16z(body)),
            ..Formats::default()
        }
    }

    /// What an app pasting text gets.
    fn paste() -> String {
        // SAFETY: the clipboard is opened and closed here, and the block is only read while locked.
        unsafe {
            open(HWND::default()).unwrap();
            let handle = GetClipboardData(CF_UNICODETEXT).unwrap();
            let global = HGLOBAL(handle.0);
            let len = GlobalSize(global) / 2;
            let units = std::slice::from_raw_parts(GlobalLock(global).cast::<u16>(), len).to_vec();
            let _ = GlobalUnlock(global);
            let _ = CloseClipboard();
            let end = units.iter().position(|&unit| unit == 0).unwrap_or(len);
            String::from_utf16_lossy(&units[..end])
        }
    }

    fn marked(name: PCWSTR) -> bool {
        // SAFETY: asking for a format's presence needs no open clipboard.
        unsafe { IsClipboardFormatAvailable(RegisterClipboardFormatW(name)).is_ok() }
    }

    #[test]
    #[ignore = "takes over the clipboard; CI runs it on its own"]
    fn each_paste_takes_the_next_item() {
        let settled = SETTLE + Duration::from_millis(300);
        // SAFETY: a message-only window of a system class, never in front: it reads the clipboard
        // the way a program in the background does, and later copies something else.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .unwrap();

        start_items(vec![text("one"), text("two"), text("three")]).unwrap();
        assert_eq!(queue::status(), Some(QueueStatus { next: 1, total: 3 }));
        assert!(marked(OWN_FORMAT), "Panopin's own copy");
        assert!(
            marked(w!("ExcludeClipboardContentFromMonitorProcessing")),
            "not for clipboard history and other tools"
        );
        // A program in the background asking at once, as the bridge to WSL's apps does, gets
        // nothing and moves nothing.
        // SAFETY: the clipboard is opened and closed here.
        let held = unsafe {
            OpenClipboard(Some(window)).unwrap();
            let data = GetClipboardData(CF_UNICODETEXT);
            let _ = CloseClipboard();
            data
        };
        assert!(held.is_err(), "held back from the background");
        thread::sleep(settled);
        assert_eq!(queue::status(), Some(QueueStatus { next: 1, total: 3 }));

        assert_eq!(paste(), "one");
        assert_eq!(paste(), "one", "a paste may read the text twice");
        thread::sleep(settled);
        assert_eq!(queue::status(), Some(QueueStatus { next: 2, total: 3 }));
        assert_eq!(paste(), "two");
        thread::sleep(settled);
        assert_eq!(paste(), "three");
        thread::sleep(settled);
        assert_eq!(queue::status(), None, "the queue is done");
        assert_eq!(paste(), "three", "the last item stays");

        // A copy made elsewhere ends a queue at once.
        start_items(vec![text("a"), text("b")]).unwrap();
        assert!(queue::is_running());
        clipboard_win::write(window.0 as isize, &text("elsewhere")).unwrap();
        assert_eq!(queue::status(), None);
        assert_eq!(paste(), "elsewhere");
    }
}
