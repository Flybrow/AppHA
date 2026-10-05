//! Supervision loop: waits for HA, starts the browser, restarts it when it
//! crashes, exceeds its RAM budget, gets too old, or when HA comes back after an outage.

use std::path::Path;
use std::process::Child;
use std::sync::mpsc::{self, Receiver};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::{Browser, Config};
use crate::memory::MemoryProbe;
use crate::{backend, health, info, update, warn};

const MIN_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// How often the browser's exit is checked.
const EXIT_POLL: Duration = Duration::from_millis(250);
/// Below this lifetime, a stop counts as a startup crash.
const STABLE_AFTER: Duration = Duration::from_secs(30);

/// Why a browser session stopped.
enum Stop {
    Exited,
    /// The user asked for the settings from the WebView.
    Settings,
    Scheduled,
    Memory(u64),
    Reconnected,
    /// A new version has just been installed.
    Updated,
}

/// Interval between two update checks.
const UPDATE_EVERY: Duration = Duration::from_secs(6 * 3600);

/// Checks and installs updates in the background, never blocking supervision.
/// Only when the executable is writable by the current user (Linux install:
/// /opt/ha-kiosk belongs to the kiosk user).
struct Updater {
    next: Instant,
    pending: Option<Receiver<bool>>,
}

impl Updater {
    fn new(enabled: bool) -> Option<Self> {
        (enabled && update::can_self_update()).then(|| Self { next: Instant::now(), pending: None })
    }

    /// True once a new version is installed and ready to be restarted.
    fn poll(&mut self) -> bool {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(installed) => {
                    self.pending = None;
                    return installed;
                }
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => self.pending = None,
            }
        }
        if Instant::now() >= self.next {
            self.next = Instant::now() + UPDATE_EVERY;
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let installed = match update::check() {
                    Ok(Some(release)) => {
                        info!("new version {}, installing", release.tag);
                        update::install(&release).map_err(|e| warn!("update: {e:#}")).is_ok()
                    }
                    Ok(None) => false,
                    Err(e) => {
                        warn!("update check: {e:#}");
                        false
                    }
                };
                let _ = tx.send(installed);
            });
            self.pending = Some(rx);
        }
        false
    }
}

pub fn run(mut cfg: Config, config_path: &Path) -> Result<()> {
    let mut backoff = MIN_BACKOFF;
    let mut probe = MemoryProbe::new();
    let mut updater = Updater::new(cfg.auto_update);
    crate::display::apply_rotation(&cfg);
    // Windows: re-sync the startup registration (current exe path).
    if cfg!(windows)
        && let Err(e) = crate::autostart::apply(cfg.autostart)
    {
        warn!("{e:#}");
    }
    info!("dashboard: {} (browser: {:?})", cfg.dashboard_url(), cfg.resolved_browser());

    loop {
        // The WebView shows its own waiting page (with access to the settings).
        if cfg.resolved_browser() != Browser::Webview {
            wait_until_reachable(&cfg);
        }
        let started = Instant::now();
        let mut child = match backend::spawn(&cfg, config_path) {
            Ok(c) => {
                info!("browser started (pid {})", c.id());
                c
            }
            Err(e) => {
                warn!("{e:#}");
                backoff = wait_backoff(backoff);
                continue;
            }
        };

        let reason = watch(&cfg, &mut child, &mut probe, started, &mut updater);
        let _ = child.kill();
        let _ = child.wait();
        match reason {
            Stop::Exited => {
                // Under cage, stopping the compositor kills the browser: stop too,
                // otherwise cage (which waits for its child) never ends.
                if !crate::display::compositor_alive() {
                    info!("compositor stopped, exiting");
                    return Ok(());
                }
                info!("browser stopped")
            }
            Stop::Settings => {
                info!("opening the settings");
                // A failure here must not stop the kiosk: resume the display.
                let outcome = backend::open_settings(config_path, true).unwrap_or_else(|e| {
                    warn!("settings: {e:#}");
                    backend::SettingsOutcome::Cancelled
                });
                if outcome == backend::SettingsOutcome::Updated {
                    return restart_self();
                }
                if outcome == backend::SettingsOutcome::Quit {
                    info!("exit requested from the settings");
                    return Ok(());
                }
                if outcome == backend::SettingsOutcome::Saved {
                    match Config::load(config_path) {
                        Ok(new) => {
                            info!("configuration reloaded");
                            if new.window.rotation != cfg.window.rotation && touch_rotation_installed() {
                                // The touchscreen is rotated before cage starts: have systemd
                                // restart the service (Restart=on-failure).
                                info!("rotation changed, restarting the service for the touchscreen");
                                std::process::exit(75);
                            }
                            cfg = new;
                            crate::i18n::set(cfg.language);
                            crate::display::apply_rotation(&cfg);
                        }
                        Err(e) => warn!("{e:#}"),
                    }
                }
                backoff = MIN_BACKOFF;
                continue;
            }
            Stop::Scheduled => info!("scheduled preventive restart"),
            Stop::Memory(mb) => warn!("browser RAM too high ({mb} MB), restarting"),
            Stop::Reconnected => info!("Home Assistant reachable again, reloading"),
            Stop::Updated => return restart_self(),
        }

        backoff = if started.elapsed() < STABLE_AFTER { wait_backoff(backoff) } else { MIN_BACKOFF };
    }
}

