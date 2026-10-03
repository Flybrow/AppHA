//! Built-in system WebView: WebView2 (Windows) or WebKitGTK (Linux).
//! Runs in a child process started by the supervisor.

use std::time::Duration;

use anyhow::Result;
use url::Url;
use wry::WebViewBuilder;

use crate::config::Config;
use crate::{auth, health};
use crate::ui::{self, WindowSpec, js_string, render};

/// Exit code asking the supervisor to open the settings.
pub const EXIT_SETTINGS: i32 = 4;

const LOADING: &str = include_str!("../ui/assets/loading.html");

pub fn run(cfg: &Config) -> Result<()> {
    let w = &cfg.window;
    let spec = WindowSpec {
        title: "HA Kiosk",
        mode: w.mode,
        width: w.width,
        height: w.height,
        // Persistent profile: cookies, cache and HA session survive restarts.
        profile: "webview",
        close_code: 0,
        insecure_tls: cfg.insecure_tls,
        gpu: w.gpu,
        animations: w.animations,
        hide_cursor: w.hide_cursor,
    };
    // Local loading page: waits for HA, then redirects to the dashboard.
    let html = render(
        LOADING,
        &[("BASE", &js_string(cfg.url.as_str())), ("TARGET", &js_string(cfg.dashboard_url().as_str()))],
    );
    let auth_script = auth::init_script(&cfg.url, &cfg.token);
    let insecure = cfg.insecure_tls;
    let gpu = w.gpu;
    let css = [(!w.animations, ui::REDUCE_MOTION_CSS), (w.hide_cursor, ui::HIDE_CURSOR_CSS)]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, css)| *css)
        .collect::<Vec<_>>()
        .join("
");
    let base = cfg.url.clone();

    ui::run(spec, move |mut builder, ui| {
        builder = builder.with_html(html).with_initialization_script(ui::SETTINGS_GESTURE_JS);
        if !css.is_empty() {
            builder = builder.with_initialization_script(ui::inject_css_script(&css));
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

/// Tests HA in the background, then switches the loading page to the dashboard.
/// Called once the loading page is ready to receive calls.
fn wait_then_redirect(ui: ui::Ui, base: Url) {
    std::thread::spawn(move || {
        while !health::is_reachable(&base) {
            ui.eval("onUnreachable()");
            std::thread::sleep(Duration::from_secs(3));
        }
        ui.eval("onReachable()");
    });
}

/// RAM-oriented Chromium arguments: one renderer, no background service useless in a kiosk.
/// They replace wry's, whose `--disable-features` are included again.
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
    let _ = insecure; // handled by the WebKitGTK TLS policy (ui::attach)
    builder
}
