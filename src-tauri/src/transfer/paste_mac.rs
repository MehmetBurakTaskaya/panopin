//! Cmd+V into the previous app on macOS (D-073). Sending keys needs Accessibility permission, and
//! a focused password field is never pasted into.

use std::{
    ptr::NonNull,
    thread,
    time::{Duration, Instant},
};

use objc2_application_services::{
    kAXTrustedCheckOptionPrompt, AXError, AXIsProcessTrusted, AXIsProcessTrustedWithOptions,
    AXUIElement,
};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFRetained, CFString, CFType};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation, CGKeyCode,
};

use crate::focus::{self, Target};

/// The V key's position (kVK_ANSI_V); macOS maps Cmd+V by key code.
const KEY_V: CGKeyCode = 9;

pub enum Sent {
    Pasted,
    PasswordField,
    Blocked,
}

/// Whether Panopin may send keys. The first time it may not, macOS shows its own prompt that
/// leads to System Settings → Privacy & Security → Accessibility.
pub fn may_paste() -> bool {
    // SAFETY: Accessibility queries; the options dictionary outlives the call.
    unsafe {
        if AXIsProcessTrusted() {
            return true;
        }
        let options = CFDictionary::<CFString, CFBoolean>::from_slices(
            &[kAXTrustedCheckOptionPrompt],
            &[CFBoolean::new(true)],
        );
        AXIsProcessTrustedWithOptions(Some(options.as_opaque()))
    }
}

/// Brings the app back, waits until it is in front, then sends Cmd+V.
pub fn send_paste(target: Target) -> Sent {
    if !focus::restore(target) || !wait_until_front(target, Duration::from_millis(600)) {
        return Sent::Blocked;
    }
    if focused_is_secure() {
        return Sent::PasswordField;
    }
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
    let (Some(down), Some(up)) = (
        CGEvent::new_keyboard_event(source.as_deref(), KEY_V, true),
        CGEvent::new_keyboard_event(source.as_deref(), KEY_V, false),
    ) else {
        return Sent::Blocked;
    };
    for event in [&*down, &*up] {
        CGEvent::set_flags(Some(event), CGEventFlags::MaskCommand);
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event));
    }
    Sent::Pasted
}

fn wait_until_front(target: Target, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if focus::foreground().is_some_and(|front| front.pid == target.pid) {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Whether the focused control of the app in front is a password field.
fn focused_is_secure() -> bool {
    // SAFETY: Accessibility queries whose results are owned; a slow app answers within a quarter
    // of a second or counts as no password field.
    unsafe {
        let system = AXUIElement::new_system_wide();
        let _ = system.set_messaging_timeout(0.25);
        attribute(&system, "AXFocusedUIElement")
            .and_then(|value| value.downcast::<AXUIElement>().ok())
            .and_then(|element| attribute(&element, "AXSubrole"))
            .and_then(|value| value.downcast::<CFString>().ok())
            .is_some_and(|subrole| subrole.to_string() == "AXSecureTextField")
    }
}

/// # Safety
/// `element` must be a valid Accessibility element.
unsafe fn attribute(element: &AXUIElement, name: &str) -> Option<CFRetained<CFType>> {
    let mut value: *const CFType = std::ptr::null();
    let status = element.copy_attribute_value(&CFString::from_str(name), NonNull::from(&mut value));
    if status != AXError::Success {
        return None;
    }
    Some(CFRetained::from_raw(NonNull::new(value.cast_mut())?))
}
