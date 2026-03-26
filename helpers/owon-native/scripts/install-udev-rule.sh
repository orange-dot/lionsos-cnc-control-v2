#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RULE_SRC="$SCRIPT_DIR/../udev/70-owon-vds1022.rules"
RULE_DST="/etc/udev/rules.d/70-owon-vds1022.rules"

if [[ "$(id -u)" -ne 0 ]]; then
    echo "Run this script as root." >&2
    echo "Example: sudo bash scripts/install-udev-rule.sh" >&2
    exit 1
fi

if [[ ! -f "$RULE_SRC" ]]; then
    echo "Missing rule source: $RULE_SRC" >&2
    exit 1
fi

install -D -m 0644 "$RULE_SRC" "$RULE_DST"

if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules
    udevadm trigger --subsystem-match=usb --attr-match=idVendor=5345 --attr-match=idProduct=1234 || true
fi

echo "Installed udev rule at $RULE_DST"
echo "Unplug and replug the scope, then rerun:"
echo "  cargo run -p owon-probe -- open"
