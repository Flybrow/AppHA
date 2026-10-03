//! Journalisation minimale sur stderr (capturée par journald sous systemd).

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { eprintln!("[ha-kiosk] {}", format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { eprintln!("[ha-kiosk] ATTENTION: {}", format_args!($($arg)*)) };
}
