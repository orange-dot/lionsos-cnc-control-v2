#!/bin/bash

set -euo pipefail

RULE_CANDIDATES=(
    "/etc/udev/rules.d/70-owon-vds1022.rules"
    "/etc/udev/rules.d/70-owon-vds-tiny.rules"
    "/usr/lib/udev/rules.d/70-owon-vds1022.rules"
    "/usr/lib/udev/rules.d/70-owon-vds-tiny.rules"
)

found=0

for rule in "${RULE_CANDIDATES[@]}"; do
    if [[ -f "$rule" ]]; then
        echo "Found: $rule"
        grep -n '5345\|1234' "$rule" || true
        found=1
        echo
    fi
done

if [[ "$found" -eq 0 ]]; then
    echo "No OWON udev rule found in the common system rule locations."
fi

