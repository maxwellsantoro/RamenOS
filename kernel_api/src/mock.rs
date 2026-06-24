//! Mock hardware helpers for deterministic driver replay gates.

pub mod pci_device {
    /// PCI/MMIO operation observed in an Oracle driver trace.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum PciReplayOp {
        PciConfigRead,
        PciConfigWrite,
        MmioRead,
        MmioWrite,
    }

    /// One normalized PCI/MMIO access used by the S11 replay scoreboard.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct PciReplayEvent {
        pub op: PciReplayOp,
        pub bar: u8,
        pub offset: u64,
        pub width: u8,
        pub value: u64,
    }

    impl PciReplayEvent {
        pub const fn pci_config_read(offset: u64, width: u8, value: u64) -> Self {
            Self {
                op: PciReplayOp::PciConfigRead,
                bar: 0xff,
                offset,
                width,
                value,
            }
        }

        pub const fn pci_config_write(offset: u64, width: u8, value: u64) -> Self {
            Self {
                op: PciReplayOp::PciConfigWrite,
                bar: 0xff,
                offset,
                width,
                value,
            }
        }

        pub const fn mmio_read(bar: u8, offset: u64, width: u8, value: u64) -> Self {
            Self {
                op: PciReplayOp::MmioRead,
                bar,
                offset,
                width,
                value,
            }
        }

        pub const fn mmio_write(bar: u8, offset: u64, width: u8, value: u64) -> Self {
            Self {
                op: PciReplayOp::MmioWrite,
                bar,
                offset,
                width,
                value,
            }
        }
    }

    /// Deterministic mismatch reasons for replay gates.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum ReplayMismatch {
        UnexpectedAccess {
            observed: PciReplayEvent,
        },
        Op {
            expected: PciReplayOp,
            observed: PciReplayOp,
        },
        Bar {
            expected: u8,
            observed: u8,
        },
        Offset {
            expected: u64,
            observed: u64,
        },
        Width {
            expected: u8,
            observed: u8,
        },
        Value {
            expected: u64,
            observed: u64,
        },
        Incomplete {
            expected_total: usize,
            observed_total: usize,
        },
    }

    pub type ReplayResult<T> = Result<T, ReplayMismatch>;

    /// Compares live PCI/MMIO accesses against a fixed Oracle trace.
    pub struct ReplayScoreboard<'a> {
        expected: &'a [PciReplayEvent],
        cursor: usize,
        mismatch: Option<ReplayMismatch>,
    }

    impl<'a> ReplayScoreboard<'a> {
        pub const fn new(expected: &'a [PciReplayEvent]) -> Self {
            Self {
                expected,
                cursor: 0,
                mismatch: None,
            }
        }

        pub fn record(&mut self, observed: PciReplayEvent) -> ReplayResult<()> {
            if let Some(mismatch) = self.mismatch {
                return Err(mismatch);
            }

            let Some(expected) = self.expected.get(self.cursor).copied() else {
                return self.fail(ReplayMismatch::UnexpectedAccess { observed });
            };

            if expected.op != observed.op {
                return self.fail(ReplayMismatch::Op {
                    expected: expected.op,
                    observed: observed.op,
                });
            }
            if expected.bar != observed.bar {
                return self.fail(ReplayMismatch::Bar {
                    expected: expected.bar,
                    observed: observed.bar,
                });
            }
            if expected.offset != observed.offset {
                return self.fail(ReplayMismatch::Offset {
                    expected: expected.offset,
                    observed: observed.offset,
                });
            }
            if expected.width != observed.width {
                return self.fail(ReplayMismatch::Width {
                    expected: expected.width,
                    observed: observed.width,
                });
            }
            if expected.value != observed.value {
                return self.fail(ReplayMismatch::Value {
                    expected: expected.value,
                    observed: observed.value,
                });
            }

            self.cursor += 1;
            Ok(())
        }

        pub fn finish(&mut self) -> ReplayResult<()> {
            if let Some(mismatch) = self.mismatch {
                return Err(mismatch);
            }
            if self.cursor != self.expected.len() {
                return self.fail(ReplayMismatch::Incomplete {
                    expected_total: self.expected.len(),
                    observed_total: self.cursor,
                });
            }
            Ok(())
        }

        pub fn observed_count(&self) -> usize {
            self.cursor
        }

        pub fn expected_count(&self) -> usize {
            self.expected.len()
        }

        pub fn remaining_count(&self) -> usize {
            self.expected.len().saturating_sub(self.cursor)
        }

        pub fn mismatch(&self) -> Option<ReplayMismatch> {
            self.mismatch
        }

        fn fail<T>(&mut self, mismatch: ReplayMismatch) -> ReplayResult<T> {
            self.mismatch = Some(mismatch);
            Err(mismatch)
        }
    }

    /// Mock PCI device whose reads return Oracle values and whose writes are scored.
    pub struct MockPciDevice<'a> {
        scoreboard: ReplayScoreboard<'a>,
    }

    impl<'a> MockPciDevice<'a> {
        pub const fn new(expected: &'a [PciReplayEvent]) -> Self {
            Self {
                scoreboard: ReplayScoreboard::new(expected),
            }
        }

        pub fn pci_config_read(&mut self, offset: u64, width: u8) -> ReplayResult<u64> {
            let value = self.peek_next_value()?;
            self.scoreboard
                .record(PciReplayEvent::pci_config_read(offset, width, value))?;
            Ok(value)
        }

        pub fn pci_config_write(&mut self, offset: u64, width: u8, value: u64) -> ReplayResult<()> {
            self.scoreboard
                .record(PciReplayEvent::pci_config_write(offset, width, value))
        }

        pub fn mmio_read(&mut self, bar: u8, offset: u64, width: u8) -> ReplayResult<u64> {
            let value = self.peek_next_value()?;
            self.scoreboard
                .record(PciReplayEvent::mmio_read(bar, offset, width, value))?;
            Ok(value)
        }

        pub fn mmio_write(
            &mut self,
            bar: u8,
            offset: u64,
            width: u8,
            value: u64,
        ) -> ReplayResult<()> {
            self.scoreboard
                .record(PciReplayEvent::mmio_write(bar, offset, width, value))
        }

        pub fn finish(&mut self) -> ReplayResult<()> {
            self.scoreboard.finish()
        }

        pub fn scoreboard(&self) -> &ReplayScoreboard<'a> {
            &self.scoreboard
        }

        fn peek_next_value(&mut self) -> ReplayResult<u64> {
            if let Some(mismatch) = self.scoreboard.mismatch {
                return Err(mismatch);
            }
            self.scoreboard
                .expected
                .get(self.scoreboard.cursor)
                .map(|event| event.value)
                .ok_or_else(|| {
                    let observed = PciReplayEvent::pci_config_read(0, 0, 0);
                    let mismatch = ReplayMismatch::UnexpectedAccess { observed };
                    self.scoreboard.mismatch = Some(mismatch);
                    mismatch
                })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const VIRTIO_NET_INIT_TRACE: [PciReplayEvent; 4] = [
            PciReplayEvent::pci_config_read(0x00, 2, 0x1af4),
            PciReplayEvent::pci_config_read(0x02, 2, 0x1000),
            PciReplayEvent::mmio_read(0, 0x12, 2, 0x0000),
            PciReplayEvent::mmio_write(0, 0x12, 2, 0x0001),
        ];

        #[test]
        fn replay_scoreboard_accepts_matching_trace() {
            let mut scoreboard = ReplayScoreboard::new(&VIRTIO_NET_INIT_TRACE);

            for event in VIRTIO_NET_INIT_TRACE {
                scoreboard.record(event).unwrap();
            }

            assert_eq!(scoreboard.observed_count(), 4);
            assert_eq!(scoreboard.remaining_count(), 0);
            scoreboard.finish().unwrap();
        }

        #[test]
        fn replay_scoreboard_rejects_mismatched_write_value() {
            let mut scoreboard = ReplayScoreboard::new(&VIRTIO_NET_INIT_TRACE);

            scoreboard.record(VIRTIO_NET_INIT_TRACE[0]).unwrap();
            let err = scoreboard
                .record(PciReplayEvent::pci_config_read(0x02, 2, 0x1001))
                .unwrap_err();

            assert_eq!(
                err,
                ReplayMismatch::Value {
                    expected: 0x1000,
                    observed: 0x1001
                }
            );
            assert_eq!(scoreboard.mismatch(), Some(err));
        }

        #[test]
        fn replay_scoreboard_rejects_wrong_offset() {
            let mut scoreboard = ReplayScoreboard::new(&VIRTIO_NET_INIT_TRACE);

            let err = scoreboard
                .record(PciReplayEvent::pci_config_read(0x04, 2, 0x1af4))
                .unwrap_err();

            assert_eq!(
                err,
                ReplayMismatch::Offset {
                    expected: 0x00,
                    observed: 0x04
                }
            );
        }

        #[test]
        fn replay_scoreboard_rejects_incomplete_trace_on_finish() {
            let mut scoreboard = ReplayScoreboard::new(&VIRTIO_NET_INIT_TRACE);

            scoreboard.record(VIRTIO_NET_INIT_TRACE[0]).unwrap();
            let err = scoreboard.finish().unwrap_err();

            assert_eq!(
                err,
                ReplayMismatch::Incomplete {
                    expected_total: 4,
                    observed_total: 1
                }
            );
        }

        #[test]
        fn replay_scoreboard_rejects_extra_access_after_trace_end() {
            let trace = [PciReplayEvent::mmio_write(0, 0x10, 4, 1)];
            let mut scoreboard = ReplayScoreboard::new(&trace);

            scoreboard.record(trace[0]).unwrap();
            let err = scoreboard
                .record(PciReplayEvent::mmio_write(0, 0x10, 4, 1))
                .unwrap_err();

            assert_eq!(
                err,
                ReplayMismatch::UnexpectedAccess {
                    observed: PciReplayEvent::mmio_write(0, 0x10, 4, 1)
                }
            );
        }

        #[test]
        fn mock_pci_device_returns_oracle_read_values_and_scores_writes() {
            let mut device = MockPciDevice::new(&VIRTIO_NET_INIT_TRACE);

            assert_eq!(device.pci_config_read(0x00, 2).unwrap(), 0x1af4);
            assert_eq!(device.pci_config_read(0x02, 2).unwrap(), 0x1000);
            assert_eq!(device.mmio_read(0, 0x12, 2).unwrap(), 0x0000);
            device.mmio_write(0, 0x12, 2, 0x0001).unwrap();

            assert_eq!(device.scoreboard().observed_count(), 4);
            device.finish().unwrap();
        }

        #[test]
        fn mock_pci_device_rejects_wrong_read_shape_before_returning_value() {
            let mut device = MockPciDevice::new(&VIRTIO_NET_INIT_TRACE);

            let err = device.pci_config_read(0x04, 2).unwrap_err();

            assert_eq!(
                err,
                ReplayMismatch::Offset {
                    expected: 0x00,
                    observed: 0x04
                }
            );
        }
    }
}

