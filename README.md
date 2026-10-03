# HA Kiosk

<details open>
<summary><b>🇬🇧 English</b></summary>

Shows your Home Assistant dashboard full screen on a Raspberry Pi or a Windows PC. Logs in by itself, recovers from crashes and outages, updates itself.

### Before you start

In Home Assistant: **your profile → Security → Long-lived access tokens → Create token**. Copy it.

### Install

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

### Use

- **Settings**: tap the top-left corner 5 times, or press **F10**.
- **Command line**: type `ha-kiosk` to see the commands, e.g. `ha-kiosk config rotation 180`.
- **Logs** (Linux): `journalctl -t ha-kiosk -f`

The interface follows the system language (English or French). To force it: `ha-kiosk config language en`.

### Troubleshooting

- **Slow on a Raspberry Pi**: keep `animations` off (default). Above 60 °C a Pi 3 slows down: add a heatsink.
- **Upside-down screen**: `ha-kiosk config rotation 180`, then `sudo systemctl restart ha-kiosk`.
- **"Permission denied" when saving (Linux)**: run the install command again; your settings are kept.

</details>

<details>
<summary><b>🇫🇷 Français</b></summary>

Affiche votre dashboard Home Assistant en plein écran sur un Raspberry Pi ou un PC Windows. Se connecte tout seul, se relance après un plantage ou une coupure, se met à jour tout seul.

### Avant de commencer

Dans Home Assistant : **votre profil → Sécurité → Jetons d'accès longue durée → Créer un jeton**. Copiez-le.

### Installation

<details>
<summary><b>Raspberry Pi</b> (Pi 3/4/5, Raspberry Pi OS 64 bits)</summary>

```sh
curl -fsSL https://raw.githubusercontent.com/Flybrow/AppHA/main/deploy/linux/get.sh | sudo sh -s -- $USER
sudo raspi-config nonint do_boot_behaviour B1   # démarrage sans bureau
sudo reboot
```

Pendant l'installation, un assistant demande l'adresse de Home Assistant et le jeton.

</details>

<details>
<summary><b>Windows</b> (10/11)</summary>

1. Téléchargez `ha-kiosk-windows-x86_64.exe` depuis la [dernière version](https://github.com/Flybrow/AppHA/releases/latest).
2. Placez-le dans un dossier à vous (ex. `C:\HA-Kiosk`) et lancez-le.
3. Saisissez l'adresse de Home Assistant et le jeton, puis **Enregistrer et lancer**.

Si Windows SmartScreen vous avertit : **Informations complémentaires → Exécuter quand même**.

</details>

### Utilisation

- **Paramètres** : 5 tapes dans le coin haut-gauche, ou **F10**.
- **Ligne de commande** : tapez `ha-kiosk` pour voir les commandes, ex. `ha-kiosk config rotation 180`.
- **Journaux** (Linux) : `journalctl -t ha-kiosk -f`

L'interface est en français si le système l'est. Pour forcer la langue : `ha-kiosk config language fr`.

### Dépannage

- **Lent sur Raspberry Pi** : laissez `animations` désactivé (par défaut). Au-delà de 60 °C, un Pi 3 ralentit : ajoutez un dissipateur.
- **Écran à l'envers** : `ha-kiosk config rotation 180`, puis `sudo systemctl restart ha-kiosk`.
- **« Permission denied » à l'enregistrement (Linux)** : relancez la commande d'installation ; vos réglages sont conservés.

</details>

---

[Architecture](docs/ARCHITECTURE.md) · MIT License
