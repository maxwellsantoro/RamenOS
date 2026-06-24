#!/usr/bin/env bash
# Boot the Linux Oracle capsule with virtio-net-pci and capture harness packet JSONL.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="$ROOT_DIR/out/oracle_capture"
SERIAL_LOG="$OUT_DIR/packet_serial.log"
JSONL_OUT="${1:-$OUT_DIR/virtio_net_packet_oracle_events.jsonl}"
PROMOTE="${PROMOTE_CAPTURE:-1}"
QEMU_BIN="${QEMU_SYSTEM_X86_64:-qemu-system-x86_64}"
CAPTURE_SOURCE="$ROOT_DIR/tools/trace/virtio_net_packet_oracle_capture.c"

usage() {
  echo "usage: $0 [out-events.jsonl]" >&2
  echo "  PROMOTE_CAPTURE=0  keep JSONL only, do not replace vault fixture" >&2
  exit 2
}

if [[ "${2:-}" != "" ]]; then
  usage
fi

mkdir -p "$OUT_DIR"

resolve_kernel() {
  if [[ -n "${S2_COMPAT_KERNEL:-}" && -f "${S2_COMPAT_KERNEL}" ]]; then
    echo "${S2_COMPAT_KERNEL}"
    return 0
  fi
  if [[ -f "$ROOT_DIR/out/compat_s2/kernel/bzImage" ]]; then
    echo "$ROOT_DIR/out/compat_s2/kernel/bzImage"
    return 0
  fi
  if [[ -n "${S2_COMPAT_KERNEL_URL:-}" ]]; then
    S2_COMPAT_KERNEL_URL="${S2_COMPAT_KERNEL_URL}" \
      S2_COMPAT_KERNEL_SHA256="${S2_COMPAT_KERNEL_SHA256:-}" \
      bash "$ROOT_DIR/tools/compat/fetch_compat_kernel.sh" >/dev/null
    echo "$ROOT_DIR/out/compat_s2/kernel/bzImage"
    return 0
  fi
  return 1
}

if ! command -v "$QEMU_BIN" >/dev/null 2>&1; then
  echo "RAMEN_CAPTURE_VIRTIO_NET_PACKET_ORACLE: fail error=missing_qemu" >&2
  exit 1
fi

if ! KERNEL_IMAGE="$(resolve_kernel)"; then
  echo "RAMEN_CAPTURE_VIRTIO_NET_PACKET_ORACLE: fail error=missing_kernel" >&2
  echo "Set S2_COMPAT_KERNEL or S2_COMPAT_KERNEL_URL" >&2
  exit 1
fi

CAPTURE_SOURCE="$CAPTURE_SOURCE" bash "$ROOT_DIR/tools/trace/build_oracle_capture_initrd.sh"
# Packet capture initrd bundles virtio_net.ko for kernel AF_PACKET RX.
INITRD="$OUT_DIR/initrd.cpio.gz"

rm -f "$SERIAL_LOG"
"$QEMU_BIN" \
  -machine q35 \
  -m 512 \
  -smp 1 \
  -kernel "$KERNEL_IMAGE" \
  -initrd "$INITRD" \
  -append "console=ttyS0 quiet" \
  -netdev user,id=net0 \
  -device virtio-net-pci,netdev=net0,bus=pcie.0,addr=0x4,disable-modern=on,disable-legacy=off \
  -no-reboot \
  -nographic \
  -serial "file:$SERIAL_LOG"

if ! grep -q 'RAMEN_VIRTIO_NET_PACKET_CAPTURE: ok' "$SERIAL_LOG"; then
  echo "RAMEN_CAPTURE_VIRTIO_NET_PACKET_ORACLE: fail error=capture_guest_failed" >&2
  tail -20 "$SERIAL_LOG" >&2 || true
  exit 1
fi

python3 - "$SERIAL_LOG" "$JSONL_OUT" <<'PY'
import sys

serial_path, jsonl_path = sys.argv[1], sys.argv[2]
text = open(serial_path, "r", encoding="utf-8", errors="replace").read()
begin = "RAMEN_VIRTIO_NET_PACKET_CAPTURE_BEGIN\n"
end = "\nRAMEN_VIRTIO_NET_PACKET_CAPTURE_END"
start = text.find(begin)
if start < 0:
    raise SystemExit("capture begin sentinel missing")
start += len(begin)
stop = text.find(end, start)
if stop < 0:
    raise SystemExit("capture end sentinel missing")
payload = text[start:stop].strip()
if not payload:
    raise SystemExit("capture payload empty")
with open(jsonl_path, "w", encoding="utf-8") as out:
    out.write(payload)
    out.write("\n")
PY

echo "RAMEN_CAPTURE_VIRTIO_NET_PACKET_ORACLE: ok jsonl=$JSONL_OUT"

if [[ "$PROMOTE" == "1" ]]; then
  bash "$ROOT_DIR/tools/trace/promote_virtio_net_packet_capture.sh" "$JSONL_OUT"
fi