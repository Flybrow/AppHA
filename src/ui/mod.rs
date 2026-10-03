//! Window + WebView shared by every screen (kiosk, settings).

pub mod settings;

use anyhow::{Context, Result};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Fullscreen, Window, WindowBuilder};
use wry::{WebContext, WebView, WebViewBuilder};

use crate::config::WindowMode;
use crate::paths;

/// Script injected in every page: gesture and shortcut opening the settings.
pub const SETTINGS_GESTURE_JS: &str = include_str!("assets/gesture.js");
const INJECT_CSS_JS: &str = include_str!("assets/inject-css.js");

/// Near-instant (1 ms) animations and transitions rather than removed ones: some
/// cards (Bubble Card…) wait for animationend / transitionend to open their popups.
pub const REDUCE_MOTION_CSS: &str = "*, *::before, *::after { animation-delay: 0s !important; animation-duration: 1ms !important; animation-iteration-count: 1 !important; transition-delay: 0s !important; transition-duration: 1ms !important; scroll-behavior: auto !important; }";

/// Invisible cursor everywhere (touch screens).
pub const HIDE_CURSOR_CSS: &str = "*, *::before, *::after { cursor: none !important; }";

/// Script applying `css` to the whole page, shadow roots included.
pub fn inject_css_script(css: &str) -> String {
    INJECT_CSS_JS.replace("/*CSS*/", &js_string(css))
}
/// Logs animations and display loops (HA_KIOSK_DEBUG_ANIMATIONS=1).
pub const DEBUG_ANIMATIONS_JS: &str = include_str!("assets/debug-animations.js");

pub enum UiEvent {
    Eval(String),
    Exit(i32),
}

/// Thread-safe handle to drive the UI from an IPC handler.
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
    /// WebView profile subdirectory (cookies, cache) in the data directory.
    pub profile: &'a str,
    /// Exit code when the user closes the window.
    pub close_code: i32,
    /// Accepts invalid (self-signed) HTTPS certificates. On Windows, see the
    /// browser arguments in `backend::webview`.
    pub insecure_tls: bool,
    /// Hardware acceleration (Linux: WebKitGTK policy; Windows: see `backend::webview`).
    pub gpu: bool,
    /// Hidden mouse cursor (touch screens).
    pub hide_cursor: bool,
    /// False: also asks the system to reduce motion (prefers-reduced-motion, Linux).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub animations: bool,
}

/// Opens the window and runs the event loop (never returns).
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
        .context("creating the window")?;

    let mut context = WebContext::new(paths::data_dir().map(|d| d.join(spec.profile)));
    let ui = Ui(event_loop.create_proxy());
    let builder = setup(WebViewBuilder::new_with_web_context(&mut context), ui)?;
    #[cfg(target_os = "linux")]
    if !spec.animations {
        // WebKitGTK exposes this GTK setting to pages as `prefers-reduced-motion`.
        use gtk::prelude::GtkSettingsExt;
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(false);
        }
    }
    let webview = attach(builder, &window, spec.insecure_tls, spec.gpu).context("creating the WebView")?;
    window.set_focus();
    if spec.hide_cursor {
        window.set_cursor_visible(false);
    }
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

/// Lets the next started process come to the foreground. Otherwise Windows denies
/// focus to a window opened by a background process (the supervisor): call it from
/// the active window before handing over.
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

/// Fills an HTML template: `/*THEME*/`, `/*LANG*/`, then each `/*KEY*/` with its value.
pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let base = template.replace("/*THEME*/", THEME_CSS).replace("/*LANG*/", &js_string(crate::i18n::code()));
    vars.iter().fold(base, |html, (key, value)| html.replace(&format!("/*{key}*/"), value))
}

/// JSON string usable as is in JavaScript.
pub fn js_string(s: &str) -> String {
    js_value(&serde_json::Value::from(s))
}

/// JSON value safe inside a `<script>`: `</` is escaped so a value containing
/// `</script>` cannot close the tag.
pub fn js_value(value: &serde_json::Value) -> String {
    value.to_string().replace("</", r"<\/")
}

#[cfg(target_os = "linux")]
fn attach(builder: WebViewBuilder<'_>, window: &Window, insecure_tls: bool, gpu: bool) -> wry::Result<WebView> {
    use tao::platform::unix::WindowExtUnix;
    use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, TLSErrorsPolicy, WebViewExt, WebsiteDataManagerExt};
    use wry::{WebViewBuilderExtUnix, WebViewExtUnix};
    let webview = builder.build_gtk(window.default_vbox().expect("tao GTK vbox"))?;
    if let Some(settings) = WebViewExt::settings(&webview.webview()) {
        let policy = if gpu { HardwareAccelerationPolicy::Always } else { HardwareAccelerationPolicy::Never };
        settings.set_hardware_acceleration_policy(policy);
    }
    if insecure_tls {
        // The loading page is local: the policy applies before any request to HA.
        if let Some(manager) = webview.webview().website_data_manager() {
            manager.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
        }
    }
    Ok(webview)
}

#[cfg(not(target_os = "linux"))]
fn attach(builder: WebViewBuilder<'_>, window: &Window, _insecure_tls: bool, _gpu: bool) -> wry::Result<WebView> {
    builder.build(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_values_cannot_close_the_tag() {
        assert_eq!(js_string("a</script>b"), r#""a<\/script>b""#);
    }

    #[test]
    fn render_replaces_placeholders() {
        let html = render("<style>/*THEME*/</style>/*X*/", &[("X", "42")]);
        assert!(html.ends_with("42") && !html.contains("/*THEME*/"));
    }
}
