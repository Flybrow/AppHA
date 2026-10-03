#!/bin/sh
# Downloads the latest ha-kiosk release and installs it as a kiosk.
# Usage: curl -fsSL https://raw.githubusercontent.com/Flybrow/AppHA/main/deploy/linux/get.sh | sudo sh -s -- [user]
set -eu

REPO=Flybrow/AppHA
case "$(uname -m)" in
    aarch64|arm64) ARCH=aarch64 ;;
    x86_64|amd64) ARCH=x86_64 ;;
    *) echo "Architecture $(uname -m) not supported (Raspberry Pi: install Raspberry Pi OS 64-bit)." >&2; exit 1 ;;
esac

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
URL="https://github.com/$REPO/releases/latest/download/ha-kiosk-linux-$ARCH.tar.gz"
echo "==> Downloading $URL"
curl -fsSL "$URL" | tar -xz -C "$TMP"
sh "$TMP/ha-kiosk/deploy/linux/install.sh" "$TMP/ha-kiosk/ha-kiosk" "$@"
