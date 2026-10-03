#!/bin/sh
# Télécharge la dernière version de ha-kiosk et l'installe en kiosk.
# Usage : curl -fsSL https://raw.githubusercontent.com/Flybrow/AppHA/main/deploy/linux/get.sh | sudo sh -s -- [utilisateur]
set -eu

REPO=Flybrow/AppHA
case "$(uname -m)" in
    aarch64|arm64) ARCH=aarch64 ;;
    x86_64|amd64) ARCH=x86_64 ;;
    *) echo "Architecture $(uname -m) non prise en charge (Raspberry Pi : installez Raspberry Pi OS 64 bits)." >&2; exit 1 ;;
esac

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
URL="https://github.com/$REPO/releases/latest/download/ha-kiosk-linux-$ARCH.tar.gz"
echo "==> Téléchargement de $URL"
curl -fsSL "$URL" | tar -xz -C "$TMP"
sh "$TMP/ha-kiosk/deploy/linux/install.sh" "$TMP/ha-kiosk/ha-kiosk" "$@"
