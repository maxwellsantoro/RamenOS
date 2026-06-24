#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/out/logs"
mkdir -p "$LOG_DIR"

skip_gate() {
  local reason="$1"
  echo "FOUNDRY_COMPAT_S2: skipped ($reason)"
  if [[ "${RAMEN_CI_STRICT:-}" == "1" ]]; then
    echo "FOUNDRY_COMPAT_S2: FAIL (strict mode, skip not allowed)" >&2
    exit 1
  fi
  exit 0
}

KERNEL_IMAGE="${S2_COMPAT_KERNEL:-}"
INITRD_IMAGE="${S2_COMPAT_INITRD:-}"
CC_BIN="${CC:-cc}"
ARTIFACT_IMAGE="${S2_COMPAT_ARTIFACT:-}"

if [[ -z "$KERNEL_IMAGE" && -n "${S2_COMPAT_KERNEL_URL:-}" ]]; then
  "$ROOT_DIR/tools/compat/fetch_compat_kernel.sh"
  KERNEL_IMAGE="$ROOT_DIR/out/compat_s2/kernel/bzImage"
fi

if [[ -z "$KERNEL_IMAGE" ]]; then
  for candidate in /boot/vmlinuz-*; do
    if [[ -f "$candidate" ]]; then
      KERNEL_IMAGE="$candidate"
      break
    fi
  done
fi

if [[ -z "$KERNEL_IMAGE" ]]; then
  skip_gate "set S2_COMPAT_KERNEL and S2_COMPAT_INITRD and S2_COMPAT_ARTIFACT"
fi

if [[ -n "$KERNEL_IMAGE" ]]; then
  base="$(basename "$KERNEL_IMAGE")"
  cfg="/boot/config-${base#vmlinuz-}"
  if [[ -f "$cfg" ]]; then
    if ! grep -q "^CONFIG_EXT4_FS=y" "$cfg"; then
      skip_gate "ext4 not built-in in $cfg"
    fi
  fi
fi

if [[ -z "$INITRD_IMAGE" ]]; then
  INITRD_IMAGE="$ROOT_DIR/out/compat_s2/initrd.cpio.gz"
  if [[ ! -f "$INITRD_IMAGE" ]]; then
    if ! command -v cpio >/dev/null 2>&1; then
      skip_gate "missing cpio"
    fi
    if ! command -v gzip >/dev/null 2>&1; then
      skip_gate "missing gzip"
    fi
    if ! command -v "$CC_BIN" >/dev/null 2>&1; then
      skip_gate "missing compiler: $CC_BIN"
    fi
    "$ROOT_DIR/tools/compat/build_compat_initrd.sh"
  fi
fi

if [[ -z "$ARTIFACT_IMAGE" ]]; then
  ARTIFACT_IMAGE="$ROOT_DIR/out/compat_s2/artifact.img"
  if [[ ! -f "$ARTIFACT_IMAGE" ]]; then
    if ! command -v mkfs.ext4 >/dev/null 2>&1 && ! command -v mke2fs >/dev/null 2>&1; then
      skip_gate "missing mkfs.ext4/mke2fs"
    fi
    if ! command -v debugfs >/dev/null 2>&1; then
      skip_gate "missing debugfs"
    fi
    "$ROOT_DIR/tools/compat/build_compat_artifact_img.sh"
  fi
fi

if [[ -z "$INITRD_IMAGE" || -z "$ARTIFACT_IMAGE" ]]; then
  skip_gate "set S2_COMPAT_INITRD and S2_COMPAT_ARTIFACT"
fi

plan_dir="$ROOT_DIR/out/compat_s2"
kernel_dst="$plan_dir/kernel/bzImage"
initrd_dst="$plan_dir/initrd.cpio.gz"
artifact_dst="$plan_dir/artifact.img"

mkdir -p "$plan_dir/kernel"

