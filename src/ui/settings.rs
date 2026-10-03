//! Écran de paramètres : édite `config.toml` puis se ferme.
//! Code de sortie : `SAVED` si la config a été enregistrée, `QUIT` pour arrêter le kiosk, `CANCELLED` sinon.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;
use url::Url;

use super::{Ui, WindowSpec, js_string, render};
use crate::config::{Config, WindowMode};
use crate::{health, update};

pub const SAVED: i32 = 0;
pub const CANCELLED: i32 = 3;
pub const QUIT: i32 = 5;
/// Nouvelle version installée : le superviseur doit se relancer.
pub const UPDATED: i32 = 6;

const TEMPLATE: &str = include_str!("assets/settings.html");

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Message {
    Test { url: String },
    /// `quit` : enregistre puis arrête le kiosk au lieu de le (re)lancer.
    Save {
        config: serde_json::Value,
        #[serde(default)]
        quit: bool,
    },
    Cancel,
    Quit,
    UpdateCheck,
    UpdateInstall,
}

/// `can_cancel` : faux au premier lancement, quand aucune config valide n'existe.
pub fn run(path: &Path, can_cancel: bool) -> Result<()> {
    let initial = Config::load(path)
        .ok()
        .and_then(|c| serde_json::to_value(c).ok())
        .unwrap_or_else(Config::defaults_json);
    let html = render(
        TEMPLATE,
        &[
            ("CONFIG", &initial.to_string()),
            ("CAN_CANCEL", if can_cancel { "true" } else { "false" }),
            ("VERSION", &js_string(env!("CARGO_PKG_VERSION"))),
        ],
    );
    let path = path.to_path_buf();
    let spec = WindowSpec {
        title: "HA Kiosk — Paramètres",
        mode: WindowMode::Windowed,
        width: 720,
        height: 820,
        profile: "settings",
        close_code: CANCELLED,
        insecure_tls: false,
        gpu: false,
        animations: true,
        hide_cursor: false,
    };

    super::run(spec, move |builder, ui| {
        Ok(builder.with_html(html).with_ipc_handler(move |req| handle(&ui, &path, req.body())))
    })
}

fn handle(ui: &Ui, path: &PathBuf, body: &str) {
    let Ok(msg) = serde_json::from_str::<Message>(body) else { return };
    match msg {
        Message::Test { url } => {
            let ui = ui.clone();
            // Test réseau hors de la boucle d'événements pour ne pas figer la fenêtre.
            std::thread::spawn(move || {
                let ok = Url::parse(&url).is_ok_and(|u| health::is_reachable(&u));
                ui.eval(format!("onTestResult({ok})"));
            });
        }
        Message::Save { config, quit } => match Config::from_json(config).and_then(|c| c.save(path)) {
            Ok(()) => ui.exit(if quit { QUIT } else { SAVED }),
            Err(e) => ui.eval(format!("showError({})", js_string(&format!("{e:#}")))),
        },
        Message::Cancel => ui.exit(CANCELLED),
        Message::Quit => ui.exit(QUIT),
        Message::UpdateCheck => update_in_background(ui, false),
        Message::UpdateInstall => update_in_background(ui, true),
    }
}

/// Recherche (et installe si `install`) la dernière version, hors de la boucle d'événements.
fn update_in_background(ui: &Ui, install: bool) {
    let ui = ui.clone();
    std::thread::spawn(move || {
        let result = update::check().and_then(|release| match release {
            Some(r) if install => update::install(&r).map(|()| Some(r)),
            other => Ok(other),
        });
        match result {
            Ok(Some(_)) if install => ui.exit(UPDATED),
            Ok(Some(r)) => ui.eval(format!("onUpdate('available', {})", js_string(&r.tag))),
            Ok(None) => ui.eval("onUpdate('none')"),
            Err(e) => {
                let hint = if cfg!(unix) { " — essayez : sudo ha-kiosk update" } else { "" };
                ui.eval(format!("onUpdate('error', {})", js_string(&format!("{e:#}{hint}"))));
            }
        }
    });
}