pub mod packet_harness {

    pub const NET_STATUS_OK: u32 = 0;
    pub const DEFAULT_PACKET_SHMEM_CAP: u64 = 0x1000;
    pub const DEFAULT_PACKET_SHMEM_SIZE: usize = 8192;

    /// Harness-level packet operation observed in an Oracle trace.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum PacketReplayOp {
        SendPacket,
        ReceivePacket,
    }

    /// One normalized harness.net packet exchange used by the replay scoreboard.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct PacketReplayEvent<'a> {
        pub op: PacketReplayOp,
        pub request_id: u64,
        pub shm_cap: u64,
        pub offset: u64,
        pub len: u32,
        pub payload: &'a [u8],
        pub status: u32,
        pub bytes: u32,
    }

    /// Deterministic mismatch reasons for harness packet replay gates.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum PacketReplayMismatch {
        UnexpectedCall {
            op: PacketReplayOp,
            request_id: u64,
        },
        Op {
            expected: PacketReplayOp,
            observed: PacketReplayOp,
        },
        RequestId {
            expected: u64,
            observed: u64,
        },
        ShmCap {
            expected: u64,
            observed: u64,
        },
        Offset {
            expected: u64,
            observed: u64,
        },
        Len {
            expected: u32,
            observed: u32,
        },
        Payload,
        Status {
            expected: u32,
            observed: u32,
        },
        Bytes {
            expected: u32,
            observed: u32,
        },
        ShmWrite,
        ShmRead,
        Incomplete {
            expected_total: usize,
            observed_total: usize,
        },
    }

    pub type PacketReplayResult<T> = Result<T, PacketReplayMismatch>;

    /// Fixed shared-memory backing store for harness packet replay.
    pub struct MockPacketShmem {
        cap: u64,
        buffer: [u8; DEFAULT_PACKET_SHMEM_SIZE],
    }

    impl MockPacketShmem {
        pub const fn new(cap: u64) -> Self {
            Self {
                cap,
                buffer: [0; DEFAULT_PACKET_SHMEM_SIZE],
            }
        }

        pub const fn cap(&self) -> u64 {
            self.cap
        }

        pub fn write(&mut self, cap: u64, offset: u64, data: &[u8]) -> PacketReplayResult<()> {
            if cap != self.cap {
                return Err(PacketReplayMismatch::ShmCap {
                    expected: self.cap,
                    observed: cap,
                });
            }
            let end = offset as usize + data.len();
            if end > self.buffer.len() {
                return Err(PacketReplayMismatch::ShmWrite);
            }
            self.buffer[offset as usize..end].copy_from_slice(data);
            Ok(())
        }

        pub fn read(&self, cap: u64, offset: u64, len: u32) -> PacketReplayResult<&[u8]> {
            if cap != self.cap {
                return Err(PacketReplayMismatch::ShmCap {
                    expected: self.cap,
                    observed: cap,
                });
            }
            let end = offset as usize + len as usize;
            if end > self.buffer.len() {
                return Err(PacketReplayMismatch::ShmRead);
            }
            Ok(&self.buffer[offset as usize..end])
        }

        pub fn slice(&self, cap: u64, offset: u64, len: u32) -> PacketReplayResult<&[u8]> {
            self.read(cap, offset, len)
        }
    }

    /// Compares harness packet calls against a fixed Oracle trace.
    pub struct PacketReplayScoreboard<'a> {
        expected: &'a [PacketReplayEvent<'a>],
        cursor: usize,
        mismatch: Option<PacketReplayMismatch>,
    }

    impl<'a> PacketReplayScoreboard<'a> {
        pub const fn new(expected: &'a [PacketReplayEvent<'a>]) -> Self {
            Self {
                expected,
                cursor: 0,
                mismatch: None,
            }
        }

        pub fn finish(&mut self) -> PacketReplayResult<()> {
            if let Some(mismatch) = self.mismatch {
                return Err(mismatch);
            }
            if self.cursor != self.expected.len() {
                return self.fail(PacketReplayMismatch::Incomplete {
                    expected_total: self.expected.len(),
                    observed_total: self.cursor,
                });
            }
            Ok(())
        }

        pub fn observed_count(&self) -> usize {
            self.cursor
        }

        pub fn expected_count(&self) -> usize {
            self.expected.len()
        }

        pub fn mismatch(&self) -> Option<PacketReplayMismatch> {
            self.mismatch
        }

        fn fail<T>(&mut self, mismatch: PacketReplayMismatch) -> PacketReplayResult<T> {
            self.mismatch = Some(mismatch);
            Err(mismatch)
        }

        fn next_expected(&mut self) -> PacketReplayResult<PacketReplayEvent<'a>> {
            if let Some(mismatch) = self.mismatch {
                return Err(mismatch);
            }
            self.expected.get(self.cursor).copied().ok_or_else(|| {
                let mismatch = PacketReplayMismatch::UnexpectedCall {
                    op: PacketReplayOp::SendPacket,
                    request_id: 0,
                };
                self.mismatch = Some(mismatch);
                mismatch
            })
        }

        fn advance(&mut self) {
            self.cursor += 1;
        }
    }

    /// Mock harness.net endpoint that scores send/receive calls against Oracle events.
    pub struct MockPacketHarness<'a> {
        scoreboard: PacketReplayScoreboard<'a>,
        shmem: MockPacketShmem,
    }

    impl<'a> MockPacketHarness<'a> {
        pub fn new(expected: &'a [PacketReplayEvent<'a>], shmem_cap: u64) -> Self {
            Self {
                scoreboard: PacketReplayScoreboard::new(expected),
                shmem: MockPacketShmem::new(shmem_cap),
            }
        }

        pub fn shmem(&mut self) -> &mut MockPacketShmem {
            &mut self.shmem
        }

        pub fn send_packet(
            &mut self,
            request: crate::generated::SendPacket,
        ) -> PacketReplayResult<crate::generated::SendPacketReply> {
            let expected = self.scoreboard.next_expected()?;
            if expected.op != PacketReplayOp::SendPacket {
                return self.scoreboard.fail(PacketReplayMismatch::Op {
                    expected: expected.op,
                    observed: PacketReplayOp::SendPacket,
                });
            }
            if expected.request_id != request.request_id {
                return self.scoreboard.fail(PacketReplayMismatch::RequestId {
                    expected: expected.request_id,
                    observed: request.request_id,
                });
            }
            if expected.shm_cap != request.data_shm_cap {
                return self.scoreboard.fail(PacketReplayMismatch::ShmCap {
                    expected: expected.shm_cap,
                    observed: request.data_shm_cap,
                });
            }
            if expected.offset != request.data_offset {
                return self.scoreboard.fail(PacketReplayMismatch::Offset {
                    expected: expected.offset,
                    observed: request.data_offset,
                });
            }
            if expected.len != request.data_len {
                return self.scoreboard.fail(PacketReplayMismatch::Len {
                    expected: expected.len,
                    observed: request.data_len,
                });
            }

            let observed_payload =
                self.shmem
                    .read(request.data_shm_cap, request.data_offset, request.data_len)?;
            if observed_payload != expected.payload {
                return self.scoreboard.fail(PacketReplayMismatch::Payload);
            }

            self.scoreboard.advance();
            Ok(crate::generated::SendPacketReply {
                request_id: request.request_id,
                status: expected.status,
                bytes_sent: expected.bytes,
            })
        }

        pub fn receive_packet(
            &mut self,
            request: crate::generated::ReceivePacket,
        ) -> PacketReplayResult<crate::generated::ReceivePacketReply> {
            let expected = self.scoreboard.next_expected()?;
            if expected.op != PacketReplayOp::ReceivePacket {
                return self.scoreboard.fail(PacketReplayMismatch::Op {
                    expected: expected.op,
                    observed: PacketReplayOp::ReceivePacket,
                });
            }
            if expected.request_id != request.request_id {
                return self.scoreboard.fail(PacketReplayMismatch::RequestId {
                    expected: expected.request_id,
                    observed: request.request_id,
                });
            }
            if expected.shm_cap != request.buffer_shm_cap {
                return self.scoreboard.fail(PacketReplayMismatch::ShmCap {
                    expected: expected.shm_cap,
                    observed: request.buffer_shm_cap,
                });
            }
            if expected.offset != request.buffer_offset {
                return self.scoreboard.fail(PacketReplayMismatch::Offset {
                    expected: expected.offset,
                    observed: request.buffer_offset,
                });
            }
            if request.buffer_len < expected.len {
                return self.scoreboard.fail(PacketReplayMismatch::Len {
                    expected: expected.len,
                    observed: request.buffer_len,
                });
            }

            self.shmem.write(
                request.buffer_shm_cap,
                request.buffer_offset,
                expected.payload,
            )?;

            self.scoreboard.advance();
            Ok(crate::generated::ReceivePacketReply {
                request_id: request.request_id,
                status: expected.status,
                bytes_received: expected.bytes,
            })
        }

        pub fn finish(&mut self) -> PacketReplayResult<()> {
            self.scoreboard.finish()
        }

        pub fn scoreboard(&self) -> &PacketReplayScoreboard<'a> {
            &self.scoreboard
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::generated::{ReceivePacket, SendPacket};

        const SEND_PAYLOAD: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
        const RECV_PAYLOAD: [u8; 4] = [0xca, 0xfe, 0xba, 0xbe];

        const PACKET_TRACE: [PacketReplayEvent<'_>; 2] = [
            PacketReplayEvent {
                op: PacketReplayOp::SendPacket,
                request_id: 1,
                shm_cap: DEFAULT_PACKET_SHMEM_CAP,
                offset: 0,
                len: 4,
                payload: &SEND_PAYLOAD,
                status: NET_STATUS_OK,
                bytes: 4,
            },
            PacketReplayEvent {
                op: PacketReplayOp::ReceivePacket,
                request_id: 2,
                shm_cap: DEFAULT_PACKET_SHMEM_CAP,
                offset: 256,
                len: 4,
                payload: &RECV_PAYLOAD,
                status: NET_STATUS_OK,
                bytes: 4,
            },
        ];

        #[test]
        fn mock_packet_harness_replays_send_and_receive() {
            let mut harness = MockPacketHarness::new(&PACKET_TRACE, DEFAULT_PACKET_SHMEM_CAP);
            harness
                .shmem()
                .write(DEFAULT_PACKET_SHMEM_CAP, 0, &SEND_PAYLOAD)
                .unwrap();

            let send_reply = harness
                .send_packet(SendPacket {
                    request_id: 1,
                    data_shm_cap: DEFAULT_PACKET_SHMEM_CAP,
                    data_offset: 0,
                    data_len: 4,
                })
                .unwrap();
            assert_eq!(send_reply.status, NET_STATUS_OK);
            assert_eq!(send_reply.bytes_sent, 4);

            let recv_reply = harness
                .receive_packet(ReceivePacket {
                    request_id: 2,
                    buffer_shm_cap: DEFAULT_PACKET_SHMEM_CAP,
                    buffer_offset: 256,
                    buffer_len: 1500,
                })
                .unwrap();
            assert_eq!(recv_reply.status, NET_STATUS_OK);
            assert_eq!(recv_reply.bytes_received, 4);
            assert_eq!(
                harness
                    .shmem()
                    .read(DEFAULT_PACKET_SHMEM_CAP, 256, 4)
                    .unwrap(),
                &RECV_PAYLOAD
            );

            harness.finish().unwrap();
        }

        #[test]
        fn mock_packet_harness_rejects_payload_mismatch() {
            let mut harness = MockPacketHarness::new(&PACKET_TRACE, DEFAULT_PACKET_SHMEM_CAP);
            harness
                .shmem()
                .write(DEFAULT_PACKET_SHMEM_CAP, 0, &[0x00, 0x00, 0x00, 0x00])
                .unwrap();

            let err = harness
                .send_packet(SendPacket {
                    request_id: 1,
                    data_shm_cap: DEFAULT_PACKET_SHMEM_CAP,
                    data_offset: 0,
                    data_len: 4,
                })
                .unwrap_err();

            assert_eq!(err, PacketReplayMismatch::Payload);
        }
    }
}

