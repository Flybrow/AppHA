//! User-facing language: English (default) or French.
//!
//! Texts are written inline as pairs, `tr!("English", "Français")`, so both
//! versions stay side by side. Logs and internal errors are English only.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

/// `language` setting.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Follow the system language.
    #[default]
    Auto,
    En,
    Fr,
}

const UNSET: u8 = 0;
const EN: u8 = 1;
const FR: u8 = 2;
static CURRENT: AtomicU8 = AtomicU8::new(UNSET);

/// Applies the configured language (`Auto` keeps the system language).
pub fn set(language: Language) {
    let value = match language {
        Language::Auto => detect(),
        Language::En => EN,
        Language::Fr => FR,
    };
    CURRENT.store(value, Ordering::Relaxed);
}

pub fn is_fr() -> bool {
    let mut value = CURRENT.load(Ordering::Relaxed);
    if value == UNSET {
        value = detect();
        CURRENT.store(value, Ordering::Relaxed);
    }
    value == FR
}

/// Language code for the web pages (`en` or `fr`).
pub fn code() -> &'static str {
    if is_fr() { "fr" } else { "en" }
}

/// Picks the text matching the current language.
#[macro_export]
macro_rules! tr {
    ($en:expr, $fr:expr $(,)?) => {
        if $crate::i18n::is_fr() { $fr } else { $en }
    };
}

#[cfg(windows)]
fn detect() -> u8 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    const LANG_FRENCH: u16 = 0x0c;
    // The low 10 bits are the primary language, whatever the region (fr-FR, fr-CA…).
    if unsafe { GetUserDefaultUILanguage() } & 0x3ff == LANG_FRENCH { FR } else { EN }
}

#[cfg(not(windows))]
fn detect() -> u8 {
    let french = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|var| std::env::var(var).ok())
        .find(|v| !v.is_empty())
        .is_some_and(|v| v.starts_with("fr"));
    if french { FR } else { EN }
}
