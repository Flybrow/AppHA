# ha-kiosk

Afficheur kiosk léger et supervisé pour les dashboards Home Assistant, sous Windows et Linux (Raspberry Pi compris).

- **Superviseur** d'environ 2 Mo de RAM, qui attend que HA soit joignable avant de lancer le navigateur. Il relance le navigateur en cas de crash, de dépassement du budget RAM, de redémarrage planifié ou de retour de HA après une coupure.
- **Navigateurs** au choix : `cog` (WPE WebKit, le plus léger sous Linux), WebView intégrée (WebView2 ou WebKitGTK), ou n'importe quelle commande (Chromium, Firefox…).
- **Connexion automatique** avec un jeton longue durée (backend `webview`).
- HTTP, HTTPS, IP locale, domaine, Nabu Casa.

## Utilisation

```sh
cp config.example.toml config.toml   # puis éditer
ha-kiosk check                        # vérifie config et connexion
ha-kiosk                              # lance et supervise
ha-kiosk settings                     # écran de paramètres
```

Sans `config.toml`, l'écran de paramètres s'ouvre au premier lancement. Avec le backend `webview`, on l'ouvre aussi en tapant 5 fois dans le coin haut-gauche (en moins de 3 s), avec F10 ou Ctrl+, : la config est enregistrée puis le navigateur est relancé.

## Compilation

```sh
cargo build --release
```

Pour Linux, il faut les en-têtes WebKitGTK (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`) pour le backend `webview`.

## Raspberry Pi / Linux sans bureau

Il faut **Raspberry Pi OS 64 bits** (Pi 3, 4 ou 5) ou une autre distribution ARM64 ou x86_64. L'installation se fait en une commande :

```sh
curl -fsSL https://raw.githubusercontent.com/Flybrow/AppHA/main/deploy/linux/get.sh | sudo sh -s -- [utilisateur]
```

La commande télécharge la dernière [release](https://github.com/Flybrow/AppHA/releases), installe `cage` (compositeur Wayland mono-application), `cog` et WebKitGTK, crée le service systemd et démarre sur tty1. Ensuite :

```sh
sudo ha-kiosk --config /etc/ha-kiosk/config.toml setup   # assistant (lancé aussi à l'installation)
sudo systemctl start ha-kiosk
journalctl -u ha-kiosk -f             # logs
```

À partir d'un binaire local : `sudo deploy/linux/install.sh ./ha-kiosk [utilisateur]`.

## Paramètres

Avec le backend `webview`, les paramètres s'ouvrent par 5 tapes dans le coin haut-gauche (en moins de 3 s), F10 ou Ctrl+,. On peut aussi les ouvrir avec `ha-kiosk settings`. Le bouton « Quitter le kiosk » arrête le kiosk et le superviseur.

## RAM

Sous Windows, avec un dashboard type, le total est d'environ 220 Mo, dont 6 Mo pour ha-kiosk lui-même ; le reste revient à WebView2. Le GPU est désactivé par défaut (`window.gpu = false`), ce qui économise environ 70 Mo. Pour aller plus loin, allégez le dashboard (cartes historique, caméras).

## Architecture

| Fichier | Rôle |
|---|---|
| `main.rs` | CLI et répartition des commandes |
| `config.rs` | Chargement et validation de la config |
| `supervisor.rs` | Boucle de supervision |
| `health.rs` | Joignabilité de HA |
| `memory.rs` | RAM de l'arbre de process |
| `backend/` | Lancement du navigateur (`external`, `webview`) |
| `auth.rs` | Injection du jeton dans le frontend |
| `paths.rs` | Dossiers système multiplateformes |
| `ui/` | Fenêtre WebView commune, page de chargement, écran de paramètres |
