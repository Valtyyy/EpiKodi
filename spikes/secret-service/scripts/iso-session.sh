#!/usr/bin/env bash
# Runs a command on a PRIVATE D-Bus session bus, with its own HOME, XDG_DATA_HOME,
# XDG_CONFIG_HOME and XDG_RUNTIME_DIR under target/i/<name>/ (h, d, c, r, b, s).
# The bus has an EMPTY service directory: org.freedesktop.secrets cannot be
# auto-activated, so the only keyring is the one the scenario starts itself.
# The bus dies with the command (dbus-run-session), so no daemon outlives the scenario.
#
# Usage: iso-session.sh <name> -- <command> [args...]
set -euo pipefail

if [[ $# -lt 3 || "$2" != "--" ]]; then
  echo "usage: $0 <name> -- <command> [args...]" >&2
  exit 2
fi

NAME="$1"
shift 2

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Short paths on purpose: the bus socket path must stay under the 108-byte Unix limit.
ISO="$ROOT/target/i/$NAME"

rm -rf "$ISO"
mkdir -p "$ISO/h" "$ISO/d" "$ISO/c" "$ISO/r" "$ISO/b" "$ISO/s"
chmod 700 "$ISO/r"

cat > "$ISO/bus.conf" <<CONF
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir=$ISO/b</listen>
  <auth>EXTERNAL</auth>
  <servicedir>$ISO/s</servicedir>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
CONF

export HOME="$ISO/h"
export XDG_DATA_HOME="$ISO/d"
export XDG_CONFIG_HOME="$ISO/c"
export XDG_RUNTIME_DIR="$ISO/r"
export XDG_CURRENT_DESKTOP=GNOME
unset DBUS_SESSION_BUS_ADDRESS GNOME_KEYRING_CONTROL SSH_AUTH_SOCK

exec dbus-run-session --config-file="$ISO/bus.conf" -- "$@"
