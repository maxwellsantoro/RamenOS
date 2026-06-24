#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/capsule_relay"
ROOTFS="$OUT_DIR/root"
INITRD="$OUT_DIR/capsule_initrd.cpio.gz"
GENERATED_DIR="$ROOT_DIR/tools/capsule/generated"
CONTROL_IDL="$ROOT_DIR/idl/harness/capsule_control_v0.toml"
CONTROL_HDR="$GENERATED_DIR/capsule_control_v0.h"

# Use musl-gcc for truly static binaries; fall back to cc if unavailable
if command -v musl-gcc >/dev/null 2>&1; then
    CC="musl-gcc"
elif command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    CC="x86_64-linux-musl-gcc"
else
    CC="${CC:-cc}"
fi

CFLAGS="-O2 -Wall -Wextra -std=c99"
# Only add -static if not on macOS (macOS doesn't support -static for executables)
if [[ "$(uname -s)" != "Darwin" ]]; then
    CFLAGS="$CFLAGS -static"
fi

rm -rf "$ROOTFS"
mkdir -p "$ROOTFS"
mkdir -p "$GENERATED_DIR"

if ! command -v "$CC" >/dev/null 2>&1; then
    echo "Missing compiler: $CC" >&2
    exit 2
fi

if [[ "$(uname -s)" == "Darwin" && "$CC" == "cc" ]]; then
    echo "capsule initrd build requires a Linux-targeting C toolchain on macOS (e.g. musl-gcc)" >&2
    exit 2
fi

echo "Compiling capsule_agent with $CC..."
cargo run -p idl_codegen -- --in "$CONTROL_IDL" --out "$CONTROL_HDR"
$CC $CFLAGS -I"$GENERATED_DIR" -o "$ROOTFS/init" "$ROOT_DIR/tools/capsule/capsule_agent.c"
chmod 0755 "$ROOTFS/init"

(
    cd "$ROOTFS"
    find . -print0 | cpio --null -H newc -o
) | gzip -9 > "$INITRD"

echo "capsule initrd: $INITRD"
