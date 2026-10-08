//! Explicit ten-message Artifact368 codec; generated layout is not validation.
use super::StoreStatus;
use kernel_api::{
    cap::{Handle, HandleKind},
    generated::desktop_artifact_v1 as a,
    ipc::Envelope,
};
pub trait ArtifactMessage: Sized {
    const PROTOCOL: u32;
    const KIND: u32;
    const LENGTH: usize;
    fn encode(&self, handle: Handle) -> Result<Envelope, StoreStatus>;
    fn decode(env: &Envelope) -> Result<Self, StoreStatus>;
}
trait Field: Sized {
    fn put(self, out: &mut [u8], offset: &mut usize);
    fn get(input: &[u8], offset: &mut usize) -> Self;
}
impl Field for u64 {
    fn put(self, out: &mut [u8], offset: &mut usize) {
        out[*offset..*offset + 8].copy_from_slice(&self.to_le_bytes());
        *offset += 8;
    }
    fn get(input: &[u8], offset: &mut usize) -> Self {
        let value = Self::from_le_bytes(input[*offset..*offset + 8].try_into().unwrap());
        *offset += 8;
        value
    }
}
impl Field for u32 {
    fn put(self, out: &mut [u8], offset: &mut usize) {
        out[*offset..*offset + 4].copy_from_slice(&self.to_le_bytes());
        *offset += 4;
    }
    fn get(input: &[u8], offset: &mut usize) -> Self {
        let value = Self::from_le_bytes(input[*offset..*offset + 4].try_into().unwrap());
        *offset += 4;
        value
    }
}
macro_rules! message {($name:ident,$kind:expr,$len:expr,{$($field:ident),+})=>{impl ArtifactMessage for a::$name {const PROTOCOL:u32=368;const KIND:u32=$kind;const LENGTH:usize=$len;fn encode(&self,handle:Handle)->Result<Envelope,StoreStatus>{let mut env=Envelope::empty(368,$kind);env.handle=handle;env.payload_len=$len;let mut offset=0;$(self.$field.put(&mut env.payload,&mut offset);)+assert_eq!(offset,$len as usize);validate(&env)?;Ok(env)}fn decode(env:&Envelope)->Result<Self,StoreStatus>{validate(env)?;if env.protocol!=368||env.msg_type!=$kind {return Err(StoreStatus::Invalid)}let mut offset=0;let value=Self {$($field:Field::get(&env.payload,&mut offset)),+};assert_eq!(offset,$len as usize);Ok(value)}}};}
message!(ReadSelected,1,40,{request_id,session_id,session_generation,instance_id,instance_generation});
message!(ReadSelectedReply,2,40,{request_id,revision,data_shm,object_generation,byte_len,status});
message!(AllocateSaveId,3,32,{request_id,session_id,session_generation,expected_revision});
message!(AllocateSaveIdReply,4,24,{request_id,operation_id,status,reserved});
message!(Commit,5,56,{request_id,session_id,session_generation,expected_revision,operation_id,source_shm,byte_len,reserved});
message!(CommitReply,6,40,{request_id,operation_id,revision,receipt_shm,status,receipt_len});
message!(SaveStatus,7,32,{request_id,session_id,session_generation,operation_id});
message!(SaveStatusReply,8,40,{request_id,operation_id,revision,receipt_shm,status,receipt_len});
message!(ObserveAllocation,9,32,{request_id,session_id,session_generation,original_allocate_request_id});
message!(ObserveAllocationReply,10,32,{request_id,original_allocate_request_id,operation_id,status,reserved});
pub(super) fn validate(env: &Envelope) -> Result<(), StoreStatus> {
    if env.protocol != 368 {
        return Err(StoreStatus::Unsupported);
    }
    let (len, reserved, status) = match env.msg_type {
        1 => (40, None, None),
        2 => (40, None, Some(36)),
        3 => (32, None, None),
        4 => (24, Some(20), Some(16)),
        5 => (56, Some(52), None),
        6 => (40, None, Some(32)),
        7 => (32, None, None),
        8 => (40, None, Some(32)),
        9 => (32, None, None),
        10 => (32, Some(28), Some(24)),
        _ => return Err(StoreStatus::Unsupported),
    };
    if env.payload_len != len
        || env.payload[len as usize..].iter().any(|b| *b != 0)
        || reserved.is_some_and(|o| env.payload[o..o + 4].iter().any(|b| *b != 0))
        || status
            .is_some_and(|o| u32::from_le_bytes(env.payload[o..o + 4].try_into().unwrap()) > 11)
    {
        return Err(StoreStatus::Invalid);
    }
    let h = env.handle;
    if h != Handle::INVALID
        && (h.kind == HandleKind::Invalid
            || h.index == 0
            || h.index > 65535
            || h.generation == 0
            || h.generation > u32::MAX as u64)
    {
        return Err(StoreStatus::Invalid);
    }
    Ok(())
}
pub fn encode_envelope_wire(env: &Envelope) -> Result<[u8; 88], StoreStatus> {
    validate(env)?;
    let mut out = [0; 88];
    out[..4].copy_from_slice(&env.protocol.to_le_bytes());
    out[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    out[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    out[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    out[20..84].copy_from_slice(&env.payload);
    Ok(out)
}
pub fn decode_envelope_wire(raw: &[u8]) -> Result<Envelope, StoreStatus> {
    if raw.len() != 88 || raw[84..].iter().any(|b| *b != 0) {
        return Err(StoreStatus::Invalid);
    }
    let packed = u64::from_le_bytes(raw[8..16].try_into().unwrap());
    if packed & 0x00ff_0000_0000_0000 != 0 || packed >> 56 > 3 {
        return Err(StoreStatus::Invalid);
    }
    let mut env = Envelope::empty(
        u32::from_le_bytes(raw[..4].try_into().unwrap()),
        u32::from_le_bytes(raw[4..8].try_into().unwrap()),
    );
    env.handle = Handle::unpack(packed);
    if env.handle.pack() != packed {
        return Err(StoreStatus::Invalid);
    }
    env.payload_len = u32::from_le_bytes(raw[16..20].try_into().unwrap());
    env.payload.copy_from_slice(&raw[20..84]);
    validate(&env)?;
    Ok(env)
}
