#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def build_protocol_trace():
    return {
        "schema_version": 1,
        "trace_type": "protocol_trace",
        "protocol_trace": {
            "metadata": {
                "capsule_id": "compat-demo",
                "capsule_image": "sha256:placeholder",
                "harness_name": "ping_harness",
                "harness_version": 0,
            },
            "events": [
                {
                    "seq": 1,
                    "dir": "request",
                    "op": "ping",
                    "bytes_hex": "70696e67",
                    "result": "ok",
                },
                {
                    "seq": 2,
                    "dir": "response",
                    "op": "pong",
                    "bytes_hex": "706f6e67",
                    "result": "ok",
                },
            ],
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument(
        "--trace-type",
        default="protocol",
        choices=["protocol"],
        help="trace type to emit",
    )
    args = parser.parse_args()

    if args.trace_type == "protocol":
        trace = build_protocol_trace()
    else:
        raise SystemExit("unsupported trace type")

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(trace, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
