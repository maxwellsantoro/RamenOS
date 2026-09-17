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

EFIVAR_DIR="${RAMEN_HIL_EFIVAR_DIR:-/sys/firmware/efi/efivars}"
GUID="a3b8c14e-5f20-4d71-9e62-1308ab080000"
NAME="RamenBootNonce-${GUID}"
EFIVAR_PATH="${EFIVAR_DIR}/${NAME}"

if [[ ! -d "$EFIVAR_DIR" ]]; then
  fail "EFIVARFS_MISSING" "efivarfs not mounted at ${EFIVAR_DIR}"
fi

if [[ -n "${1:-}" ]]; then
  NONCE_HEX="$1"
else
  NONCE_HEX="$(python3 - <<'PY'
import secrets
print(f"{secrets.randbelow((1 << 64) - 1) + 1:016x}")
PY
)"
fi

# Validate before removing an existing variable.
python3 - "$NONCE_HEX" <<'PYVALID'
import re, sys
text = sys.argv[1]
if not re.fullmatch(r"[0-9a-fA-F]{1,16}", text) or int(text, 16) == 0:
    raise SystemExit("nonce must be a nonzero 64-bit hexadecimal value")
PYVALID

if [[ -e "$EFIVAR_PATH" ]]; then
  chattr -i "$EFIVAR_PATH" 2>/dev/null || true
  rm -f "$EFIVAR_PATH"
fi

python3 - "$EFIVAR_PATH" "$NONCE_HEX" <<'PYWRITE'
import sys
payload = (7).to_bytes(4, "little") + int(sys.argv[2], 16).to_bytes(8, "little")
with open(sys.argv[1], "wb") as output:
    if output.write(payload) != len(payload):
        raise OSError("short efivar write")
    output.flush()
    # efivarfs performs the firmware write synchronously and has no fsync operation.
PYWRITE

echo "SET_RAMENOS_BOOT_NONCE: METRIC boot_epoch_nonce=${NONCE_HEX}"
echo "SET_RAMENOS_BOOT_NONCE: ok"