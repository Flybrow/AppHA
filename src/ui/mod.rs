//! Fenêtre + WebView communes à toutes les interfaces (kiosk, paramètres).

pub mod settings;

use anyhow::{Context, Result};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Fullscreen, Window, WindowBuilder};
use wry::{WebContext, WebView, WebViewBuilder};

use crate::config::WindowMode;
use crate::paths;

/// Script injecté dans chaque page : geste et raccourci d'ouverture des paramètres.
pub const SETTINGS_GESTURE_JS: &str = include_str!("assets/gesture.js");

pub enum UiEvent {
    Eval(String),
    Exit(i32),
}

/// Poignée thread-safe pour piloter l'interface depuis un handler IPC.
#[derive(Clone)]
pub struct Ui(EventLoopProxy<UiEvent>);

impl Ui {
    pub fn eval(&self, js: impl Into<String>) {
        let _ = self.0.send_event(UiEvent::Eval(js.into()));
    }

    pub fn exit(&self, code: i32) {
        let _ = self.0.send_event(UiEvent::Exit(code));
    }
}

pub struct WindowSpec<'a> {
    pub title: &'a str,
    pub mode: WindowMode,
    pub width: u32,
    pub height: u32,
    /// Sous-dossier de profil WebView (cookies, cache) dans le dossier de données.
    pub profile: &'a str,
    /// Code de sortie quand l'utilisateur ferme la fenêtre.
    pub close_code: i32,
    /// Accepte les certificats HTTPS invalides (auto-signés). Sous Windows, voir
    /// les arguments navigateur de `backend::webview`.
    pub insecure_tls: bool,
}

/// Ouvre la fenêtre et exécute la boucle d'événements (ne rend jamais la main).
pub fn run<F>(spec: WindowSpec, setup: F) -> Result<()>
where
    F: for<'a> FnOnce(WebViewBuilder<'a>, Ui) -> Result<WebViewBuilder<'a>>,
{
    let event_loop = EventLoopBuilder::<UiEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title(spec.title)
        .with_inner_size(LogicalSize::new(spec.width, spec.height))
        .with_fullscreen((spec.mode == WindowMode::Fullscreen).then_some(Fullscreen::Borderless(None)))
        .with_decorations(spec.mode == WindowMode::Windowed)
        .build(&event_loop)
        .context("création de la fenêtre")?;

    let mut context = WebContext::new(paths::data_dir().map(|d| d.join(spec.profile)));
    let ui = Ui(event_loop.create_proxy());
    let builder = setup(WebViewBuilder::new_with_web_context(&mut context), ui)?;
    let webview = attach(builder, &window, spec.insecure_tls).context("création de la WebView")?;
    window.set_focus();
    let close_code = spec.close_code;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep_alive = (&window, &context);
        match event {
            Event::UserEvent(UiEvent::Eval(js)) => {
                let _ = webview.evaluate_script(&js);
            }
            Event::UserEvent(UiEvent::Exit(code)) => std::process::exit(code),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => std::process::exit(close_code),
            _ => {}
        }
    });
}

/// Autorise le prochain process lancé à passer au premier plan. Windows refuse
/// sinon le focus à une fenêtre ouverte par un process d'arrière-plan (le superviseur) :
/// à appeler depuis la fenêtre active avant de lui passer la main.
#[cfg(windows)]
pub fn allow_foreground_handoff() {
    const ASFW_ANY: u32 = u32::MAX;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn AllowSetForegroundWindow(pid: u32) -> i32;
    }
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
}

#[cfg(not(windows))]
pub fn allow_foreground_handoff() {}

const THEME_CSS: &str = include_str!("assets/theme.css");

/// Remplit un gabarit HTML : `/*THEME*/` puis chaque `/*CLÉ*/` par sa valeur.
pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter()
        .fold(template.replace("/*THEME*/", THEME_CSS), |html, (key, value)| html.replace(&format!("/*{key}*/"), value))
}

/// Chaîne JSON utilisable telle quelle dans du JavaScript.
pub fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

#[cfg(target_os = "linux")]
fn attach(builder: WebViewBuilder<'_>, window: &Window, insecure_tls: bool) -> wry::Result<WebView> {
    use tao::platform::unix::WindowExtUnix;
    use webkit2gtk::{TLSErrorsPolicy, WebViewExt, WebsiteDataManagerExt};
    use wry::{WebViewBuilderExtUnix, WebViewExtUnix};
    let webview = builder.build_gtk(window.default_vbox().expect("vbox GTK de tao"))?;
    if insecure_tls {
        // La page de chargement est locale : la politique s'applique avant toute requête vers HA.
        if let Some(manager) = webview.webview().website_data_manager() {
            manager.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
        }
    }
    Ok(webview)
}

#[cfg(not(target_os = "linux"))]
fn attach(builder: WebViewBuilder<'_>, window: &Window, _insecure_tls: bool) -> wry::Result<WebView> {
    builder.build(window)
}
