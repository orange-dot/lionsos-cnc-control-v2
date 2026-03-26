#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NATIVE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

PREFIX="${1:-rpi-uart-boot}"
CHANNEL="${2:-ch1}"
BAUD="${3:-115200}"

cd "$NATIVE_DIR"

echo "Starting OWON capture for Raspberry Pi UART bench test..."
echo "  Prefix:  $PREFIX"
echo "  Channel: $CHANNEL"
echo "  Baud:    $BAUD"
echo

OWON_CAPTURE_TRACE=1 cargo run -p owon-probe -- uart-bench-capture "$PREFIX" "$CHANNEL"

echo
echo "Decoding captured waveform..."
cargo run -p owon-probe -- decode-uart "${PREFIX}-meta.json" "$CHANNEL" "$BAUD"

echo
echo "Done."
echo "Artifacts:"
echo "  ${PREFIX}-meta.json"
echo "  ${PREFIX}-debug.json"
echo "  ${PREFIX}-${CHANNEL}.bin"