if [[ "$KERNEL_IMAGE" != /* ]]; then
  KERNEL_IMAGE="$ROOT_DIR/$KERNEL_IMAGE"
fi
if [[ "$INITRD_IMAGE" != /* ]]; then
  INITRD_IMAGE="$ROOT_DIR/$INITRD_IMAGE"
fi
if [[ "$ARTIFACT_IMAGE" != /* ]]; then
  ARTIFACT_IMAGE="$ROOT_DIR/$ARTIFACT_IMAGE"
fi

if [[ "$KERNEL_IMAGE" != "$kernel_dst" ]]; then
  cp -f "$KERNEL_IMAGE" "$kernel_dst"
fi
if [[ "$INITRD_IMAGE" != "$initrd_dst" ]]; then
  cp -f "$INITRD_IMAGE" "$initrd_dst"
fi
if [[ "$ARTIFACT_IMAGE" != "$artifact_dst" ]]; then
  cp -f "$ARTIFACT_IMAGE" "$artifact_dst"
fi

assert_log() {
  local log="$1"
  grep -q "COMPAT_S2: hello" "$log"
  grep -q "COMPAT_S2: read artifact ok" "$log"
  grep -q "COMPAT_S2: write blocked ok" "$log"
}

wait_for_log() {
  local log="$1"
  local timeout_s="$2"
  local max_iters=$((timeout_s * 5))

  for _ in $(seq 1 "$max_iters"); do
    if [[ -f "$log" ]] && assert_log "$log"; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

log="$LOG_DIR/qemu_compat_s2.log"
sup_log="$LOG_DIR/supervisor_compat_s2.log"
rm -f "$log"
rm -f "$sup_log"

plan_path="$plan_dir/launch_plan.json"
installed_root="$ROOT_DIR/out/installed"

cargo run -p store_cli -- emit-plan \
  --catalog "$ROOT_DIR/store/catalog.json" \
  --program-id "ramen.compat.hello" \
  --out "$plan_path" \
  --installed-root "$installed_root"

PLAN_PATH="$plan_path" INSTALLED_ROOT="$installed_root" python3 - <<'PY'
import json
import os

plan = json.load(open(os.environ["PLAN_PATH"]))

def check_id(val, label):
    if not isinstance(val, str):
        raise SystemExit(f"{label} missing")
    if not val.startswith("sha256:"):
        raise SystemExit(f"{label} must be content id: {val}")
    if "/" in val or "\\" in val:
        raise SystemExit(f"{label} must be path-free: {val}")

check_id(plan["artifact_ref"], "artifact_ref")
if plan.get("schema_version") == 1:
    import json as _json

    runner_config = plan.get("runner_config") or {}
    config_json = runner_config.get("config_json") or "{}"
    payload = _json.loads(config_json)
    capsule = payload.get("compat_capsule")
else:
    capsule = plan.get("compat_capsule")
if not capsule:
    raise SystemExit("compat_capsule missing")
check_id(capsule["kernel_content_id"], "kernel_content_id")
check_id(capsule["initrd_content_id"], "initrd_content_id")
for idx, disk in enumerate(capsule.get("artifact_disks", [])):
    check_id(disk.get("content_id"), f"artifact_disks[{idx}].content_id")
if "log_path" in capsule:
    raise SystemExit("compat_capsule must not include log_path")

installed_root = os.environ["INSTALLED_ROOT"]

def blob_path(root, content_id):
    name = content_id.replace("sha256:", "")
    return os.path.join(root, "artifacts", f"{name}.blob")

def manifest_path(root, content_id):
    name = content_id.replace("sha256:", "")
    return os.path.join(root, "artifacts", f"{name}.manifest.json")

for label, cid in [
    ("artifact_ref", plan["artifact_ref"]),
    ("kernel_content_id", capsule["kernel_content_id"]),
    ("initrd_content_id", capsule["initrd_content_id"]),
]:
    if not os.path.exists(blob_path(installed_root, cid)):
        raise SystemExit(f"missing blob for {label}: {cid}")
    if not os.path.exists(manifest_path(installed_root, cid)):
        raise SystemExit(f"missing manifest for {label}: {cid}")

for disk in capsule.get("artifact_disks", []):
    cid = disk["content_id"]
    if not os.path.exists(blob_path(installed_root, cid)):
        raise SystemExit(f"missing blob for disk: {cid}")
    if not os.path.exists(manifest_path(installed_root, cid)):
        raise SystemExit(f"missing manifest for disk: {cid}")
PY

# Launch via the runtime_supervisor → compat_runner path.
cargo run -p runtime_supervisor -- \
  --plan "$plan_path" \
  --installed-root "$installed_root" \
  --compat-log-path "$log" >"$sup_log" 2>&1 &

pid=$!
if ! wait_for_log "$log" 15; then
  kill "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
  assert_log "$log"
fi

kill "$pid" >/dev/null 2>&1 || true
wait "$pid" >/dev/null 2>&1 || true

echo "FOUNDRY_COMPAT_S2: ok"
