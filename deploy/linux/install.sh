#!/bin/sh
# Installe ha-kiosk en mode kiosk sur une distribution Linux sans bureau.
# Usage : sudo ./install.sh <binaire ha-kiosk> [utilisateur]
set -eu

BIN=${1:?usage: sudo ./install.sh <binaire ha-kiosk> [utilisateur]}
KIOSK_USER=${2:-${SUDO_USER:-kiosk}}
HERE=$(dirname "$0")

install_pkgs() {
    if command -v apt-get >/dev/null; then
        # libwebkit2gtk : requis par le binaire (écran de paramètres, backend webview).
        apt-get update && apt-get install -y cage libwebkit2gtk-4.1-0 "$@" || apt-get install -y cage libwebkit2gtk-4.1-0
    elif command -v dnf >/dev/null; then dnf install -y cage "$@"
    elif command -v pacman >/dev/null; then pacman -S --needed --noconfirm cage "$@"
    elif command -v apk >/dev/null; then apk add cage "$@"
    elif command -v zypper >/dev/null; then zypper install -y cage "$@"
    else echo "Gestionnaire de paquets inconnu : installez cage et cog manuellement." >&2
    fi
}

echo "==> Paquets (cage + cog/WPE WebKit)"
install_pkgs cog

id "$KIOSK_USER" >/dev/null 2>&1 || useradd -m -G video,input,render "$KIOSK_USER"
usermod -aG video,input "$KIOSK_USER" || true

echo "==> Binaire et configuration"
install -m 0755 "$BIN" /usr/local/bin/ha-kiosk
install -d /etc/ha-kiosk
[ -f /etc/ha-kiosk/config.toml ] || install -m 0640 -g "$KIOSK_USER" "$HERE/../../config.example.toml" /etc/ha-kiosk/config.toml

echo "==> Service systemd"
sed -e "s/%KIOSK_USER%/$KIOSK_USER/" -e "s/%KIOSK_UID%/$(id -u "$KIOSK_USER")/" \
    "$HERE/ha-kiosk.service" > /etc/systemd/system/ha-kiosk.service
systemctl disable getty@tty1.service 2>/dev/null || true
install -m 0644 "$HERE/ha-kiosk-update.service" "$HERE/ha-kiosk-update.timer" /etc/systemd/system/
systemctl daemon-reload
systemctl enable ha-kiosk.service
systemctl enable --now ha-kiosk-update.timer

echo "Terminé. Éditez /etc/ha-kiosk/config.toml puis : sudo systemctl start ha-kiosk"
echo "Logs : journalctl -u ha-kiosk -f"
echo "Mises à jour : quotidiennes (ha-kiosk-update.timer), ou à la main : sudo ha-kiosk update"
