#!/usr/bin/env bash
# Stage RamenAbSlot UEFI variable for S13.8 metal graduation (Linux efivarfs).
#
# Usage: set_ramenos_ab_slot.sh <A|B>
#
# Payload v1: schema=1, active_slot(0=A|1=B), rollback_ready=1

set -euo pipefail

fail() {
  echo "SET_RAMENOS_AB_SLOT: FAIL code=$1 detail=$2" >&2
  exit 1
}

SLOT="${1:-}"
case "$SLOT" in
  A|a) ACTIVE=0 ;;
  B|b) ACTIVE=1 ;;
  *)
    fail "INVALID_SLOT" "usage: set_ramenos_ab_slot.sh <A|B>"
    ;;
esac

EFIVAR_DIR="${RAMEN_HIL_EFIVAR_DIR:-/sys/firmware/efi/efivars}"
GUID="a3b8c14e-5f20-4d71-9e62-1308ab080000"
NAME="RamenAbSlot-${GUID}"
EFIVAR_PATH="${EFIVAR_DIR}/${NAME}"

if [[ ! -d "$EFIVAR_DIR" ]]; then
  fail "EFIVARFS_MISSING" "efivarfs not mounted at ${EFIVAR_DIR} (Linux firmware interface required)"
fi

if [[ -e "$EFIVAR_PATH" ]]; then
  chattr -i "$EFIVAR_PATH" 2>/dev/null || true
  rm -f "$EFIVAR_PATH"
fi

# EFI_VARIABLE_NON_VOLATILE | EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS
python3 - "$EFIVAR_PATH" "$ACTIVE" <<'PYWRITE'
import sys
payload = (7).to_bytes(4, "little") + bytes([1, int(sys.argv[2]), 1])
with open(sys.argv[1], "wb") as output:
    if output.write(payload) != len(payload):
        raise OSError("short efivar write")
    output.flush()
    # efivarfs performs the firmware write synchronously and has no fsync operation.
PYWRITE

echo "SET_RAMENOS_AB_SLOT: METRIC active_slot=${SLOT}"
echo "SET_RAMENOS_AB_SLOT: METRIC rollback_ready=1"
echo "SET_RAMENOS_AB_SLOT: ok"