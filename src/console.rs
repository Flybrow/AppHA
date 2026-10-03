//! Sortie console d'un exÃ©cutable sans console (sous-systÃ¨me Windows Â« windows Â»).

/// Rattache la console du terminal parent, s'il y en a un, pour que les messages
/// restent visibles en ligne de commande. Vrai si une console est disponible.
#[cfg(all(windows, not(debug_assertions)))]
pub fn attach() -> bool {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    unsafe extern "system" {
        fn AttachConsole(pid: u32) -> i32;
    }
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 }
}

#[cfg(not(all(windows, not(debug_assertions))))]
pub fn attach() -> bool {
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