pub mod block_harness {
    pub const BLOCK_STATUS_OK: u32 = 0;
    pub const DEFAULT_BLOCK_SHMEM_CAP: u64 = 4096;
    pub const DEFAULT_BLOCK_SHMEM_SIZE: usize = 4096;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum BlockReplayOp {
        ReadBlocks,
        WriteBlocks,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct BlockReplayEvent<'a> {
        pub op: BlockReplayOp,
        pub request_id: u64,
        pub lba: u64,
        pub block_count: u32,
        pub block_size: u32,
        pub shm_cap: u64,
        pub offset: u64,
        pub len: u32,
        pub payload: &'a [u8],
        pub status: u32,
        pub bytes: u32,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum BlockReplayMismatch {
        UnexpectedCall {
            op: BlockReplayOp,
            request_id: u64,
        },
        Op {
            expected: BlockReplayOp,
            observed: BlockReplayOp,
        },
        RequestId {
            expected: u64,
            observed: u64,
        },
        Lba {
            expected: u64,
            observed: u64,
        },
        BlockCount {
            expected: u32,
            observed: u32,
        },
        BlockSize {
            expected: u32,
            observed: u32,
        },
        ShmCap {
            expected: u64,
            observed: u64,
        },
        Offset {
            expected: u64,
            observed: u64,
        },
        Len {
            expected: u32,
            observed: u32,
        },
        Payload,
        Status {
            expected: u32,
            observed: u32,
        },
        Bytes {
            expected: u32,
            observed: u32,
        },
        ShmWrite,
        ShmRead,
        Incomplete {
            expected_total: usize,
            observed_total: usize,
        },
    }

    pub type BlockReplayResult<T> = Result<T, BlockReplayMismatch>;

    pub struct MockBlockShmem {
        cap: u64,
        buffer: [u8; DEFAULT_BLOCK_SHMEM_SIZE],
    }

    impl MockBlockShmem {
        pub const fn new(cap: u64) -> Self {
            Self {
                cap,
                buffer: [0; DEFAULT_BLOCK_SHMEM_SIZE],
            }
        }

        pub const fn cap(&self) -> u64 {
            self.cap
        }

        pub fn write(&mut self, cap: u64, offset: u64, data: &[u8]) -> BlockReplayResult<()> {
            if cap != self.cap {
                return Err(BlockReplayMismatch::ShmCap {
                    expected: self.cap,
                    observed: cap,
                });
            }
            let end = offset as usize + data.len();
            if end > self.buffer.len() {
                return Err(BlockReplayMismatch::ShmWrite);
            }
            self.buffer[offset as usize..end].copy_from_slice(data);
            Ok(())
        }

        pub fn read(&self, cap: u64, offset: u64, len: u32) -> BlockReplayResult<&[u8]> {
            if cap != self.cap {
                return Err(BlockReplayMismatch::ShmCap {
                    expected: self.cap,
                    observed: cap,
                });
            }
            let end = offset as usize + len as usize;
            if end > self.buffer.len() {
                return Err(BlockReplayMismatch::ShmRead);
            }
            Ok(&self.buffer[offset as usize..end])
        }
    }

    pub struct BlockReplayScoreboard<'a> {
        expected: &'a [BlockReplayEvent<'a>],
        cursor: usize,
        mismatch: Option<BlockReplayMismatch>,
    }

    impl<'a> BlockReplayScoreboard<'a> {
        pub const fn new(expected: &'a [BlockReplayEvent<'a>]) -> Self {
            Self {
                expected,
                cursor: 0,
                mismatch: None,
            }
        }

        pub fn finish(&mut self) -> BlockReplayResult<()> {
            if let Some(mismatch) = self.mismatch {
                return Err(mismatch);
            }
            if self.cursor != self.expected.len() {
                return Err(BlockReplayMismatch::Incomplete {
                    expected_total: self.expected.len(),
                    observed_total: self.cursor,
                });
            }
            Ok(())
        }

        fn fail<T>(&mut self, mismatch: BlockReplayMismatch) -> BlockReplayResult<T> {
            self.mismatch = Some(mismatch);
            Err(mismatch)
        }

        fn next_expected(&mut self) -> BlockReplayResult<BlockReplayEvent<'a>> {
            if let Some(mismatch) = self.mismatch {
                return Err(mismatch);
            }
            self.expected.get(self.cursor).copied().ok_or_else(|| {
                let mismatch = BlockReplayMismatch::UnexpectedCall {
                    op: BlockReplayOp::ReadBlocks,
                    request_id: 0,
                };
                self.mismatch = Some(mismatch);
                mismatch
            })
        }

        fn advance(&mut self) {
            self.cursor += 1;
        }
    }

    pub struct MockBlockHarness<'a> {
        scoreboard: BlockReplayScoreboard<'a>,
        shmem: MockBlockShmem,
    }

    impl<'a> MockBlockHarness<'a> {
        pub fn new(expected: &'a [BlockReplayEvent<'a>], shmem_cap: u64) -> Self {
            Self {
                scoreboard: BlockReplayScoreboard::new(expected),
                shmem: MockBlockShmem::new(shmem_cap),
            }
        }

        pub fn shmem(&mut self) -> &mut MockBlockShmem {
            &mut self.shmem
        }

        pub fn read_blocks(
            &mut self,
            request: crate::generated::ReadBlocks,
        ) -> BlockReplayResult<crate::generated::ReadBlocksReply> {
            let expected = self.scoreboard.next_expected()?;
            if expected.op != BlockReplayOp::ReadBlocks {
                return self.scoreboard.fail(BlockReplayMismatch::Op {
                    expected: expected.op,
                    observed: BlockReplayOp::ReadBlocks,
                });
            }
            if expected.request_id != request.request_id {
                return self.scoreboard.fail(BlockReplayMismatch::RequestId {
                    expected: expected.request_id,
                    observed: request.request_id,
                });
            }
            if expected.lba != request.lba {
                return self.scoreboard.fail(BlockReplayMismatch::Lba {
                    expected: expected.lba,
                    observed: request.lba,
                });
            }
            if expected.block_count != request.block_count {
                return self.scoreboard.fail(BlockReplayMismatch::BlockCount {
                    expected: expected.block_count,
                    observed: request.block_count,
                });
            }
            if expected.block_size != request.block_size {
                return self.scoreboard.fail(BlockReplayMismatch::BlockSize {
                    expected: expected.block_size,
                    observed: request.block_size,
                });
            }
            if expected.shm_cap != request.buffer_shm_cap {
                return self.scoreboard.fail(BlockReplayMismatch::ShmCap {
                    expected: expected.shm_cap,
                    observed: request.buffer_shm_cap,
                });
            }
            if expected.offset != request.buffer_offset {
                return self.scoreboard.fail(BlockReplayMismatch::Offset {
                    expected: expected.offset,
                    observed: request.buffer_offset,
                });
            }
            let byte_len = request.block_size.checked_mul(request.block_count).ok_or(
                BlockReplayMismatch::Len {
                    expected: expected.len,
                    observed: 0,
                },
            )?;
            if byte_len != expected.len {
                return self.scoreboard.fail(BlockReplayMismatch::Len {
                    expected: expected.len,
                    observed: byte_len,
                });
            }

            self.shmem.write(
                request.buffer_shm_cap,
                request.buffer_offset,
                expected.payload,
            )?;

            self.scoreboard.advance();
            Ok(crate::generated::ReadBlocksReply {
                request_id: request.request_id,
                status: expected.status,
                bytes_read: expected.bytes,
            })
        }

        pub fn write_blocks(
            &mut self,
            request: crate::generated::WriteBlocks,
        ) -> BlockReplayResult<crate::generated::WriteBlocksReply> {
            let expected = self.scoreboard.next_expected()?;
            if expected.op != BlockReplayOp::WriteBlocks {
                return self.scoreboard.fail(BlockReplayMismatch::Op {
                    expected: expected.op,
                    observed: BlockReplayOp::WriteBlocks,
                });
            }
            if expected.request_id != request.request_id {
                return self.scoreboard.fail(BlockReplayMismatch::RequestId {
                    expected: expected.request_id,
                    observed: request.request_id,
                });
            }
            if expected.lba != request.lba {
                return self.scoreboard.fail(BlockReplayMismatch::Lba {
                    expected: expected.lba,
                    observed: request.lba,
                });
            }
            if expected.block_count != request.block_count {
                return self.scoreboard.fail(BlockReplayMismatch::BlockCount {
                    expected: expected.block_count,
                    observed: request.block_count,
                });
            }
            if expected.block_size != request.block_size {
                return self.scoreboard.fail(BlockReplayMismatch::BlockSize {
                    expected: expected.block_size,
                    observed: request.block_size,
                });
            }
            if expected.shm_cap != request.data_shm_cap {
                return self.scoreboard.fail(BlockReplayMismatch::ShmCap {
                    expected: expected.shm_cap,
                    observed: request.data_shm_cap,
                });
            }
            if expected.offset != request.data_offset {
                return self.scoreboard.fail(BlockReplayMismatch::Offset {
                    expected: expected.offset,
                    observed: request.data_offset,
                });
            }
            let byte_len = request.block_size.checked_mul(request.block_count).ok_or(
                BlockReplayMismatch::Len {
                    expected: expected.len,
                    observed: 0,
                },
            )?;
            if byte_len != expected.len {
                return self.scoreboard.fail(BlockReplayMismatch::Len {
                    expected: expected.len,
                    observed: byte_len,
                });
            }

            let observed = self
                .shmem
                .read(request.data_shm_cap, request.data_offset, byte_len)?;
            if observed != expected.payload {
                return self.scoreboard.fail(BlockReplayMismatch::Payload);
            }

            self.scoreboard.advance();
            Ok(crate::generated::WriteBlocksReply {
                request_id: request.request_id,
                status: expected.status,
                bytes_written: expected.bytes,
            })
        }

        pub fn finish(&mut self) -> BlockReplayResult<()> {
            self.scoreboard.finish()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::generated::{ReadBlocks, WriteBlocks};

        const READ_PAYLOAD: [u8; 4] = [0x13, 0x26, 0x39, 0x4c];
        const WRITE_PAYLOAD: [u8; 4] = [0x37, 0x6e, 0xa5, 0xdc];

        const SECTOR_TRACE: [BlockReplayEvent<'_>; 2] = [
            BlockReplayEvent {
                op: BlockReplayOp::ReadBlocks,
                request_id: 1,
                lba: 0,
                block_count: 1,
                block_size: 4,
                shm_cap: DEFAULT_BLOCK_SHMEM_CAP,
                offset: 0,
                len: 4,
                payload: &READ_PAYLOAD,
                status: BLOCK_STATUS_OK,
                bytes: 4,
            },
            BlockReplayEvent {
                op: BlockReplayOp::WriteBlocks,
                request_id: 2,
                lba: 1,
                block_count: 1,
                block_size: 4,
                shm_cap: DEFAULT_BLOCK_SHMEM_CAP,
                offset: 512,
                len: 4,
                payload: &WRITE_PAYLOAD,
                status: BLOCK_STATUS_OK,
                bytes: 4,
            },
        ];

        #[test]
        fn mock_block_harness_replays_read_and_write() {
            let mut harness = MockBlockHarness::new(&SECTOR_TRACE, DEFAULT_BLOCK_SHMEM_CAP);

            let read_reply = harness
                .read_blocks(ReadBlocks {
                    request_id: 1,
                    lba: 0,
                    block_count: 1,
                    block_size: 4,
                    buffer_shm_cap: DEFAULT_BLOCK_SHMEM_CAP,
                    buffer_offset: 0,
                })
                .unwrap();
            assert_eq!(read_reply.status, BLOCK_STATUS_OK);
            assert_eq!(read_reply.bytes_read, 4);
            assert_eq!(
                harness.shmem().read(DEFAULT_BLOCK_SHMEM_CAP, 0, 4).unwrap(),
                &READ_PAYLOAD
            );

            harness
                .shmem()
                .write(DEFAULT_BLOCK_SHMEM_CAP, 512, &WRITE_PAYLOAD)
                .unwrap();
            let write_reply = harness
                .write_blocks(WriteBlocks {
                    request_id: 2,
                    lba: 1,
                    block_count: 1,
                    block_size: 4,
                    data_shm_cap: DEFAULT_BLOCK_SHMEM_CAP,
                    data_offset: 512,
                })
                .unwrap();
            assert_eq!(write_reply.status, BLOCK_STATUS_OK);
            assert_eq!(write_reply.bytes_written, 4);

            harness.finish().unwrap();
        }
    }
}
