#!/usr/bin/env python3
import argparse
import pathlib
import struct
import sys

MAGIC = int.from_bytes(b"RINI", "little")
VERSION = 1

OP_HELLO = 1
OP_PING_PONG = 2
OP_BADLEN = 3
OP_TRACE = 4
OP_ALT_HELLO = 5
OP_SHMEM_TEST = 6  # S8 Phase 4: shared-memory data-plane integration test
OP_SEMANTIC_SNAPSHOT = 7  # S10.5.0: semantic snapshot over QEMU init/shmem
OP_SEMANTIC_IPC_RELAY = 8  # S10.5.2: host-framed IPC relay over COM2
OP_NET_PACKET_IO = 9  # S11.8: runtime harness.net packet I/O
OP_GOP_PROBE = 10  # S12.1: golden machine GOP probe
OP_HIL_BOOT = 11  # S12.2: physical HIL boot graduation marker
OP_IOMMU_INVENTORY = 12  # S12.3: ACPI DMAR / VT-d inventory marker
OP_BLOCK_IO = 13  # S13.6: runtime harness.block sector I/O
OP_NVME_BOOT = 14  # S13.7: metal NVMe boot graduation marker
OP_ATOMIC_UPDATE = 15  # S13.8: A/B slot atomic update graduation marker

PROFILES = {
    "default": ("init-default", [OP_HELLO, OP_PING_PONG, OP_BADLEN, OP_TRACE]),
    "alt": ("init-alt", [OP_ALT_HELLO, OP_PING_PONG, OP_BADLEN, OP_TRACE]),
    "bad": ("", []),
    "shmem_test": ("init-shmem-test", [OP_SHMEM_TEST]),  # S8 Phase 4 integration test
    "semantic_snapshot": ("init-semantic-snapshot", [OP_SEMANTIC_SNAPSHOT]),
    "semantic_ipc_bridge": ("init-semantic-ipc-bridge", [OP_SEMANTIC_IPC_RELAY]),
    "net_packet_io": ("init-net-packet-io", [OP_NET_PACKET_IO]),
    "gop_probe": ("init-gop-probe", [OP_GOP_PROBE]),
    "hil_boot": ("init-hil-boot", [OP_HIL_BOOT]),
    "iommu_inventory": ("init-iommu-inventory", [OP_IOMMU_INVENTORY]),
    "block_io": ("init-block-io", [OP_BLOCK_IO]),
    "nvme_boot": ("init-nvme-boot", [OP_NVME_BOOT]),
    "atomic_update": ("init-atomic-update", [OP_ATOMIC_UPDATE]),
}


def build(profile: str, pad_len: int) -> bytes:
    if profile not in PROFILES:
        raise SystemExit(f"unknown profile: {profile}")
    content_id, ops = PROFILES[profile]
    content_bytes = content_id.encode("utf-8")
    if len(content_bytes) > 0xFFFF:
        raise SystemExit("content_id too long")
    if len(ops) > 0xFFFF:
        raise SystemExit("too many ops")
    header = struct.pack(
        "<IHHHH", MAGIC, VERSION, len(content_bytes), len(ops), 0
    )
    body = header + content_bytes + bytes(ops)
    if pad_len:
        if len(body) > pad_len:
            raise SystemExit("image exceeds pad length")
        body = body + b"\x00" * (pad_len - len(body))
    return body


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--profile", default="default", choices=PROFILES.keys())
    parser.add_argument("--pad-len", type=int, default=4096)
    args = parser.parse_args()

    data = build(args.profile, args.pad_len)
    out_path = pathlib.Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(data)


if __name__ == "__main__":
    main()
