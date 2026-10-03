//! Minimal logging to stderr (captured by journald under systemd).

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { eprintln!("[ha-kiosk] {}", format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { eprintln!("[ha-kiosk] WARNING: {}", format_args!($($arg)*)) };
}
