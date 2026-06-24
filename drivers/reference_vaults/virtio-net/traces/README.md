# virtio-net Trace Fixtures

`oracle_init_trace.json` is a live Linux Oracle capture promoted from QEMU
`virtio-net-pci` hardware reads. `oracle_packet_trace.json` is a live harness-level
capture promoted from `virtio_net_packet_oracle_capture.c` (kernel netdev +
AF_PACKET ARP send/receive).

Re-capture or refresh the packet trace with:

```sh
tools/trace/capture_virtio_net_packet_oracle.sh
```

Re-capture or refresh the init trace with:

```sh
tools/trace/capture_virtio_net_oracle.sh
```

The capture boots a Linux Oracle capsule, records PCI/MMIO through
`tools/trace/virtio_net_oracle_capture.c`, and promotes the JSONL through
`tools/trace/promote_virtio_net_capture.sh`.

Manual promotion from an existing `pci_mmio_tracer` JSONL stream still works:

```sh
tools/trace/promote_virtio_net_capture.sh tracer-events.jsonl
```

The promotion script converts the JSONL, stamps a missing `trace_id` from the
source JSONL SHA-256 digest, validates schema, replays the trace, requires live
provenance, and only then replaces `oracle_init_trace.json`.

For a non-default destination:

```sh
tools/trace/promote_virtio_net_capture.sh tracer-events.jsonl /tmp/oracle_init_trace.json
```

When the fixtures are genuinely live, run:

```sh
REQUIRE_LIVE_ORACLE_TRACE=1 bash tools/ci/foundry_s11_reference_vault_s11_3.sh
```

The live-provenance check rejects traces whose `trace_id` contains `scaffold`
and requires a full `sha256:` trace ID, `timestamp_ns` on every event, and
contiguous `seq=1..N` event numbering. Sequence gaps usually mean the tracer
buffer wrapped or events were dropped, so the capture must be repeated.

The strict path also runs `driver_foundry assert-hardware-packet-trace` on
`oracle_packet_trace.json`, which requires live ARP send/receive notes and
rejects slirp-derived receive fallbacks.