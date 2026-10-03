# AppHA — kiosk Home Assistant

Affiche votre dashboard Home Assistant en plein écran, en continu, sur un Raspberry Pi, un PC Windows ou Linux. Il reste léger et se rétablit tout seul.

- **Connexion automatique** avec un jeton longue durée : aucun écran de connexion.
- **Toujours affiché** : relance après un crash, après une coupure de Home Assistant ou du réseau, et redémarrage préventif planifié.
- **Léger** : animations désactivées par défaut sous Linux, fluide même sur un Raspberry Pi 3. Le kiosk lui-même consomme quelques Mo.
- **Mises à jour automatiques** depuis GitHub.
- Compatible HTTP, HTTPS (certificat auto-signé compris), IP locale, nom de domaine et Nabu Casa.

---

## Avant de commencer : créer un jeton

Dans Home Assistant, ouvrez **votre profil** (en bas à gauche), puis l'onglet **Sécurité**, puis **Jetons d'accès longue durée** et **Créer un jeton**. Copiez-le : il ne sera plus affiché ensuite.

---

## Raspberry Pi

Fonctionne sur Pi 3, 4 et 5, avec **Raspberry Pi OS 64 bits** (la version Lite suffit). Pour vérifier : `dpkg --print-architecture` doit afficher `arm64`.

### Installation

Dans un terminal sur le Pi, ou en SSH :

```sh
curl -fsSL https://raw.githubusercontent.com/Flybrow/AppHA/main/deploy/linux/get.sh | sudo sh -s -- $USER
```

L'installation dure de 5 à 20 minutes sur un Pi 3. Un **assistant** pose ensuite les questions une par une (rotation de l'écran, adresse de Home Assistant, jeton…). Il suffit d'appuyer sur Entrée pour garder la valeur proposée.

Ensuite :

```sh
sudo raspi-config nonint do_boot_behaviour B1   # démarrage sans bureau (libère de la RAM)
sudo reboot
```

Le Pi démarre alors directement sur le dashboard.

> L'écran est à l'envers ? Lancez `sudo ha-kiosk config rotation 180`, puis `sudo systemctl restart ha-kiosk`. L'écran officiel 7" monté dans certains boîtiers a besoin de 180.

### Changer un réglage

```sh
sudo ha-kiosk config                  # affiche tous les réglages
sudo ha-kiosk config token            # change le jeton (question posée)
sudo ha-kiosk config rotation 180     # change directement une valeur
sudo ha-kiosk setup                   # refait l'assistant complet
sudo systemctl restart ha-kiosk       # applique les changements
```

Tapez `ha-kiosk` seul pour afficher l'aide complète.

Avec un clavier ou un écran tactile, l'**écran de paramètres** s'ouvre aussi directement sur le kiosk : **5 tapes rapides dans le coin haut-gauche**, ou **F10**.

### Commandes utiles

| Besoin | Commande |
|---|---|
| Redémarrer le kiosk | `sudo systemctl restart ha-kiosk` |
| Arrêter le kiosk | `sudo systemctl stop ha-kiosk` |
| Mettre à jour maintenant | `sudo ha-kiosk update` |
| Voir les journaux | `journalctl -b \| grep ha-kiosk` |
| Tester la connexion à HA | `sudo ha-kiosk check` |

### Revenir au bureau

```sh
sudo systemctl disable --now ha-kiosk ha-kiosk-update.timer
sudo systemctl enable getty@tty1
sudo raspi-config nonint do_boot_behaviour B4
sudo reboot
```

---

## Windows

Windows 10 ou 11 (64 bits).

1. Téléchargez **`ha-kiosk-windows-x86_64.zip`** depuis la [dernière version](https://github.com/Flybrow/AppHA/releases/latest).
2. Décompressez-le, par exemple dans `C:\AppHA`.
3. Double-cliquez sur **`ha-kiosk.exe`**. Si Windows affiche « Windows a protégé votre ordinateur », cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.
4. L'**écran de paramètres** s'ouvre : saisissez l'adresse de Home Assistant et le jeton, puis cliquez sur **Enregistrer et lancer**.

**Pendant l'utilisation :**
- **Paramètres** : F10, Ctrl+, ou 5 tapes rapides dans le coin haut-gauche.
- **Quitter** : depuis les paramètres, bouton **Quitter le kiosk**.
- **Mode d'affichage** : plein écran, fenêtré ou fenêtré sans bordure, au choix dans les paramètres.

**Lancement au démarrage de Windows :** appuyez sur Win+R, tapez `shell:startup`, puis placez dans ce dossier un raccourci vers `ha-kiosk.exe`.

Les mises à jour s'installent toutes seules. Vous pouvez aussi lancer la recherche à la main dans les paramètres (**Rechercher une mise à jour**).

---

## Linux avec bureau (PC, Pi avec bureau)

Téléchargez l'archive correspondant à votre machine depuis la [dernière version](https://github.com/Flybrow/AppHA/releases/latest) : `linux-aarch64` pour un Raspberry Pi, `linux-x86_64` pour un PC. Ensuite :

```sh
sudo apt install libwebkit2gtk-4.1-0
tar xzf ha-kiosk-linux-*.tar.gz
./ha-kiosk/ha-kiosk run
```

Au premier lancement, l'écran de paramètres s'ouvre, comme sous Windows. Pour un kiosk dédié, préférez l'installation [Raspberry Pi](#raspberry-pi) : elle fonctionne aussi sur un PC Linux.

---

## Réglages

Ils sont disponibles dans l'écran de paramètres, dans l'assistant (`ha-kiosk setup`) ou avec `ha-kiosk config <réglage> <valeur>`.

| Réglage | Rôle |
|---|---|
| `url` | Adresse de Home Assistant, par ex. `https://192.168.1.10:8123` |
| `dashboard` | Dashboard à afficher, par ex. `lovelace-kiosk/0` (vide = dashboard par défaut) |
| `token` | Jeton d'accès longue durée |
| `insecure_tls` | Accepter un certificat HTTPS auto-signé |
| `mode` | `fullscreen`, `windowed` ou `borderless` |
| `rotation` | `0`, `90`, `180` ou `270` (Linux) |
| `animations` | `non` = bien plus fluide sur les petites machines (défaut sous Linux) |
| `gpu` | Accélération matérielle : environ 70 Mo de RAM en plus, inutile sur Pi 3 |
| `restart_every_hours` | Redémarrage préventif (`0` = jamais) |
| `max_memory_mb` | Redémarre le navigateur au-delà de cette RAM (`0` = pas de limite) |
| `auto_update` | Mise à jour automatique |
| `browser` | `auto` (recommandé), `webview` ou `external` (Chromium, cog…) |

---

## Questions fréquentes

**Écran noir avec seulement le curseur (Linux)**
Mettez à jour (`sudo ha-kiosk update`) et réinstallez le service avec la commande d'installation : les versions récentes corrigent ce problème.

**L'affichage est lent sur Raspberry Pi**
Vérifiez que `animations` vaut `non`. Un Pi 3 bride sa fréquence au-delà de 60 °C : un dissipateur ou un petit ventilateur aide. Une alimentation trop faible (sous-tension) le ralentit aussi : `vcgencmd get_throttled` doit afficher `0x0`.

**Je ne suis pas connecté automatiquement**
Le jeton n'est utilisé qu'avec `browser = auto` ou `webview`, et avec Chromium. `cog` et Firefox ne permettent pas la connexion automatique.

---

Licence MIT.
