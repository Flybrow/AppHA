#!/bin/sh
# Installs ha-kiosk as a kiosk on a Linux distribution without a desktop.
# Usage: sudo ./install.sh <ha-kiosk binary> [user]
set -eu

BIN=${1:?usage: sudo ./install.sh <ha-kiosk binary> [user]}
KIOSK_USER=${2:-${SUDO_USER:-kiosk}}
HERE=$(dirname "$0")

install_pkgs() {
    if command -v apt-get >/dev/null; then
        # libwebkit2gtk: required by the binary (settings screen, webview backend).
        apt-get update && apt-get install -y cage libwebkit2gtk-4.1-0 "$@" || apt-get install -y cage libwebkit2gtk-4.1-0
    elif command -v dnf >/dev/null; then dnf install -y cage "$@"
    elif command -v pacman >/dev/null; then pacman -S --needed --noconfirm cage "$@"
    elif command -v apk >/dev/null; then apk add cage "$@"
    elif command -v zypper >/dev/null; then zypper install -y cage "$@"
    else echo "Unknown package manager: install cage, cog and WebKitGTK manually." >&2
    fi
}

echo "==> Packages (cage, WebKitGTK, cog, wlr-randr)"
install_pkgs cog wlr-randr

id "$KIOSK_USER" >/dev/null 2>&1 || useradd -m -G video,input,render "$KIOSK_USER"
usermod -aG video,input "$KIOSK_USER" || true

echo "==> Binary and configuration"
# /opt/ha-kiosk belongs to the kiosk user: it can update itself (no root).
install -d -o "$KIOSK_USER" -g "$(id -gn "$KIOSK_USER")" -m 0755 /opt/ha-kiosk
install -m 0755 -o "$KIOSK_USER" "$BIN" /opt/ha-kiosk/ha-kiosk
rm -f /usr/local/bin/ha-kiosk
ln -s /opt/ha-kiosk/ha-kiosk /usr/local/bin/ha-kiosk
# Directory owned by the kiosk user: atomic saves create a temporary file there.
install -d -o "$KIOSK_USER" -g "$(id -gn "$KIOSK_USER")" -m 0700 /etc/ha-kiosk
CONF=/etc/ha-kiosk/config.toml
if [ ! -f "$CONF" ]; then
    # Interactive wizard when a terminal is available (including via curl | sh).
    if [ -r /dev/tty ] && [ -w /dev/tty ]; then
        echo "==> Configuration wizard"
        /usr/local/bin/ha-kiosk --config "$CONF" setup </dev/tty >/dev/tty || true
    fi
    [ -f "$CONF" ] || install -m 0600 "$HERE/../../config.example.toml" "$CONF"
fi
# Writable by the kiosk user (settings screen), unreadable by others (token).
chown "$KIOSK_USER:$(id -gn "$KIOSK_USER")" "$CONF"
chmod 0600 "$CONF"

install -d -m 0755 /usr/local/lib/ha-kiosk
install -m 0755 -o root -g root "$HERE/touch-rotation.sh" /usr/local/lib/ha-kiosk/touch-rotation.sh

echo "==> systemd service"
sed -e "s/%KIOSK_USER%/$KIOSK_USER/" -e "s/%KIOSK_UID%/$(id -u "$KIOSK_USER")/"     "$HERE/ha-kiosk.service" > /etc/systemd/system/ha-kiosk.service
systemctl disable getty@tty1.service 2>/dev/null || true
# Old update timer (root): replaced by the supervisor's own updates.
systemctl disable --now ha-kiosk-update.timer 2>/dev/null || true
rm -f /etc/systemd/system/ha-kiosk-update.service /etc/systemd/system/ha-kiosk-update.timer
systemctl daemon-reload
systemctl enable ha-kiosk.service

echo "Done. Start: sudo systemctl start ha-kiosk (or reboot)"
echo "Reconfigure: ha-kiosk setup"
echo "Logs: journalctl -t ha-kiosk -f"
echo "Updates: automatic every 6 h, or manually: ha-kiosk update"
