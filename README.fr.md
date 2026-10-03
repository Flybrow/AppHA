# HA Kiosk

[English](README.md) | **Français**

Affiche votre dashboard Home Assistant en plein écran sur un Raspberry Pi ou un PC Windows. Se connecte tout seul, se relance après un plantage ou une coupure, se met à jour tout seul.

## Avant de commencer

Dans Home Assistant : **votre profil → Sécurité → Jetons d'accès longue durée → Créer un jeton**. Copiez-le.

## Installation

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

## Utilisation

- **Paramètres** : 5 tapes dans le coin haut-gauche, ou **F10**.
- **Ligne de commande** : tapez `ha-kiosk` pour voir les commandes, ex. `ha-kiosk config rotation 180`.
- **Journaux** (Linux) : `journalctl -t ha-kiosk -f`

L'interface est en français si le système l'est. Pour forcer la langue : `ha-kiosk config language fr`.

## Dépannage

- **Lent sur Raspberry Pi** : laissez `animations` désactivé (par défaut). Au-delà de 60 °C, un Pi 3 ralentit : ajoutez un dissipateur.
- **Écran à l'envers** : `ha-kiosk config rotation 180`, puis `sudo systemctl restart ha-kiosk`.
- **« Permission denied » à l'enregistrement (Linux)** : relancez la commande d'installation ; vos réglages sont conservés.

---

[Architecture](docs/ARCHITECTURE.md) (en anglais) · Licence MIT
