//! Ctrl+Enter on Windows: the checks before any input goes to another app, then Ctrl+V.

use std::{thread, time::Duration};

use windows::core::BOOL;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND, LPARAM},
    Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
        TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
    },
    System::{
        Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED},
        Threading::{
            GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::{
        Accessibility::{CUIAutomation, IUIAutomation},
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
        },
        WindowsAndMessaging::{EnumChildWindows, GetWindowThreadProcessId},
    },
};

use crate::focus::{self, Target, MASK_KEY};

pub enum Sent {
    Pasted,
    PasswordField,
    /// Focus did not arrive or Windows dropped the keystrokes.
    Blocked,
}

/// The mandatory integrity level (a RID such as 0x2000 for medium) of a process token.
unsafe fn integrity_of(process: HANDLE) -> Option<u32> {
    let mut token = HANDLE::default();
    OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
    let mut len = 0u32;
    let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut len);
    let mut buffer = vec![0u64; (len as usize).div_ceil(8)];
    let read = GetTokenInformation(
        token,
        TokenIntegrityLevel,
        Some(buffer.as_mut_ptr().cast()),
        len,
        &mut len,
    );
    let _ = CloseHandle(token);
    read.ok()?;
    let label = &*buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
    let sid = label.Label.Sid;
    let count = *GetSidSubAuthorityCount(sid);
    Some(*GetSidSubAuthority(sid, u32::from(count.checked_sub(1)?)))
}

/// UIPI: input from Panopin never reaches a window of a higher integrity level, such as an app
/// run as administrator. A process that cannot even be queried counts as higher.
pub fn is_elevated_above_us(pid: u32) -> bool {
    // SAFETY: handles are checked and closed; the token buffer is sized by the first call.
    unsafe {
        let Some(ours) = integrity_of(GetCurrentProcess()) else {
            return false;
        };
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return true;
        };
        let theirs = integrity_of(process);
        let _ = CloseHandle(process);
        theirs.is_none_or(|level| level > ours)
    }
}

/// The processes whose focused element belongs to the target: its own, and those of its child
/// windows. A Store app's frame belongs to ApplicationFrameHost while the app runs in its own
/// process; matching the frame alone made every paste there wait and skip the password check.
unsafe fn target_processes(target: Target) -> Vec<u32> {
    // lparam is the vector handed to EnumChildWindows below, alive during the call.
    unsafe extern "system" fn add(child: HWND, lparam: LPARAM) -> BOOL {
        let pids = &mut *(lparam.0 as *mut Vec<u32>);
        let mut pid = 0u32;
        GetWindowThreadProcessId(child, Some(&mut pid));
        if pid != 0 && !pids.contains(&pid) {
            pids.push(pid);
        }
        BOOL::from(true)
    }
    let mut pids = vec![target.pid];
    let _ = EnumChildWindows(
        Some(HWND(target.hwnd as *mut _)),
        Some(add),
        LPARAM(&raw mut pids as isize),
    );
    pids
}

/// Whether the element that has focus in the target app is a password field, if UI Automation
/// can tell within a short time (focus inside the app is restored asynchronously).
fn focused_is_password(target: Target) -> Option<bool> {
    // SAFETY: COM is initialised for this worker thread before UI Automation is created.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let pids = target_processes(target);
        for _ in 0..15 {
            if let Ok(element) = automation.GetFocusedElement() {
                let owner = element
                    .CurrentProcessId()
                    .ok()
                    .and_then(|pid| u32::try_from(pid).ok());
                if owner.is_some_and(|pid| pids.contains(&pid)) {
                    return element.CurrentIsPassword().ok().map(|yes| yes.as_bool());
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        None
    }
}

fn held(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: reads the asynchronous key state only.
    unsafe { GetAsyncKeyState(i32::from(vk.0)) < 0 }
}

/// Waits for the previous window to be in front, refuses password fields, releases held
/// modifiers so the app sees exactly Ctrl+V, then sends it.
pub fn send_paste(target: Target) -> Sent {
    if !focus::wait_until_foreground(target, Duration::from_millis(600)) {
        return Sent::Blocked;
    }
    if focused_is_password(target) == Some(true) {
        return Sent::PasswordField;
    }
    let mut inputs = Vec::new();
    if [VK_MENU, VK_LWIN, VK_RWIN].into_iter().any(held) {
        // Releasing Alt or Win alone would open a menu; the mask key in between prevents that.
        inputs.extend([focus::key(MASK_KEY, false), focus::key(MASK_KEY, true)]);
    }
    for vk in [VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN, VK_CONTROL] {
        if held(vk) {
            inputs.push(focus::key(vk, true));
        }
    }
    inputs.extend([
        focus::key(VK_CONTROL, false),
        focus::key(VK_V, false),
        focus::key(VK_V, true),
        focus::key(VK_CONTROL, true),
    ]);
    if focus::send(&inputs) {
        Sent::Pasted
    } else {
        Sent::Blocked
    }
}
