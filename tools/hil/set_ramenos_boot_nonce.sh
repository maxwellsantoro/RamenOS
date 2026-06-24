#!/usr/bin/env bash
# Stage RamenBootNonce UEFI variable for HIL graduation evidence.
#
# Usage: set_ramenos_boot_nonce.sh [hex_nonce]
# Default: random 64-bit nonce.

set -euo pipefail

fail() {
  echo "SET_RAMENOS_BOOT_NONCE: FAIL code=$1 detail=$2" >&2
  exit 1
}

EFIVAR_DIR="/sys/firmware/efi/efivars"
GUID="a3b8c14e-5f20-4d71-9e62-1308ab080000"
NAME="RamenBootNonce-${GUID}"
PATH="${EFIVAR_DIR}/${NAME}"

if [[ ! -d "$EFIVAR_DIR" ]]; then
  fail "EFIVARFS_MISSING" "efivarfs not mounted at ${EFIVAR_DIR}"
fi

if [[ -n "${1:-}" ]]; then
  NONCE_HEX="$1"
else
  NONCE_HEX="$(python3 - <<'PY'
import secrets
print(f"{secrets.randbits(64):016x}")
PY
)"
fi

NONCE_LE="$(python3 - "$NONCE_HEX" <<'PY'
import sys
value = int(sys.argv[1], 16)
print("".join(f"\\x{b:02x}" for b in value.to_bytes(8, "little")))
PY
)"

if [[ -e "$PATH" ]]; then
  chattr -i "$PATH" 2>/dev/null || true
  rm -f "$PATH"
fi

ATTRS=$((0x00000001 | 0x00000002 | 0x00000004))
printf "\\x%02x\\x%02x\\x%02x\\x%02x%s" \
  $((ATTRS & 0xff)) $(((ATTRS >> 8) & 0xff)) $(((ATTRS >> 16) & 0xff)) $(((ATTRS >> 24) & 0xff)) \
  "$NONCE_LE" >"$PATH"

echo "SET_RAMENOS_BOOT_NONCE: METRIC boot_epoch_nonce=${NONCE_HEX}"
echo "SET_RAMENOS_BOOT_NONCE: ok"