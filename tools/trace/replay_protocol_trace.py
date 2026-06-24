#!/usr/bin/env python3
import argparse
import hashlib
import json


def hex_to_bytes(value: str) -> bytes:
    if len(value) % 2 != 0:
        raise ValueError("hex length must be even")
    return bytes.fromhex(value)


EXPECTED_LEN = {
    "open_file_ro": 16,
    "open_file_ro_reply": 24,
    "resolve_token": 8,
    "resolve_token_reply": 32,
    "cancel": 8,
    "cancel_reply": 16,
    "hello": 16,
    "hello_reply": 16,
    "health": 8,
    "health_reply": 16,
    "shutdown": 16,
    "shutdown_reply": 16,
    "echo_request": 16,
    "echo_reply": 16,
    "start_quarantine_domain": 24,
    "start_quarantine_domain_reply": 24,
    "stop_quarantine_domain": 16,
    "stop_quarantine_domain_reply": 24,
    "export_display": 40,
    "export_display_reply": 40,
    "report_scanout": 32,
    "report_scanout_reply": 24,
}

EXPECTED_REPLY_OP = {
    "open_file_ro": "open_file_ro_reply",
    "resolve_token": "resolve_token_reply",
    "cancel": "cancel_reply",
    "hello": "hello_reply",
    "health": "health_reply",
    "shutdown": "shutdown_reply",
    "echo_request": "echo_reply",
    "start_quarantine_domain": "start_quarantine_domain_reply",
    "stop_quarantine_domain": "stop_quarantine_domain_reply",
    "export_display": "export_display_reply",
    "report_scanout": "report_scanout_reply",
    "ping": "pong",
}


def fail(code: str, detail: str) -> None:
    raise SystemExit(f"REPLAY_PROTOCOL_TRACE: FAIL code={code} detail={detail}")


