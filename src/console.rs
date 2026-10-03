//! Console output for an executable without a console (Windows "windows" subsystem).

/// Attaches the parent terminal's console, if any, so messages stay visible on
/// the command line. True if a console is available.
pub fn attach() -> bool {
    let attached = attach_parent();
    let _ = ATTACHED.set(attached);
    attached
}

/// Result of `attach` (false if it was not called).
pub fn attached() -> bool {
    ATTACHED.get().copied().unwrap_or(false)
}

static ATTACHED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

#[cfg(all(windows, not(debug_assertions)))]
fn attach_parent() -> bool {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    unsafe extern "system" {
        fn AttachConsole(pid: u32) -> i32;
    }
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 }
}

#[cfg(not(all(windows, not(debug_assertions))))]
fn attach_parent() -> bool {
    true
}

/// Shows a fatal error when no console can (started by double-click).
#[cfg(windows)]
pub fn error_dialog(text: &str) {
    const MB_ICONERROR: u32 = 0x10;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, flags: u32) -> i32;
    }
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    unsafe { MessageBoxW(0, wide(text).as_ptr(), wide("HA Kiosk").as_ptr(), MB_ICONERROR) };
}

#[cfg(not(windows))]
pub fn error_dialog(_text: &str) {}
