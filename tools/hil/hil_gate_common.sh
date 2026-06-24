#!/usr/bin/env bash
# Shared helpers for RamenOS HIL Foundry gates (S12/S13).

set -euo pipefail

ramen_sha256_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    echo "unknown"
  fi
}

ramen_export_hil_build_env() {
  local root_dir="$1"
  local init_img="${2:-}"
  local efi_bin="${3:-}"

  export RAMEN_GIT_SHA="$(git -C "$root_dir" rev-parse HEAD 2>/dev/null || echo unknown)"
  export RAMEN_STORAGE_MANIFEST_SHA256="$(
    ramen_sha256_file "$root_dir/hardware/storage_contract_v0.toml"
  )"
  export RAMEN_MACHINE_ID="${RAMEN_HIL_MACHINE_ID:-intel-nuc-12-reference}"

  if [[ -n "$efi_bin" && -f "$efi_bin" ]]; then
    export RAMEN_KERNEL_EFI_SHA256="$(ramen_sha256_file "$efi_bin")"
  else
    export RAMEN_KERNEL_EFI_SHA256="${RAMEN_KERNEL_EFI_SHA256:-unknown}"
  fi

  if [[ -n "$init_img" && -f "$init_img" ]]; then
    export RAMEN_INIT_IMG_SHA256="$(ramen_sha256_file "$init_img")"
  else
    export RAMEN_INIT_IMG_SHA256="${RAMEN_INIT_IMG_SHA256:-unknown}"
  fi
}

ramen_hil_build_kernel_uefi() {
  local root_dir="$1"
  local init_img="$2"

  mkdir -p "$(dirname "$init_img")"
  cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet

  local efi_bin
  efi_bin="$(ramen_find_uefi_bin "$root_dir")"
  ramen_export_hil_build_env "$root_dir" "$init_img" "$efi_bin"
  cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet
  echo "$efi_bin"
}

ramen_find_uefi_bin() {
  local root_dir="$1"
  local base="$root_dir/target/x86_64-unknown-uefi/debug"
  if [[ -f "$base/kernel_uefi.efi" ]]; then
    echo "$base/kernel_uefi.efi"
    return 0
  fi
  if [[ -f "$base/kernel_uefi" ]]; then
    echo "$base/kernel_uefi"
    return 0
  fi
  echo "RAMEN_HIL: UEFI binary missing" >&2
  return 1
}

ramen_hil_resolve_serial_input() {
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
    echo "RAMEN_HIL: graduation mode forbids RAMEN_HIL_SERIAL_LOG" >&2
    return 1
  fi
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && -z "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    echo "RAMEN_HIL: RAMEN_HIL_GRADUATION=1 requires RAMEN_HIL_SERIAL_DEV" >&2
    return 1
  fi
  return 0
}

ramen_hil_assert_provenance_markers() {
  local log="$1"
  grep -q "hil_evidence: git_sha=" "$log" \
    || return 1
  grep -q "hil_evidence: init_profile=" "$log" \
    || return 1
  grep -q "hil_evidence: machine_id=" "$log" \
    || return 1
  grep -q "hil_evidence: storage_manifest_sha256=" "$log" \
    || return 1
  grep -q "hil_evidence: kernel_efi_sha256=" "$log" \
    || return 1
  grep -q "hil_evidence: init_img_sha256=" "$log" \
    || return 1
  grep -q "hil_evidence: boot_epoch_nonce=" "$log" \
    || return 1
  return 0
}

ramen_hil_evidence_level() {
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" ]]; then
    echo "PASS/METAL"
  elif [[ -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    echo "PASS/HIL-LIVE"
  elif [[ -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
    echo "PASS/HIL-LOG"
  else
    echo "PASS/QEMU"
  fi
}

ramen_hil_emit_evidence_json() {
  local out_path="$1"
  local gate_id="$2"
  local evidence_level="$3"
  local serial_log="$4"
  local marker="$5"
  local efi_path="$6"
  local init_path="$7"

  mkdir -p "$(dirname "$out_path")"
  python3 - "$out_path" "$gate_id" "$evidence_level" "$serial_log" "$marker" "$efi_path" "$init_path" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

out_path, gate_id, evidence_level, serial_log, marker, efi_path, init_path = sys.argv[1:8]
root = os.environ.get("ROOT_DIR", ".")

def sha256_file(path: str) -> str:
    import hashlib
    if not path or not os.path.isfile(path):
        return "unknown"
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

payload = {
    "schema_version": 1,
    "gate_id": gate_id,
    "evidence_level": evidence_level,
    "timestamp_utc": datetime.now(timezone.utc).isoformat(),
    "git_sha": os.environ.get("RAMEN_GIT_SHA", "unknown"),
    "machine_id": os.environ.get("RAMEN_MACHINE_ID", "unknown"),
    "storage_manifest_sha256": os.environ.get("RAMEN_STORAGE_MANIFEST_SHA256", "unknown"),
    "kernel_efi_sha256": sha256_file(efi_path),
    "init_img_sha256": sha256_file(init_path),
    "serial_log": serial_log,
    "marker": marker,
    "graduation_mode": os.environ.get("RAMEN_HIL_GRADUATION", "") == "1",
}
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(payload, f, indent=2)
    f.write("\n")
PY
}