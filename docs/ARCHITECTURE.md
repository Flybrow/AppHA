# Architecture

For contributors. Users: see the [README](../README.md).

## Processes

```
cage (Linux)                     ← Wayland compositor, started by systemd
└─ ha-kiosk run                  ← supervisor (~2 MB): monitors, restarts, updates
   ├─ ha-kiosk webview           ← window + WebView (WebView2 / WebKitGTK)
   │  └─ renderer, network…      ← browser processes
   └─ ha-kiosk settings          ← settings screen, on demand
```

- The browser always runs as a **child process**: a crash or a memory leak never reaches the supervisor, which restarts it.
- Children talk to the supervisor through their **exit code**. WebView: `4` = open the settings. Settings: `0` saved, `3` cancelled, `5` quit the kiosk, `6` updated.
- On Linux, the supervisor exits when `cage` stops (`PR_SET_PDEATHSIG`, plus a `wl_display.sync` probe when the browser dies). Otherwise `cage` would wait for its child forever.

## Modules (`src/`)

| Module | Role |
|---|---|
| `main.rs` | Command line and dispatch |
| `config.rs` | Load, validate and atomically save `config.toml`. `Config::commit` is the single entry point for changes (validate, save, autostart). |
| `supervisor.rs` | Supervision loop and background updates |
| `backend/` | Starts the browser: `webview` (built-in) or `external` (command) |
| `ui/` | Shared WebView window, loading page, settings screen, injected scripts |
| `cli/` | Text commands: `config`, `setup` (wizard), `check`, `update`; shared settings table |
| `i18n.rs` | English / French. Texts are inline pairs: `tr!("English", "Français")` |
| `auth.rs` | Automatic login: token stored in the HA frontend's `localStorage` |
| `update.rs` | Updates from GitHub releases (`curl`, plus `tar` on Linux) |
| `autostart.rs` | Start with the machine: `Run` registry key (Windows), systemd service (Linux) |
| `display.rs` | Rotation (`wlr-randr`) and compositor shutdown detection |
| `health.rs` | HA reachability (TCP connect, no TLS) |
| `memory.rs` | RAM of the browser process tree |
| `process.rs` | Auxiliary commands without a console window |
| `console.rs` | Console and error dialog on Windows (executable without a console) |
| `paths.rs` | Cross-platform system directories |

Injected into pages (`src/ui/assets/`): `gesture.js` (5 taps / F10 open the settings), `inject-css.js` (stylesheet applied to the document **and every shadow root**), `debug-animations.js` (only with `HA_KIOSK_DEBUG_ANIMATIONS=1`). The HTML pages carry English text plus `data-fr` attributes; JavaScript uses `t("en", "fr")`.

## Design choices

- **Animations at 1 ms rather than removed**: on a Raspberry Pi 3, HA frontend animations kept the CPU at 130 %; 1 ms brings it to 5 %. Removing them entirely breaks cards waiting for `transitionend` (Bubble Card popups).
- **GPU off by default**: saves ~70 MB on Windows; on a Pi 3, WebKit cannot render on that GPU anyway.
- **Updates through `curl`**: no bundled TLS stack, no RAM used between checks. On Windows the release asset is the `.exe` itself (versions ≤ 0.9.0 expected a `.zip` and must be downloaded again once).
- **Linux install**: binary in `/opt/ha-kiosk`, config in `/etc/ha-kiosk` (`0600`), both owned by the kiosk user, so the settings screen and updates work without root.
- **WebKit sandbox disabled** in the systemd service: bubblewrap fails there and gives a black screen. The kiosk only shows Home Assistant.

## Build and test

```sh
cargo test
cargo build --release
```

Linux needs `libwebkit2gtk-4.1-dev` and `libgtk-3-dev`.

## Release

1. Bump `version` in `Cargo.toml`.
2. Push to `main`: CI runs the tests and builds binaries, available as artifacts for testing.
3. Once validated, push a `vX.Y.Z` tag: CI checks it matches `Cargo.toml` and publishes the release.

Installed kiosks update themselves from the latest release: a tag goes straight to production.
