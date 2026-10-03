//! Boucle de supervision : attend HA, lance le navigateur, le relance si
//! il plante, dépasse son budget RAM, vieillit trop ou si HA revient après une coupure.

use std::path::Path;
use std::process::Child;
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::{Browser, Config};
use crate::memory::MemoryProbe;
use crate::{backend, health, info, warn};

const MIN_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Fréquence de détection de la fin du navigateur.
const EXIT_POLL: Duration = Duration::from_millis(250);
/// En dessous de cette durée de vie, un arrêt est considéré comme un crash au démarrage.
const STABLE_AFTER: Duration = Duration::from_secs(30);

/// Raison de l'arrêt d'une session navigateur.
enum Stop {
    Exited,
    /// L'utilisateur a demandé les paramètres depuis la WebView.
    Settings,
    Scheduled,
    Memory(u64),
    Reconnected,
}

pub fn run(mut cfg: Config, config_path: &Path) -> Result<()> {
    let mut backoff = MIN_BACKOFF;
    let mut probe = MemoryProbe::new();
    info!("dashboard : {} (navigateur : {:?})", cfg.dashboard_url(), cfg.resolved_browser());

    loop {
        // La WebView affiche sa propre page d'attente (avec accès aux paramètres).
        if cfg.resolved_browser() != Browser::Webview {
            wait_until_reachable(&cfg);
        }
        let started = Instant::now();
        let mut child = match backend::spawn(&cfg, config_path) {
            Ok(c) => {
                info!("navigateur lancé (pid {})", c.id());
                c
            }
            Err(e) => {
                warn!("{e:#}");
                backoff = wait_backoff(backoff);
                continue;
            }
        };

        let reason = watch(&cfg, &mut child, &mut probe, started);
        let _ = child.kill();
        let _ = child.wait();
        match reason {
            Stop::Exited => info!("le navigateur s'est arrêté"),
            Stop::Settings => {
                info!("ouverture des paramètres");
                let outcome = backend::open_settings(config_path, true)?;
                if outcome == backend::SettingsOutcome::Quit {
                    info!("arrêt demandé depuis les paramètres");
                    return Ok(());
                }
                if outcome == backend::SettingsOutcome::Saved {
                    match Config::load(config_path) {
                        Ok(new) => {
                            info!("configuration rechargée");
                            cfg = new;
                        }
                        Err(e) => warn!("{e:#}"),
                    }
                }
                backoff = MIN_BACKOFF;
                continue;
            }
            Stop::Scheduled => info!("redémarrage préventif planifié"),
            Stop::Memory(mb) => warn!("RAM du navigateur trop élevée ({mb} Mo), redémarrage"),
            Stop::Reconnected => info!("Home Assistant de nouveau joignable, rechargement"),
        }

        backoff = if started.elapsed() < STABLE_AFTER { wait_backoff(backoff) } else { MIN_BACKOFF };
    }
}

fn watch(cfg: &Config, child: &mut Child, probe: &mut MemoryProbe, started: Instant) -> Stop {
    let s = &cfg.supervisor;
    let interval = Duration::from_secs(s.check_interval_secs.max(1));
    let max_age = (s.restart_every_hours > 0).then(|| Duration::from_secs(s.restart_every_hours * 3600));
    let mut failures = 0u32;

    loop {
        if let Some(stop) = wait_exit(child, interval) {
            return stop;
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
                warn!("Home Assistant injoignable");
            }
        }
    }
}

/// Surveille la fin du process pendant `duration` (réaction rapide, ex. ouverture des paramètres).
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
            warn!("en attente de Home Assistant ({})...", cfg.url);
            warned = true;
        }
        sleep(MIN_BACKOFF);
    }
}

/// Patiente puis renvoie le délai suivant (doublé, plafonné).
fn wait_backoff(current: Duration) -> Duration {
    sleep(current);
    (current * 2).min(MAX_BACKOFF)
}
