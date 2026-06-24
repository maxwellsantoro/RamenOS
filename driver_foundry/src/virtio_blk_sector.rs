//! virtio-blk sector I/O driver distilled from the harness.block Oracle trace.
//!
//! Replays shared-memory `read_blocks` / `write_blocks` exchanges against
//! `MockBlockHarness`.

use kernel_api::generated::{ReadBlocks, ReadBlocksReply, WriteBlocks, WriteBlocksReply};
use kernel_api::mock::block_harness::{
    BlockReplayEvent, BlockReplayMismatch, BlockReplayOp, BlockReplayResult, MockBlockHarness,
};

/// Run the sector exchange sequence described by the Oracle trace events.
pub fn exchange_sectors_from_events(
    harness: &mut MockBlockHarness<'_>,
    events: &[BlockReplayEvent<'_>],
) -> BlockReplayResult<()> {
    for event in events {
        match event.op {
            BlockReplayOp::ReadBlocks => {
                read_blocks_from_event(harness, *event)?;
            }
            BlockReplayOp::WriteBlocks => {
                harness
                    .shmem()
                    .write(event.shm_cap, event.offset, event.payload)?;
                write_blocks_from_event(harness, *event)?;
            }
        }
    }
    harness.finish()
}

pub fn read_blocks_from_event(
    harness: &mut MockBlockHarness<'_>,
    event: BlockReplayEvent<'_>,
) -> BlockReplayResult<ReadBlocksReply> {
    let reply = harness.read_blocks(ReadBlocks {
        request_id: event.request_id,
        lba: event.lba,
        block_count: event.block_count,
        block_size: event.block_size,
        buffer_shm_cap: event.shm_cap,
        buffer_offset: event.offset,
    })?;
    if reply.status != event.status {
        return Err(BlockReplayMismatch::Status {
            expected: event.status,
            observed: reply.status,
        });
    }
    if reply.bytes_read != event.bytes {
        return Err(BlockReplayMismatch::Bytes {
            expected: event.bytes,
            observed: reply.bytes_read,
        });
    }

    let observed = harness
        .shmem()
        .read(event.shm_cap, event.offset, event.len)?;
    if observed != event.payload {
        return Err(BlockReplayMismatch::Payload);
    }

    Ok(reply)
}

pub fn write_blocks_from_event(
    harness: &mut MockBlockHarness<'_>,
    event: BlockReplayEvent<'_>,
) -> BlockReplayResult<WriteBlocksReply> {
    let reply = harness.write_blocks(WriteBlocks {
        request_id: event.request_id,
        lba: event.lba,
        block_count: event.block_count,
        block_size: event.block_size,
        data_shm_cap: event.shm_cap,
        data_offset: event.offset,
    })?;
    if reply.status != event.status {
        return Err(BlockReplayMismatch::Status {
            expected: event.status,
            observed: reply.status,
        });
    }
    if reply.bytes_written != event.bytes {
        return Err(BlockReplayMismatch::Bytes {
            expected: event.bytes,
            observed: reply.bytes_written,
        });
    }
    Ok(reply)
}

pub fn replay_vault_sector_trace() -> Result<(), crate::DriverReplayError> {
    let trace = crate::load_sector_trace(vault_sector_fixture_path())?;
    replay_sector_trace(&trace)
}

pub fn replay_sector_trace(
    trace: &artifact_store_schema::block_sector_trace::BlockSectorTraceV0,
) -> Result<(), crate::DriverReplayError> {
    let bundle = crate::sector_trace_to_replay_bundle(trace)?;
    bundle.with_events(|events| {
        let mut harness = MockBlockHarness::new(events, bundle.shm_cap());
        exchange_sectors_from_events(&mut harness, events)
            .map_err(|err| crate::DriverReplayError::PacketReplay(format!("{err:?}")))
    })
}

fn vault_sector_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../drivers/reference_vaults/virtio-blk/traces/oracle_block_trace.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    const READ_PAYLOAD: [u8; 4] = [0x13, 0x26, 0x39, 0x4c];
    const WRITE_PAYLOAD: [u8; 4] = [0x37, 0x6e, 0xa5, 0xdc];

    #[test]
    fn exchange_sectors_from_events_replays_read_then_write() {
        let trace = [
            BlockReplayEvent {
                op: BlockReplayOp::ReadBlocks,
                request_id: 1,
                lba: 0,
                block_count: 1,
                block_size: 4,
                shm_cap: 4096,
                offset: 0,
                len: 4,
                payload: &READ_PAYLOAD,
                status: 0,
                bytes: 4,
            },
            BlockReplayEvent {
                op: BlockReplayOp::WriteBlocks,
                request_id: 2,
                lba: 1,
                block_count: 1,
                block_size: 4,
                shm_cap: 4096,
                offset: 512,
                len: 4,
                payload: &WRITE_PAYLOAD,
                status: 0,
                bytes: 4,
            },
        ];
        let mut harness = MockBlockHarness::new(&trace, 4096);
        harness.shmem().write(4096, 512, &WRITE_PAYLOAD).unwrap();
        exchange_sectors_from_events(&mut harness, &trace).unwrap();
    }

    #[test]
    fn replay_vault_sector_trace_matches_oracle_fixture() {
        replay_vault_sector_trace().expect("vault sector trace must replay");
    }
}
