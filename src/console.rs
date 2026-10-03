//! Sortie console d'un exécutable sans console (sous-système Windows « windows »).

/// Rattache la console du terminal parent, s'il y en a un, pour que les messages
/// restent visibles en ligne de commande. Vrai si une console est disponible.
pub fn attach() -> bool {
    let attached = attach_parent();
    let _ = ATTACHED.set(attached);
    attached
}

/// Résultat de `attach` (faux s'il n'a pas été appelé).
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

/// Affiche une erreur fatale quand aucune console ne peut la montrer (lancement par double-clic).
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
