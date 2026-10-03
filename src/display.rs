//! Screen rotation under Wayland (cage) through `wlr-randr`, and detection of the
//! compositor shutting down. On Windows, rotation is a system display setting.

use crate::config::Config;
#[cfg(unix)]
use crate::{info, warn};

/// Applies `window.rotation` to every display output.
#[cfg(unix)]
pub fn apply_rotation(cfg: &Config) {
    use std::process::Command;

    if std::env::var_os("WAYLAND_DISPLAY").is_none() || !crate::paths::in_path("wlr-randr") {
        if cfg.window.rotation != 0 {
            warn!("rotation ignored: requires Wayland (cage) and wlr-randr");
        }
        return;
    }
    let transform = match cfg.window.rotation {
        90 => "90",
        180 => "180",
        270 => "270",
        _ => "normal",
    };
    let Ok(out) = Command::new("wlr-randr").output() else { return };
    // Output names are the non-indented lines: `DSI-1 "…"`.
    let listing = String::from_utf8_lossy(&out.stdout);
    for output in listing.lines().filter(|l| !l.starts_with(char::is_whitespace)).filter_map(|l| l.split_whitespace().next()) {
        match Command::new("wlr-randr").args(["--output", output, "--transform", transform]).status() {
            Ok(s) if s.success() => info!("rotation {transform} applied to {output}"),
            _ => warn!("cannot rotate {output}"),
        }
    }
}

#[cfg(not(unix))]
pub fn apply_rotation(_cfg: &Config) {}

/// False if the Wayland compositor (cage) no longer answers: it is shutting down.
/// True outside Wayland. Does a `wl_display.sync` → `wl_callback.done` round trip.
#[cfg(unix)]
pub fn compositor_alive() -> bool {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::time::Duration;

    let Some(name) = std::env::var_os("WAYLAND_DISPLAY") else { return true };
    let mut path = PathBuf::from(&name);
    if path.is_relative() {
        let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") else { return true };
        path = PathBuf::from(dir).join(name);
    }
    let Ok(mut socket) = UnixStream::connect(&path) else { return false };
    let _ = socket.set_read_timeout(Some(Duration::from_secs(2)));
    // Object 1 (wl_display), opcode 0 (sync), size 12 bytes, new object 2.
    let mut msg = Vec::with_capacity(12);
    for word in [1u32, 12 << 16, 2] {
        msg.extend_from_slice(&word.to_ne_bytes());
    }
    let mut reply = [0u8; 8];
    socket.write_all(&msg).is_ok() && socket.read_exact(&mut reply).is_ok()
}

#[cfg(not(unix))]
pub fn compositor_alive() -> bool {
    true
}
