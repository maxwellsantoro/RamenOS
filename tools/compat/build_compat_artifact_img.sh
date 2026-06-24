#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/compat_s2"
IMG="$OUT_DIR/artifact.img"
ARTIFACT_TXT="$OUT_DIR/artifact.txt"

mkdir -p "$OUT_DIR"

printf "ok\n" > "$ARTIFACT_TXT"

MKFS=""
if command -v mkfs.ext4 >/dev/null 2>&1; then
  MKFS="mkfs.ext4 -q -F"
elif command -v mke2fs >/dev/null 2>&1; then
  MKFS="mke2fs -q -F -t ext4"
else
  echo "Missing mkfs.ext4 or mke2fs" >&2
  exit 2
fi

if ! command -v debugfs >/dev/null 2>&1; then
  echo "Missing debugfs" >&2
  exit 2
fi

if ! command -v dd >/dev/null 2>&1; then
  echo "Missing dd" >&2
  exit 2
fi

dd if=/dev/zero of="$IMG" bs=1M count=2 >/dev/null 2>&1
$MKFS "$IMG" >/dev/null 2>&1
debugfs -w -R "write $ARTIFACT_TXT /artifact.txt" "$IMG" >/dev/null 2>&1

echo "compat artifact image: $IMG"
