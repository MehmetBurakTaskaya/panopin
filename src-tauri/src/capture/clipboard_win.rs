//! Windows clipboard listener: a message-only window on its own thread receives
//! `WM_CLIPBOARDUPDATE` (no polling). It only reads the clipboard and hands a `Snapshot` to the
//! capture worker; all decisions happen in `admission`.

use std::{cell::RefCell, path::Path, sync::mpsc::Sender, thread, time::Duration};

use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            DataExchange::{
                AddClipboardFormatListener, CloseClipboard, GetClipboardData, GetClipboardOwner,
                GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
                RegisterClipboardFormatW, RemoveClipboardFormatListener,
            },
            LibraryLoader::GetModuleHandleW,
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            Power::RegisterSuspendResumeNotification,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
            GetWindowThreadProcessId, KillTimer, RegisterClassW, SetTimer,
            DEVICE_NOTIFY_WINDOW_HANDLE, HWND_MESSAGE, MSG, PBT_APMRESUMEAUTOMATIC,
            WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE, WM_POWERBROADCAST, WM_TIMER,
            WNDCLASSW,
        },
    },
};

use super::{
    admission::{RawImage, Snapshot},
    utf16_until_nul, Event,
};
use crate::error::Result;

// Standard clipboard formats.
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIBV5: u32 = 17;
/// Added to every copy Panopin makes, so its own copies are never captured again.
pub const OWN_FORMAT: PCWSTR = w!("Panopin.OwnCopy");

/// A change whose clipboard another program keeps open (Windows' clipboard history, the WSL
/// bridge) is read again after these pauses, instead of being lost.
const RETRY_AFTER_MS: [u32; 3] = [100, 300, 1000];
const RETRY_TIMER: usize = 1;

/// The change waiting to be read again: its sequence number, the next pause, and the app that
/// was in front when it happened (exclusions follow that one, not the app in front later).
struct Retry {
    sequence: u32,
    attempt: usize,
    foreground_exe: Option<String>,
}

enum Reading {
    Read(Snapshot),
    Skipped,
    Busy,
}

thread_local! {
    static SINK: RefCell<Option<Sender<Event>>> = const { RefCell::new(None) };
    static RETRY: RefCell<Option<Retry>> = const { RefCell::new(None) };
}

pub fn spawn(tx: Sender<Event>) -> Result<()> {
    thread::Builder::new()
        .name("clipboard".into())
        .spawn(move || {
            if let Err(err) = run(tx) {
                log::error!("clipboard listener stopped: {err}");
            }
        })?;
    Ok(())
}

