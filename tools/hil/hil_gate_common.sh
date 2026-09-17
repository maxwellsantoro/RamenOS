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

  export RAMEN_GIT_SHA="$(git -C "$root_dir" rev-parse HEAD 2>/dev/null || echo unknown)"
  export RAMEN_STORAGE_MANIFEST_SHA256="$(
    ramen_sha256_file "$root_dir/hardware/storage_contract_v0.toml"
  )"
  export RAMEN_MACHINE_ID="${RAMEN_HIL_MACHINE_ID:-lenovo-thinkcentre-m900-i7-6700-lab-01}"

  export RAMEN_KERNEL_BUILD_ID="$(python3 -c 'import secrets; print(secrets.token_hex(32))')"

  if [[ -n "$init_img" && -f "$init_img" ]]; then
    export RAMEN_INIT_IMG_SHA256="$(ramen_sha256_file "$init_img")"
  else
    export RAMEN_INIT_IMG_SHA256="${RAMEN_INIT_IMG_SHA256:-unknown}"
  fi
}

ramen_hil_build_kernel_uefi() {
  local root_dir="$1"
  local init_img="$2"

  local profile="${3:-$(basename "$init_img" .img)}"
  profile="${profile#init_}"
  [[ -f "$init_img" ]] || return 1
  ramen_export_hil_build_env "$root_dir" "$init_img"
  cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet
  local efi_bin
  efi_bin="$(ramen_find_uefi_bin "$root_dir")"
  python3 "$root_dir/tools/hil/provenance.py" build "$efi_bin.provenance.json" "$efi_bin" "$init_img" "$profile"
  echo "$efi_bin"
}

ramen_find_uefi_bin() {
  local root_dir="$1"
  local base="${CARGO_TARGET_DIR:-$root_dir/target}/x86_64-unknown-uefi/debug"
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

# Select a previously built/deployed image without regenerating its identity.
ramen_hil_load_prepared_build() {
  local profile="$1"
  : "${RAMEN_HIL_EXPECTED_BUILD:?prepared manifest required}"
  local tool="$(dirname "${BASH_SOURCE[0]}")/provenance.py"
  RAMEN_HIL_PREPARED_EFI="$(python3 "$tool" prepared "$RAMEN_HIL_EXPECTED_BUILD" "$profile" kernel_efi)" || return 1
  RAMEN_HIL_PREPARED_INIT="$(python3 "$tool" prepared "$RAMEN_HIL_EXPECTED_BUILD" "$profile" init_img)" || return 1
  export RAMEN_HIL_PREPARED_EFI RAMEN_HIL_PREPARED_INIT
}

ramen_hil_resolve_serial_input() {
  if [[ -n "${RAMEN_HIL_SERIAL_LOG:-}" && -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    echo "RAMEN_HIL: serial input is ambiguous" >&2
    return 1
  fi
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
    echo "RAMEN_HIL: graduation mode forbids RAMEN_HIL_SERIAL_LOG" >&2
    return 1
  fi
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && -z "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    echo "RAMEN_HIL: RAMEN_HIL_GRADUATION=1 requires RAMEN_HIL_SERIAL_DEV" >&2
    return 1
  fi
  if [[ -n "${RAMEN_HIL_SERIAL_DEV:-}" && ! -c "$RAMEN_HIL_SERIAL_DEV" ]]; then
    echo "RAMEN_HIL: live input must be a character device" >&2
    return 1
  fi
  return 0
}

ramen_hil_assert_provenance_markers() {
  python3 "$(dirname "${BASH_SOURCE[0]}")/provenance.py" verify "$1"
}

# Capture once through the appliance observer so per-gate and controller records
# refer to exactly the same bytes. Never recapture an independent transcript.
ramen_hil_capture_appliance() {
  local dev="$1" log="$2" timeout_s="$3"
  : "${RAMEN_HIL_RUN_ID:?set a unique RAMEN_HIL_RUN_ID}"
  : "${RAMEN_HIL_APPLIANCE_ID:?set RAMEN_HIL_APPLIANCE_ID}"
  export RAMEN_HIL_RUN_ID="${RAMEN_HIL_RUN_ID}_${RAMEN_HIL_CAPTURE_GATE:?gate id required}"
  local evidence_dir="${RAMEN_HIL_EVIDENCE_DIR:-$ROOT_DIR/out/evidence}"
  RAMEN_HIL_SERIAL_DEV="$dev" RAMEN_HIL_CAPTURE_TIMEOUT_S="$timeout_s" \
    RAMEN_HIL_EVIDENCE_DIR="$evidence_dir" \
    bash "$ROOT_DIR/tools/hil/appliance_capture_serial.sh"
  export RAMEN_HIL_CONTROLLER_EVIDENCE="$evidence_dir/$RAMEN_HIL_RUN_ID.json"
  cp "$evidence_dir/$RAMEN_HIL_RUN_ID.serial.log" "$log"
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

ramen_hil_claim_path() {
  case "${1:-}" in
    PASS/QEMU) echo "qemu-or-scaffold"; return ;;
    PASS/HIL-LOG) echo "development-log-replay"; return ;;
  esac
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && "${RAMEN_HIL_APPLIANCE:-}" == "1" ]]; then
    echo "appliance-mediated"
  elif [[ "${RAMEN_HIL_GRADUATION:-}" == "1" ]]; then
    echo "operator-golden-machine"
  elif [[ "${RAMEN_HIL_APPLIANCE:-}" == "1" && -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    echo "appliance-live"
  elif [[ -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    echo "operator-live"
  elif [[ -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
    echo "development-log-replay"
  else
    echo "qemu-or-scaffold"
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

  local claim_path
  claim_path="$(ramen_hil_claim_path "$evidence_level")"

  python3 "$(dirname "${BASH_SOURCE[0]}")/provenance.py" emit \
    "$out_path" "$gate_id" "$evidence_level" "$serial_log" "$marker" "$efi_path" "$init_path" "$claim_path"
}
