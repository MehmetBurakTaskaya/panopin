//! The window that was in front before the shelf opened. Enter and Esc hand focus back to it and
//! Ctrl+Enter pastes into it (ARCHITECTURE §8).
#![cfg_attr(not(windows), allow(dead_code))]

/// A top-level window and the process that owned it when it was recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub hwnd: isize,
    pub pid: u32,
}

#[cfg(windows)]
mod imp {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use windows::Win32::{
        Foundation::HWND,
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_MENU,
            },
            WindowsAndMessaging::{
                GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
            },
        },
    };

    use super::Target;

    /// An unassigned virtual key: pressing it while Alt is held keeps the Alt release from
    /// opening the menu bar of the window that receives it.
    pub const MASK_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0xE8);

    fn hwnd(target: Target) -> HWND {
        HWND(target.hwnd as *mut _)
    }

    pub fn foreground() -> Option<Target> {
        // SAFETY: plain Win32 queries without pointers we own.
        unsafe {
            let window = GetForegroundWindow();
            if window.is_invalid() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(window, Some(&mut pid));
            Some(Target {
                hwnd: window.0 as isize,
                pid,
            })
        }
    }

    /// The window still exists and still belongs to the same process (handles are reused).
    pub fn is_alive(target: Target) -> bool {
        // SAFETY: IsWindow and GetWindowThreadProcessId accept any handle value.
        unsafe {
            if !IsWindow(Some(hwnd(target))).as_bool() {
                return false;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd(target), Some(&mut pid));
            pid == target.pid
        }
    }

    pub fn restore(target: Target) -> bool {
        // SAFETY: SetForegroundWindow accepts any handle; it fails for invalid ones.
        is_alive(target) && unsafe { SetForegroundWindow(hwnd(target)).as_bool() }
    }

    /// Waits until the target is really in front; activation finishes asynchronously.
    pub fn wait_until_foreground(target: Target, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            if foreground().is_some_and(|now| now.hwnd == target.hwnd) {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn alt_is_down() -> bool {
        // SAFETY: reads the asynchronous key state only.
        unsafe { GetAsyncKeyState(i32::from(VK_MENU.0)) < 0 }
    }

    pub fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    /// Sends the neutral mask key when Alt is held, so releasing Alt does not open the menu bar
    /// of the window that has focus.
    pub fn mask_alt_release() {
        if alt_is_down() {
            send(&[key(MASK_KEY, false), key(MASK_KEY, true)]);
        }
    }

    /// Lets Panopin bring a window forward when no shortcut or click gave it that right: the
    /// process that sent the last input may take the foreground, so a neutral key is sent first.
    pub fn claim_foreground() {
        send(&[key(MASK_KEY, false), key(MASK_KEY, true)]);
    }

    /// Injects keystrokes; returns false when Windows dropped them.
    pub fn send(inputs: &[INPUT]) -> bool {
        // SAFETY: the slice holds fully initialised INPUT structures of the declared size.
        let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
        sent as usize == inputs.len()
    }
}

#[cfg(windows)]
pub use imp::*;

/// macOS (D-073): apps rather than windows; the app in front is recorded and brought back.
#[cfg(target_os = "macos")]
mod imp {
    use objc2::rc::Retained;
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

    use super::Target;

    fn running(target: Target) -> Option<Retained<NSRunningApplication>> {
        let pid = libc::pid_t::try_from(target.pid).ok()?;
        NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
    }

    /// The app in front; Panopin itself never counts.
    pub fn foreground() -> Option<Target> {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let pid = u32::try_from(app.processIdentifier()).ok()?;
        (pid != std::process::id()).then_some(Target { hwnd: 0, pid })
    }

    pub fn is_alive(target: Target) -> bool {
        running(target).is_some()
    }

    pub fn restore(target: Target) -> bool {
        running(target)
            .is_some_and(|app| app.activateWithOptions(NSApplicationActivationOptions(0)))
    }

    /// Option opens no menu bar on macOS, and the shelf takes the focus when it is shown.
    pub fn mask_alt_release() {}

    pub fn claim_foreground() {}
}

#[cfg(target_os = "macos")]
pub use imp::*;

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    use super::Target;

    pub fn foreground() -> Option<Target> {
        None
    }

    pub fn is_alive(_target: Target) -> bool {
        false
    }

    pub fn restore(_target: Target) -> bool {
        false
    }

    pub fn mask_alt_release() {}

    pub fn claim_foreground() {}
}

#[cfg(not(any(windows, target_os = "macos")))]
pub use imp::*;
