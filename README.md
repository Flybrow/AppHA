# HA Kiosk

**English** | [Français](README.fr.md)

Shows your Home Assistant dashboard full screen on a Raspberry Pi or a Windows PC. Logs in by itself, recovers from crashes and outages, updates itself.

## Before you start

In Home Assistant: **your profile → Security → Long-lived access tokens → Create token**. Copy it.

## Install

<details>
<summary><b>Raspberry Pi</b> (Pi 3/4/5, Raspberry Pi OS 64-bit)</summary>

```sh
curl -fsSL https://raw.githubusercontent.com/Flybrow/AppHA/main/deploy/linux/get.sh | sudo sh -s -- $USER
sudo raspi-config nonint do_boot_behaviour B1   # boot without the desktop
sudo reboot
```

A wizard asks for your Home Assistant address and token during the install.

</details>

<details>
<summary><b>Windows</b> (10/11)</summary>

1. Download `ha-kiosk-windows-x86_64.exe` from the [latest release](https://github.com/Flybrow/AppHA/releases/latest).
2. Put it in a folder of your own (e.g. `C:\HA-Kiosk`) and run it.
3. Enter your Home Assistant address and token, then **Save and start**.

If Windows SmartScreen warns you: **More info → Run anyway**.

</details>

## Use

- **Settings**: tap the top-left corner 5 times, or press **F10**.
- **Command line**: type `ha-kiosk` to see the commands, e.g. `ha-kiosk config rotation 180`.
- **Logs** (Linux): `journalctl -t ha-kiosk -f`

The interface follows the system language (English or French). To force it: `ha-kiosk config language en`.

## Troubleshooting

- **Slow on a Raspberry Pi**: keep `animations` off (default). Above 60 °C a Pi 3 slows down: add a heatsink.
- **Upside-down screen**: `ha-kiosk config rotation 180`, then `sudo systemctl restart ha-kiosk`.
- **"Permission denied" when saving (Linux)**: run the install command again; your settings are kept.

---

[Architecture](docs/ARCHITECTURE.md) · MIT License