def summarize_replay(trace: dict) -> dict:
    if trace.get("trace_type") != "protocol_trace":
        fail("REPLAY_TRACE_TYPE_INVALID", "trace_type must be protocol_trace")
    proto = trace.get("protocol_trace")
    if not proto:
        fail("REPLAY_PROTOCOL_TRACE_MISSING", "protocol_trace missing")
    events = proto.get("events", [])
    if len(events) % 2 != 0:
        fail(
            "REPLAY_PROTOCOL_PAIRING_INVALID",
            "protocol_trace events must be request/response pairs",
        )

    canonical_pairs = []
    for i in range(0, len(events), 2):
        req = events[i]
        resp = events[i + 1]
        pair_index = (i // 2) + 1

        req_seq = req.get("seq")
        resp_seq = resp.get("seq")
        expected_req_seq = i + 1
        expected_resp_seq = i + 2
        if req_seq != expected_req_seq:
            fail(
                "REPLAY_SEQ_MISMATCH",
                f"pair={pair_index} request_seq={req_seq} expected={expected_req_seq}",
            )
        if resp_seq != expected_resp_seq:
            fail(
                "REPLAY_SEQ_MISMATCH",
                f"pair={pair_index} response_seq={resp_seq} expected={expected_resp_seq}",
            )

        if req.get("dir") != "request" or resp.get("dir") != "response":
            fail(
                "REPLAY_DIRECTION_MISMATCH",
                f"pair={pair_index} events must alternate request/response",
            )

        op = req.get("op")
        resp_op = resp.get("op")
        if not op:
            fail("REPLAY_REQUEST_OP_MISSING", f"pair={pair_index} missing request op")
        if not resp_op:
            fail("REPLAY_RESPONSE_OP_MISSING", f"pair={pair_index} missing response op")

        expected_resp_op = EXPECTED_REPLY_OP.get(op)
        if expected_resp_op and resp_op != expected_resp_op:
            fail(
                "REPLAY_OP_PAIR_MISMATCH",
                f"pair={pair_index} request_op={op} response_op={resp_op} expected={expected_resp_op}",
            )

        req_hex = req.get("bytes_hex", "")
        resp_hex = resp.get("bytes_hex", "")
        req_bytes = hex_to_bytes(req_hex)
        resp_bytes = hex_to_bytes(resp_hex)
        if op in EXPECTED_LEN:
            if len(req_bytes) != EXPECTED_LEN[op]:
                fail(
                    "REPLAY_REQUEST_LENGTH_MISMATCH",
                    f"pair={pair_index} op={op} len={len(req_bytes)} expected={EXPECTED_LEN[op]}",
                )
            if resp_op in EXPECTED_LEN and len(resp_bytes) != EXPECTED_LEN[resp_op]:
                fail(
                    "REPLAY_RESPONSE_LENGTH_MISMATCH",
                    f"pair={pair_index} op={resp_op} len={len(resp_bytes)} expected={EXPECTED_LEN[resp_op]}",
                )
        if op == "ping":
            if resp_op != "pong" or resp_bytes != b"pong":
                fail("REPLAY_PING_MISMATCH", "ping replay failed: expected pong")
        elif op == "echo_request":
            if resp_op != "echo_reply":
                fail("REPLAY_ECHO_REPLY_MISSING", "echo replay failed: missing echo_reply")
            if len(req_bytes) != 16 or len(resp_bytes) != 16:
                fail("REPLAY_ECHO_LENGTH_MISMATCH", "echo replay failed: length mismatch")
            req_id = int.from_bytes(req_bytes[0:8], "little")
            req_len = int.from_bytes(req_bytes[8:12], "little")
            resp_id = int.from_bytes(resp_bytes[0:8], "little")
            resp_len = int.from_bytes(resp_bytes[8:12], "little")
            if req_id != resp_id or req_len != resp_len:
                fail("REPLAY_ECHO_HEADER_MISMATCH", "echo replay failed: header mismatch")

        canonical_pairs.append(
            {
                "pair": pair_index,
                "request": {
                    "seq": req_seq,
                    "op": op,
                    "bytes_hex": req_hex,
                },
                "response": {
                    "seq": resp_seq,
                    "op": resp_op,
                    "bytes_hex": resp_hex,
                },
            }
        )

    canonical = json.dumps(
        {
            "schema_version": 1,
            "pairs": canonical_pairs,
        },
        separators=(",", ":"),
        sort_keys=True,
    )
    digest = f"sha256:{hashlib.sha256(canonical.encode('utf-8')).hexdigest()}"
    return {
        "events": len(events),
        "pairs": len(canonical_pairs),
        "digest": digest,
    }


def read_trace(path: str) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", required=True)
    parser.add_argument(
        "--compare",
        help="Optional second trace path. When provided, deterministic replay digests must match.",
    )
    args = parser.parse_args()

    try:
        trace_metrics = summarize_replay(read_trace(args.trace))
        print(
            "REPLAY_PROTOCOL_TRACE: METRIC "
            f"source=trace events={trace_metrics['events']} pairs={trace_metrics['pairs']} "
            f"digest={trace_metrics['digest']}"
        )

        if args.compare:
            compare_metrics = summarize_replay(read_trace(args.compare))
            print(
                "REPLAY_PROTOCOL_TRACE: METRIC "
                f"source=compare events={compare_metrics['events']} pairs={compare_metrics['pairs']} "
                f"digest={compare_metrics['digest']}"
            )
            if trace_metrics["digest"] != compare_metrics["digest"]:
                fail(
                    "REPLAY_DIGEST_MISMATCH",
                    f"trace_digest={trace_metrics['digest']} compare_digest={compare_metrics['digest']}",
                )
            print(
                "REPLAY_PROTOCOL_TRACE: MATCH "
                f"digest={trace_metrics['digest']} events={trace_metrics['events']} pairs={trace_metrics['pairs']}"
            )

    except ValueError as exc:
        fail("REPLAY_HEX_DECODE_INVALID", str(exc))

    print("REPLAY_PROTOCOL_TRACE: ok")


if __name__ == "__main__":
    main()
