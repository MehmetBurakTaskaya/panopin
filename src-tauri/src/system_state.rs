//! Whether Panopin should stay silent: a full-screen game or video, a presentation, or a focus
//! session is active (D-021). Checked when the shelf opens, before it covers the screen.

#[cfg(windows)]
pub fn wants_quiet() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    // SAFETY: a query without arguments.
    let busy = unsafe { SHQueryUserNotificationState() }.is_ok_and(|state| {
        [
            QUNS_BUSY,
            QUNS_RUNNING_D3D_FULL_SCREEN,
            QUNS_PRESENTATION_MODE,
        ]
        .contains(&state)
    });
    busy || focus_session_active()
}

/// Focus sessions exist on newer Windows 11 builds only; elsewhere this is simply false.
#[cfg(windows)]
fn focus_session_active() -> bool {
    use windows::UI::Shell::FocusSessionManager;
    FocusSessionManager::IsSupported().unwrap_or(false)
        && FocusSessionManager::GetDefault()
            .and_then(|manager| manager.IsFocusActive())
            .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn wants_quiet() -> bool {
    false
}
