#!/bin/sh
# Rotates the touchscreen like the display (`rotation` in the configuration):
# cage/wlroots rotates the image but not the touch input, so a udev rule sets the
# libinput calibration matrix. Run as root before cage starts (ExecStartPre).
# Owned by root on purpose: the kiosk user can replace its own binary.
CONF=${1:-/etc/ha-kiosk/config.toml}
RULE=/etc/udev/rules.d/99-ha-kiosk-touch.rules

ROT=$(sed -n 's/^[[:space:]]*rotation[[:space:]]*=[[:space:]]*\([0-9]*\).*/\1/p' "$CONF" 2>/dev/null | head -n 1)
case "$ROT" in
    90)  MATRIX="0 -1 1 1 0 0" ;;
    180) MATRIX="-1 0 1 0 -1 1" ;;
    270) MATRIX="0 1 0 -1 0 1" ;;
    *)   MATRIX="" ;;
esac

if [ -n "$MATRIX" ]; then
    WANT="ENV{ID_INPUT_TOUCHSCREEN}==\"1\", ENV{LIBINPUT_CALIBRATION_MATRIX}=\"$MATRIX\""
    [ "$(cat "$RULE" 2>/dev/null)" = "$WANT" ] && exit 0
    echo "$WANT" > "$RULE"
else
    [ -f "$RULE" ] || exit 0
    rm -f "$RULE"
fi
udevadm control --reload
udevadm trigger --subsystem-match=input --action=change
udevadm settle --timeout=5 || true
exit 0