fn run(tx: Sender<Event>) -> windows::core::Result<()> {
    SINK.with(|sink| *sink.borrow_mut() = Some(tx));
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = w!("PanopinClipboardListener");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
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
        )?;
        AddClipboardFormatListener(hwnd)?;
        // Listener registrations can be lost across sleep; re-register on resume.
        if let Err(err) =
            RegisterSuspendResumeNotification(HANDLE(hwnd.0), DEVICE_NOTIFY_WINDOW_HANDLE)
        {
            log::warn!("no suspend/resume notifications: {err}");
        }
        log::info!("clipboard listener ready");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            // A newer change replaces one still waiting to be read again.
            unsafe {
                let _ = KillTimer(Some(hwnd), RETRY_TIMER);
            }
            RETRY.with(|retry| retry.borrow_mut().take());
            let foreground_exe = unsafe { exe_of(GetForegroundWindow()) };
            read_change(hwnd, foreground_exe, 0);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == RETRY_TIMER => {
            unsafe {
                let _ = KillTimer(Some(hwnd), RETRY_TIMER);
            }
            if let Some(retry) = RETRY.with(|retry| retry.borrow_mut().take()) {
                if unsafe { GetClipboardSequenceNumber() } == retry.sequence {
                    read_change(hwnd, retry.foreground_exe, retry.attempt);
                }
            }
            LRESULT(0)
        }
        WM_POWERBROADCAST if wparam.0 == PBT_APMRESUMEAUTOMATIC as usize => {
            unsafe {
                let _ = RemoveClipboardFormatListener(hwnd);
                if let Err(err) = AddClipboardFormatListener(hwnd) {
                    log::warn!("clipboard listener could not re-register after resume: {err}");
                }
            }
            LRESULT(1)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Reads the current change and hands it to the capture worker; a clipboard kept busy is read
/// again a little later.
fn read_change(hwnd: HWND, foreground_exe: Option<String>, attempt: usize) {
    let sequence = unsafe { GetClipboardSequenceNumber() };
    match unsafe { read_snapshot(hwnd, sequence) } {
        Reading::Read(mut snapshot) => {
            snapshot.foreground_exe = foreground_exe;
            SINK.with(|sink| {
                if let Some(tx) = sink.borrow().as_ref() {
                    let _ = tx.send(Event::Clipboard(snapshot));
                }
            });
        }
        Reading::Skipped => {}
        Reading::Busy => match RETRY_AFTER_MS.get(attempt) {
            Some(&pause) => {
                RETRY.with(|retry| {
                    *retry.borrow_mut() = Some(Retry {
                        sequence,
                        attempt: attempt + 1,
                        foreground_exe,
                    });
                });
                unsafe { SetTimer(Some(hwnd), RETRY_TIMER, pause, None) };
            }
            None => log::info!("another program kept the clipboard open; one copy was not kept"),
        },
    }
}

/// Reads one clipboard change. The sequence number is checked before and after reading; if it
/// moved (or is 0) the change is dropped, as is anything marked private (content is not read).
unsafe fn read_snapshot(hwnd: HWND, sequence: u32) -> Reading {
    if sequence == 0 {
        return Reading::Skipped;
    }
    let owner_exe = GetClipboardOwner().ok().and_then(|window| exe_of(window));
    if !open(hwnd) {
        return Reading::Busy;
    }
    let mut snapshot = read_formats();
    let _ = CloseClipboard();
    if GetClipboardSequenceNumber() != sequence {
        return Reading::Skipped;
    }
    snapshot.owner_exe = owner_exe;
    Reading::Read(snapshot)
}

/// Another process may hold the clipboard for a moment; retry a few times, briefly.
unsafe fn open(hwnd: HWND) -> bool {
    for attempt in 1..=5 {
        if OpenClipboard(Some(hwnd)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(10 * attempt));
    }
    false
}

unsafe fn read_formats() -> Snapshot {
    let available = |format: u32| IsClipboardFormatAvailable(format).is_ok();
    let marker = |name: PCWSTR| available(RegisterClipboardFormatW(name));
    let private = marker(w!("Clipboard Viewer Ignore"))
        || marker(w!("ExcludeClipboardContentFromMonitorProcessing"))
        || dword_is_zero(RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory")))
        || dword_is_zero(RegisterClipboardFormatW(w!("CanUploadToCloudClipboard")));
    let mut snapshot = Snapshot {
        private,
        own_copy: marker(OWN_FORMAT),
        has_files: available(CF_HDROP),
        ..Snapshot::default()
    };
    if snapshot.private || snapshot.own_copy || snapshot.has_files {
        return snapshot;
    }
    if available(CF_UNICODETEXT) {
        snapshot.text = global_bytes(CF_UNICODETEXT).map(|bytes| utf16_until_nul(&bytes));
    }
    // Real text wins over a rendered picture of it, so the image is not even read then.
    if snapshot
        .text
        .as_deref()
        .is_some_and(|t| !t.chars().all(char::is_whitespace))
    {
        return snapshot;
    }
    let png = RegisterClipboardFormatW(w!("PNG"));
    snapshot.image = if available(png) {
        global_bytes(png).map(RawImage::Png)
    } else if available(CF_DIBV5) {
        global_bytes(CF_DIBV5).map(RawImage::Dib)
    } else if available(CF_DIB) {
        global_bytes(CF_DIB).map(RawImage::Dib)
    } else {
        None
    };
    snapshot
}

unsafe fn global_bytes(format: u32) -> Option<Vec<u8>> {
    let handle = GetClipboardData(format).ok()?;
    let global = HGLOBAL(handle.0);
    let size = GlobalSize(global);
    let ptr = GlobalLock(global).cast::<u8>().cast_const();
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
    let _ = GlobalUnlock(global);
    Some(bytes)
}

/// `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard` hold a DWORD; 0 means private.
unsafe fn dword_is_zero(format: u32) -> bool {
    IsClipboardFormatAvailable(format).is_ok()
        && global_bytes(format).is_some_and(|b| b.len() >= 4 && b[..4] == [0, 0, 0, 0])
}

/// Executable file name of the process owning a window, e.g. `KeePassXC.exe`.
unsafe fn exe_of(window: HWND) -> Option<String> {
    if window.is_invalid() {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(window, Some(&mut pid));
    if pid == 0 {
        return None;
    }
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    let ok = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        PWSTR(buffer.as_mut_ptr()),
        &mut len,
    )
    .is_ok();
    let _ = CloseHandle(process);
    if !ok {
        return None;
    }
    let path = String::from_utf16_lossy(&buffer[..len as usize]);
    Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}
