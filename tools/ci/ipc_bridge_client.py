#!/usr/bin/env python3
"""Host client for S10.5.2 QEMU IPC bridge gate (length-prefixed envelope frames)."""

import socket
import struct
import sys
import time

ENVELOPE_WIRE_SIZE = 88
# repr(C) sizes from kernel_api generated structs (include tail padding).
GET_SNAPSHOT_WIRE_SIZE = 24
GET_SNAPSHOT_REPLY_WIRE_SIZE = 32


def build_get_snapshot(cap: int = 0x5310_0000_0000_0002, req_id: int = 7) -> bytes:
    payload = struct.pack("<QQI", cap, req_id, 0)
    payload += b"\0" * (GET_SNAPSHOT_WIRE_SIZE - len(payload))
    payload = payload.ljust(64, b"\0")
    wire = struct.pack("<IIQI", 10, 1, 0, GET_SNAPSHOT_WIRE_SIZE) + payload
    if len(wire) != 84:
        raise RuntimeError(f"unexpected envelope wire size {len(wire)}")
    return wire + b"\0" * 4


def send_frame(sock: socket.socket, wire: bytes) -> None:
    sock.sendall(struct.pack("<I", len(wire)) + wire)


def recv_exact(sock: socket.socket, nbytes: int) -> bytes:
    buf = bytearray()
    while len(buf) < nbytes:
        chunk = sock.recv(nbytes - len(buf))
        if not chunk:
            raise RuntimeError("short read")
        buf.extend(chunk)
    return bytes(buf)


def recv_frame(sock: socket.socket) -> bytes:
    len_buf = recv_exact(sock, 4)
    frame_len = struct.unpack("<I", len_buf)[0]
    return recv_exact(sock, frame_len)


def cmd_get_snapshot(path: str) -> None:
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(10.0)
    sock.connect(path)
    time.sleep(0.5)
    send_frame(sock, build_get_snapshot())
    reply = recv_frame(sock)
    if len(reply) != ENVELOPE_WIRE_SIZE:
        raise SystemExit(f"bad reply size {len(reply)}")
    payload_len = struct.unpack("<I", reply[16:20])[0]
    if payload_len != GET_SNAPSHOT_REPLY_WIRE_SIZE:
        raise SystemExit(f"bad reply payload_len {payload_len}")
    status = struct.unpack("<I", reply[28:32])[0]
    shm_cap = struct.unpack("<Q", reply[36:44])[0]
    print(f"status={status} shm_cap={shm_cap}")
    if status != 0 or shm_cap == 0:
        raise SystemExit(1)


def cmd_oversize(path: str) -> None:
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(10.0)
    sock.connect(path)
    time.sleep(0.5)
    sock.sendall(struct.pack("<I", 5000))


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <get_snapshot|oversize> <socket_path>", file=sys.stderr)
        return 2
    mode, path = sys.argv[1], sys.argv[2]
    if mode == "get_snapshot":
        cmd_get_snapshot(path)
    elif mode == "oversize":
        cmd_oversize(path)
    else:
        print(f"unknown mode: {mode}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
