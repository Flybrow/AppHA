# Architecture et développement

Ce document est destiné aux contributeurs. Le [README](../README.md) s'adresse aux utilisateurs.

## Process

```
cage (Linux)                         ← compositeur Wayland, lancé par systemd
└─ ha-kiosk run                      ← superviseur (~2 Mo) : surveille, relance, met à jour
   ├─ ha-kiosk webview               ← fenêtre + WebView (WebView2 / WebKitGTK)
   │  └─ moteur de rendu, réseau…    ← process du navigateur
   └─ ha-kiosk settings              ← écran de paramètres, à la demande
```

- Le navigateur tourne toujours dans un **process enfant** : un crash ou une fuite mémoire n'atteint pas le superviseur, qui relance l'enfant.
- L'enfant communique avec le superviseur par son **code de sortie** : `4` = ouvrir les paramètres. L'écran de paramètres utilise `0` = enregistré, `3` = annulé, `5` = quitter le kiosk, `6` = mis à jour.
- Sous Linux, le superviseur s'arrête quand `cage` s'arrête (`PR_SET_PDEATHSIG`, et test `wl_display.sync` quand le navigateur meurt). Sinon, `cage` attend son enfant indéfiniment.

## Modules (`src/`)

| Module | Rôle |
|---|---|
| `main.rs` | Ligne de commande et répartition des commandes |
| `config.rs` | Lecture, validation et enregistrement atomique de `config.toml`. `Config::commit` est le point de passage unique des modifications (validation, enregistrement, démarrage automatique). |
| `supervisor.rs` | Boucle de supervision et mise à jour en arrière-plan |
| `backend/` | Lancement du navigateur : `webview` (intégré) ou `external` (commande) |
| `ui/` | Fenêtre WebView commune, page de chargement, écran de paramètres, scripts injectés |
| `cli/` | Commandes texte : `config`, `setup` (assistant), `check`, `update` ; table des réglages partagée |
| `auth.rs` | Connexion automatique : jeton placé dans le `localStorage` du frontend HA |
| `update.rs` | Mise à jour depuis les releases GitHub (via `curl` et `tar`) |
| `autostart.rs` | Démarrage avec la machine : clé `Run` (Windows), service systemd (Linux) |
| `process.rs` | Lancement de commandes auxiliaires sans fenêtre de console |
| `display.rs` | Rotation (`wlr-randr`) et détection de l'arrêt du compositeur |
| `health.rs` | Joignabilité de HA (connexion TCP, sans TLS) |
| `memory.rs` | RAM de l'arbre de process du navigateur |
| `console.rs` | Console et boîte d'erreur sous Windows (exécutable sans console) |
| `paths.rs` | Dossiers système multiplateformes |

### Scripts injectés dans les pages (`src/ui/assets/`)

| Fichier | Rôle |
|---|---|
| `gesture.js` | 5 tapes dans le coin haut-gauche, F10 ou Ctrl+, : ouvre les paramètres |
| `inject-css.js` | Style appliqué au document **et à chaque shadow root** (animations à 1 ms, curseur masqué) |
| `debug-animations.js` | Diagnostic, actif seulement avec `HA_KIOSK_DEBUG_ANIMATIONS=1` |
| `loading.html` | Page d'attente locale ; le test de joignabilité est fait côté Rust |
| `settings.html` | Écran de paramètres |

## Choix techniques

- **Animations à 1 ms plutôt que supprimées** : sur un Raspberry Pi 3, les animations du frontend HA faisaient monter le processeur à 130 % en continu. Le passage à 1 ms le ramène à 5 %. Les supprimer complètement casserait les cartes qui attendent `transitionend` (popups Bubble Card).
- **GPU désactivé par défaut** : environ 70 Mo économisés sous Windows. Sur un Pi 3, WebKit ne peut de toute façon pas faire le rendu sur ce GPU.
- **Mise à jour par `curl` et `tar`** : aucune pile TLS embarquée, et aucune RAM utilisée entre deux vérifications.
- **Installation Linux** : binaire dans `/opt/ha-kiosk` et config `0600`, tous deux propriété de l'utilisateur du kiosk. Ainsi, l'écran de paramètres et la mise à jour fonctionnent sans root.
- **Bac à sable WebKit désactivé** (service systemd) : bubblewrap échoue sous ce service et provoque un écran noir. Le kiosk n'affiche que Home Assistant.

## Compiler et tester

```sh
cargo test
cargo build --release
```

Sous Linux, il faut installer `libwebkit2gtk-4.1-dev` et `libgtk-3-dev`.

Diagnostic des animations : `HA_KIOSK_DEBUG_ANIMATIONS=1 ha-kiosk run`, puis lire les lignes `animations {…}` du journal.

## Publier une version

1. Mettez à jour `version` dans `Cargo.toml`.
2. Poussez sur `main` : la CI lance les tests et construit les binaires, qui sont disponibles en artefacts pour tester.
3. Après validation, créez le tag `vX.Y.Z` et poussez-le. La CI vérifie que le tag correspond à `Cargo.toml` et publie la release.

Attention : les kiosks installés se mettent à jour seuls depuis la dernière release. Un tag part donc directement en production.
