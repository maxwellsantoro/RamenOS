#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/compat_s2/kernel"
URL="${S2_COMPAT_KERNEL_URL:-}"
SHA256="${S2_COMPAT_KERNEL_SHA256:-}"

if [[ -z "$URL" ]]; then
  echo "Missing S2_COMPAT_KERNEL_URL" >&2
  exit 2
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "Missing curl" >&2
  exit 2
fi
mkdir -p "$OUT_DIR"

DEB="$OUT_DIR/kernel.deb"
VMLINUX_OUT="$OUT_DIR/bzImage"
EXTRACT_DIR="$OUT_DIR/extracted"

verify_sha256() {
  local file="$1"
  if [[ -z "$SHA256" ]]; then
    return 0
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    echo "$SHA256  $file" | sha256sum -c - >/dev/null
  elif command -v shasum >/dev/null 2>&1; then
    echo "$SHA256  $file" | shasum -a 256 -c - >/dev/null
  else
    echo "Missing sha256sum or shasum for verification" >&2
    exit 2
  fi
}

download_deb() {
  local tmp="$DEB.tmp.$$"
  rm -f "$tmp"

  # For GitHub release URLs, prefer gh CLI (handles private repo auth).
  if command -v gh >/dev/null 2>&1 && \
     [[ "$URL" =~ github\.com/([^/]+/[^/]+)/releases/download/([^/]+)/(.+)$ ]]; then
    local gh_repo="${BASH_REMATCH[1]}"
    local gh_tag="${BASH_REMATCH[2]}"
    local gh_file="${BASH_REMATCH[3]}"
    local dl_dir
    dl_dir="$(mktemp -d)"
    if gh release download "$gh_tag" --repo "$gh_repo" \
         --pattern "$gh_file" --dir "$dl_dir" --clobber; then
      mv "$dl_dir/$gh_file" "$tmp"
      rm -rf "$dl_dir"
    else
      rm -rf "$dl_dir"
      echo "gh release download failed; falling back to curl" >&2
      tmp=""  # signal curl fallback
    fi
  fi

  # Curl fallback (works for public URLs and non-GitHub sources).
  if [[ -z "$tmp" || ! -f "$tmp" ]]; then
    tmp="$DEB.tmp.$$"
    curl --fail --silent --show-error --location \
      --retry 5 --retry-delay 2 --retry-all-errors \
      --connect-timeout 10 --max-time 300 \
      "$URL" -o "$tmp"
  fi

  if ! verify_sha256 "$tmp"; then
    echo "Kernel download failed SHA256 verification" >&2
    rm -f "$tmp"
    exit 2
  fi
  mv -f "$tmp" "$DEB"
}

if [[ -f "$DEB" ]]; then
  if ! verify_sha256 "$DEB"; then
    echo "Cached kernel.deb failed verification; re-downloading" >&2
    rm -f "$DEB"
  fi
fi

if [[ ! -f "$DEB" ]]; then
  download_deb
fi

# Final verification (defense-in-depth).
verify_sha256 "$DEB"

rm -rf "$EXTRACT_DIR"
mkdir -p "$EXTRACT_DIR"

extract_deb() {
  local deb="$1"
  local out_dir="$2"

  if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb -x "$deb" "$out_dir"
    return 0
  fi

  local tmp
  tmp="$(mktemp -d)"
  cp "$deb" "$tmp/pkg.deb"
  (
    cd "$tmp"
    ar x pkg.deb
    local data_tar
    if [[ -f data.tar ]]; then
      data_tar="data.tar"
    else
      data_tar="$(ls data.tar.* 2>/dev/null | head -n 1 || true)"
    fi
    if [[ -z "$data_tar" ]]; then
      exit 1
    fi
    tar -xf "$data_tar" -C "$out_dir"
  )
  local rc=$?
  rm -rf "$tmp"
  return "$rc"
}

extract_deb "$DEB" "$EXTRACT_DIR"
VMLINUX_SRC=$(ls "$EXTRACT_DIR"/boot/vmlinuz-* 2>/dev/null | head -n 1 || true)
if [[ -z "$VMLINUX_SRC" ]]; then
  echo "Failed to locate vmlinuz in kernel package" >&2
  exit 2
fi

cp -f "$VMLINUX_SRC" "$VMLINUX_OUT"

echo "$VMLINUX_OUT"
