//! WebView système intégrée : WebView2 (Windows) ou WebKitGTK (Linux).
//! Exécutée dans un process enfant lancé par le superviseur.

use std::time::Duration;

use anyhow::Result;
use url::Url;
use wry::WebViewBuilder;

use crate::config::Config;
use crate::{auth, health};
use crate::ui::{self, WindowSpec, js_string, render};

/// Code de sortie demandant au superviseur d'ouvrir les paramètres.
pub const EXIT_SETTINGS: i32 = 4;

const LOADING: &str = include_str!("../ui/assets/loading.html");

pub fn run(cfg: &Config) -> Result<()> {
    let w = &cfg.window;
    let spec = WindowSpec {
        title: "HA Kiosk",
        mode: w.mode,
        width: w.width,
        height: w.height,
        // Profil persistant : cookies, cache et session HA survivent aux redémarrages.
        profile: "webview",
        close_code: 0,
        insecure_tls: cfg.insecure_tls,
        gpu: w.gpu,
        animations: w.animations,
    };
    // Page de chargement locale : elle attend HA puis redirige vers le dashboard.
    let html = render(
        LOADING,
        &[("BASE", &js_string(cfg.url.as_str())), ("TARGET", &js_string(cfg.dashboard_url().as_str()))],
    );
    let auth_script = auth::init_script(&cfg.url, &cfg.token);
    let insecure = cfg.insecure_tls;
    let gpu = w.gpu;
    let animations = w.animations;
    let base = cfg.url.clone();

    ui::run(spec, move |mut builder, ui| {
        builder = builder.with_html(html).with_initialization_script(ui::SETTINGS_GESTURE_JS);
        if !animations {
            builder = builder.with_initialization_script(ui::REDUCE_MOTION_JS);
        }
        if std::env::var_os("HA_KIOSK_DEBUG_ANIMATIONS").is_some() {
            builder = builder.with_initialization_script(ui::DEBUG_ANIMATIONS_JS);
        }
        if let Some(script) = &auth_script {
            builder = builder.with_initialization_script(script);
        }
        builder = builder.with_ipc_handler(move |req| match req.body().as_str() {
            "ready" => wait_then_redirect(ui.clone(), base.clone()),
            body if body.starts_with("debug:") => crate::info!("animations {}", &body[6..]),
            "settings" => {
                ui::allow_foreground_handoff();
                ui.exit(EXIT_SETTINGS)
            }
            _ => {}
        });
        Ok(apply_browser_args(builder, gpu, insecure))
    })
}

/// Teste HA en arrière-plan puis bascule la page de chargement vers le dashboard.
/// Appelé quand la page de chargement est prête à recevoir les appels.
fn wait_then_redirect(ui: ui::Ui, base: Url) {
    std::thread::spawn(move || {
        while !health::is_reachable(&base) {
            ui.eval("onUnreachable()");
            std::thread::sleep(Duration::from_secs(3));
        }
        ui.eval("onReachable()");
    });
}

/// Arguments Chromium orientés RAM : un seul renderer, aucun service de fond inutile en kiosk.
/// Remplacent ceux de wry, dont les `--disable-features` sont réinclus.
#[cfg(windows)]
const LEAN_ARGS: &str = "--renderer-process-limit=1 --disable-background-networking --disable-component-update --disable-extensions --disable-sync --no-first-run --js-flags=--optimize-for-size --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,AutofillServerCommunication,Translate,OptimizationHints,MediaRouter";

#[cfg(windows)]
fn apply_browser_args(builder: WebViewBuilder<'_>, gpu: bool, insecure: bool) -> WebViewBuilder<'_> {
    use wry::WebViewBuilderExtWindows;
    let mut args = String::from(LEAN_ARGS);
    if !gpu {
        args.push_str(" --disable-gpu");
    }
    if insecure {
        args.push_str(" --ignore-certificate-errors");
    }
    builder.with_additional_browser_args(args)
}

#[cfg(not(windows))]
fn apply_browser_args(builder: WebViewBuilder<'_>, _gpu: bool, insecure: bool) -> WebViewBuilder<'_> {
    let _ = insecure; // géré par la politique TLS de WebKitGTK (ui::attach)
    builder
}