fn watch(cfg: &Config, child: &mut Child, probe: &mut MemoryProbe, started: Instant, updater: &mut Option<Updater>) -> Stop {
    let s = &cfg.supervisor;
    let interval = Duration::from_secs(s.check_interval_secs.max(1));
    let max_age = (s.restart_every_hours > 0).then(|| Duration::from_secs(s.restart_every_hours * 3600));
    let mut failures = 0u32;

    loop {
        if let Some(stop) = wait_exit(child, interval) {
            return stop;
        }
        if updater.as_mut().is_some_and(Updater::poll) {
            return Stop::Updated;
        }
        if max_age.is_some_and(|age| started.elapsed() >= age) {
            return Stop::Scheduled;
        }
        if s.max_memory_mb > 0 {
            let mb = probe.tree_mb(child.id());
            if mb > s.max_memory_mb {
                return Stop::Memory(mb);
            }
        }
        if health::is_reachable(&cfg.url) {
            if failures >= s.failures_before_down && s.reload_on_reconnect {
                return Stop::Reconnected;
            }
            failures = 0;
        } else {
            failures = failures.saturating_add(1);
            if failures == s.failures_before_down {
                warn!("Home Assistant unreachable");
            }
        }
    }
}

/// Restarts the new executable with the same arguments, then exits.
/// On Unix, replaces the process in place (same PID): cage, whose direct child is
/// the supervisor, sees no exit and keeps the screen.
/// True when running as the systemd service that rotates the touchscreen at startup.
fn touch_rotation_installed() -> bool {
    cfg!(unix) && std::env::var_os("INVOCATION_ID").is_some() && std::path::Path::new("/usr/local/lib/ha-kiosk/touch-rotation.sh").exists()
}

fn restart_self() -> Result<()> {
    info!("restarting on the new version");
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.args(std::env::args_os().skip(1));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        return Err(cmd.exec().into());
    }
    #[cfg(not(unix))]
    {
        cmd.spawn()?;
        Ok(())
    }
}

/// Watches for the process exit during `duration` (fast reaction, e.g. opening the settings).
fn wait_exit(child: &mut Child, duration: Duration) -> Option<Stop> {
    let deadline = Instant::now() + duration;
    loop {
        match child.try_wait() {
            Ok(None) => {}
            Ok(Some(status)) if status.code() == Some(backend::webview::EXIT_SETTINGS) => return Some(Stop::Settings),
            _ => return Some(Stop::Exited),
        }
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        sleep(EXIT_POLL.min(deadline - now));
    }
}

fn wait_until_reachable(cfg: &Config) {
    let mut warned = false;
    while !health::is_reachable(&cfg.url) {
        if !warned {
            warn!("waiting for Home Assistant ({})...", cfg.url);
            warned = true;
        }
        sleep(MIN_BACKOFF);
    }
}

/// Waits, then returns the next delay (doubled, capped).
fn wait_backoff(current: Duration) -> Duration {
    sleep(current);
    (current * 2).min(MAX_BACKOFF)
}
