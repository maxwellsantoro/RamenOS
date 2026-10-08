//! Default-off, trusted host fixtures for UI1.0. No target or containment claim.
use kernel_api::cap::{Handle, HandleKind};
use kernel_api::generated::desktop_session_v1 as idl;
use kernel_api::ipc::Envelope;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const OBSERVE: u32 = 1;
const CHROME_RIGHTS: u32 = 127;
const RUNNING: u32 = 2;
const EXITED: u32 = 3;
const FAULTED: u32 = 4;
const REVOKED: u32 = 5;
const EXPIRED: u32 = 6;
const MAX_INSTANCES: usize = 16;
const MAX_EXCHANGES: usize = 64;
const MAX_ENDPOINTS: usize = 64;
const MAX_EXECUTABLE: u64 = 64 * 1024 * 1024;
// Freshness allocation only; authority/trace/accounting live in each registry.
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    Denied = 1,
    Invalid = 2,
    Unsupported = 3,
    Stale = 4,
    Exhausted = 5,
    NotReady = 6,
    Conflict = 7,
    Timeout = 8,
    Disconnected = 9,
    Unknown = 10,
    Internal = 11,
}

#[derive(Clone, Copy, Debug)]
pub enum WitnessMode {
    Observe,
    Fault,
    Stall,
    Flood,
}

#[derive(Clone, Debug)]
pub struct DevConfig {
    pub witness_path: PathBuf,
    pub pinned_sha256: [u8; 32],
    pub preview_ttl_ms: u64,
    pub instance_ttl_ms: u64,
    pub child_deadline_ms: u64,
    pub witness_mode: WitnessMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PeerClass {
    Chrome,
    Child,
}

/// Created only by this fixture broker or from an actual owned Child/pipe.
#[derive(Clone, Debug)]
pub struct PeerContext {
    registry: u64,
    owner: u64,
    session: u64,
    instance: u64,
    endpoint: Handle,
    class: PeerClass,
}

pub struct SessionFixture {
    pub trusted_peer: PeerContext,
    pub trusted_handle: Handle,
    pub session_id: u64,
    pub session_generation: u64,
}

#[derive(Clone, Debug)]
pub struct PreviewSnapshot {
    pub session_id: u64,
    pub session_generation: u64,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub selected_revision: u64,
    pub policy_revision: u64,
    pub expires_at_ms: u64,
    pub application_hash: [u8; 32],
    pub selected_fixture_hash: [u8; 32],
    pub schema_version: u32,
    pub granted_rights: u32,
}

#[derive(Clone, Debug)]
pub struct ChildEvidence {
    pub actual_pid: u32,
    pub executable_sha256: [u8; 32],
    pub bootstrap: idl::InstanceBootstrap,
    pub exchanges: Vec<(Envelope, Envelope)>,
    pub observation_count: u32,
    pub reaped: bool,
    pub exchange_limit_hit: bool,
}

// IDL objects are encoded per-field. Never copy repr(C) padding as wire data.
pub struct MessageWriter {
    env: Envelope,
    offset: usize,
}
impl MessageWriter {
    fn append(&mut self, bytes: &[u8]) {
        let end = self.offset + bytes.len();
        self.env.payload[self.offset..end].copy_from_slice(bytes);
        self.offset = end;
    }
    fn u64(&mut self, value: &u64) {
        self.append(&value.to_le_bytes());
    }
    fn u32(&mut self, value: &u32) {
        self.append(&value.to_le_bytes());
    }
    fn bytes32(&mut self, value: &[u8; 32]) {
        self.append(value);
    }
}
struct MessageReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl MessageReader<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let end = self.offset + N;
        let out = self.bytes[self.offset..end]
            .try_into()
            .expect("validated typed length");
        self.offset = end;
        out
    }
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.take())
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }
    fn bytes32(&mut self) -> [u8; 32] {
        self.take()
    }
}

pub trait DesktopMessage: Sized {
    const KIND: u32;
    const LENGTH: usize;
    fn write(&self, out: &mut MessageWriter);
    fn from_envelope(env: &Envelope) -> Result<Self, Status>;
}
macro_rules! message {
    ($ty:ident, $kind:ident, $len:expr; $( $format:ident $field:ident ),+ $(,)?) => {
        impl DesktopMessage for idl::$ty {
            const KIND: u32 = idl::$kind;
            const LENGTH: usize = $len;
            fn write(&self, out: &mut MessageWriter) { $(out.$format(&self.$field);)+ }
            fn from_envelope(env: &Envelope) -> Result<Self, Status> {
                validate_envelope(env)?;
                if env.msg_type != Self::KIND { return Err(Status::Unsupported); }
                let mut reader = MessageReader { bytes: &env.payload, offset: 0 };
                Ok(Self { $($field: reader.$format(),)+ })
            }
        }
    };
}
message!(PrepareLaunch, MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH, 64;
    u64 request_id, u64 session_id, u64 session_generation, bytes32 application_hash, u32 requested_rights, u32 reserved);
message!(PrepareLaunchReply, MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY, 48;
    u64 request_id, u64 plan_id, u64 preview_revision, u64 preview_shm, u32 preview_len, u32 status, u32 granted_rights, u32 reserved);
message!(CancelPreview, MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW, 32;
    u64 request_id, u64 session_id, u64 session_generation, u64 plan_id);
message!(CancelPreviewReply, MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW_REPLY, 16;
    u64 request_id, u32 status, u32 reserved);
message!(ConfirmLaunch, MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH, 40;
    u64 request_id, u64 session_id, u64 session_generation, u64 plan_id, u64 preview_revision);
message!(ConfirmLaunchReply, MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH_REPLY, 48;
    u64 request_id, u64 instance_id, u64 instance_generation, u64 observation_handle, u32 granted_rights, u32 status, u32 child_pid, u32 reserved);
message!(GetStatus, MSG_DESKTOP_SESSION_V1_GET_STATUS, 40;
    u64 request_id, u64 session_id, u64 session_generation, u64 instance_id, u64 instance_generation);
message!(GetStatusReply, MSG_DESKTOP_SESSION_V1_GET_STATUS_REPLY, 64;
    u64 request_id, u64 instance_id, u64 instance_generation, bytes32 executable_hash, u32 state, u32 status);
message!(CloseInstance, MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE, 40;
    u64 request_id, u64 session_id, u64 session_generation, u64 instance_id, u64 instance_generation);
message!(CloseInstanceReply, MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE_REPLY, 16;
    u64 request_id, u32 status, u32 reserved);
message!(RevokeInstance, MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE, 40;
    u64 request_id, u64 session_id, u64 session_generation, u64 instance_id, u64 instance_generation);
message!(RevokeInstanceReply, MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE_REPLY, 16;
    u64 request_id, u32 status, u32 reserved);
message!(PrepareRestart, MSG_DESKTOP_SESSION_V1_PREPARE_RESTART, 40;
    u64 request_id, u64 session_id, u64 session_generation, u64 instance_id, u64 instance_generation);
message!(PrepareRestartReply, MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY, 48;
    u64 request_id, u64 plan_id, u64 preview_revision, u64 preview_shm, u32 preview_len, u32 status, u32 granted_rights, u32 reserved);
message!(ObserveInstance, MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE, 40;
    u64 request_id, u64 session_id, u64 session_generation, u64 instance_id, u64 instance_generation);
message!(ObserveInstanceReply, MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY, 64;
    u64 request_id, u64 instance_id, u64 instance_generation, bytes32 executable_hash, u32 state, u32 status);
message!(InstanceBootstrap, MSG_DESKTOP_SESSION_V1_INSTANCE_BOOTSTRAP, 48;
    u64 session_id, u64 session_generation, u64 instance_id, u64 instance_generation, u64 observation_handle, u32 granted_rights, u32 reserved);

pub fn encode_message<T: DesktopMessage>(handle: Handle, value: &T) -> Envelope {
    let mut out = MessageWriter {
        env: Envelope::empty(idl::DESKTOP_SESSION_V1_PROTOCOL_ID, T::KIND),
        offset: 0,
    };
    out.env.handle = handle;
    value.write(&mut out);
    out.env.payload_len = T::LENGTH as u32;
    out.env
}

fn layout(kind: u32) -> Result<(usize, Option<usize>), Status> {
    match kind {
        1 => Ok((64, Some(60))),
        2 | 14 => Ok((48, Some(44))),
        3 => Ok((32, None)),
        4 | 10 | 12 => Ok((16, Some(12))),
        5 | 7 | 9 | 11 | 13 | 17 => Ok((40, None)),
        6 | 19 => Ok((48, Some(44))),
        8 | 18 => Ok((64, None)),
        _ => Err(Status::Unsupported),
    }
}
fn validate_handle(handle: Handle) -> Result<(), Status> {
    if handle == Handle::INVALID {
        return Ok(());
    }
    if handle.kind == HandleKind::Invalid
        || handle.index == 0
        || handle.index > u16::MAX as u32
        || handle.generation == 0
        || handle.generation > u32::MAX as u64
    {
        return Err(Status::Invalid);
    }
    Ok(())
}
fn validate_envelope(env: &Envelope) -> Result<(), Status> {
    if env.protocol != idl::DESKTOP_SESSION_V1_PROTOCOL_ID {
        return Err(Status::Unsupported);
    }
    let (length, reserved) = layout(env.msg_type)?;
    if env.payload_len as usize != length
        || env.payload[length..].iter().any(|b| *b != 0)
        || reserved.is_some_and(|offset| env.payload[offset..offset + 4].iter().any(|b| *b != 0))
    {
        return Err(Status::Invalid);
    }
    validate_handle(env.handle)
}
fn validate_request(env: &Envelope) -> Result<(), Status> {
    validate_envelope(env)?;
    if !matches!(env.msg_type, 1 | 3 | 5 | 7 | 9 | 11 | 13 | 17) {
        return Err(Status::Unsupported);
    }
    Ok(())
}
pub fn encode_envelope_wire(env: &Envelope) -> Result<[u8; 88], Status> {
    validate_envelope(env)?;
    let mut out = [0; 88];
    out[0..4].copy_from_slice(&env.protocol.to_le_bytes());
    out[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    out[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    out[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    out[20..84].copy_from_slice(&env.payload);
    Ok(out)
}
pub fn decode_envelope_wire(bytes: &[u8; 88]) -> Result<Envelope, Status> {
    let packed = u64::from_le_bytes(bytes[8..16].try_into().expect("fixed header"));
    // Check raw reserved bits and discriminant before Handle::unpack normalizes.
    if packed & 0x00ff_0000_0000_0000 != 0
        || packed >> 56 > 3
        || bytes[84..].iter().any(|b| *b != 0)
    {
        return Err(Status::Invalid);
    }
    let mut env = Envelope::empty(
        u32::from_le_bytes(bytes[0..4].try_into().expect("fixed header")),
        u32::from_le_bytes(bytes[4..8].try_into().expect("fixed header")),
    );
    env.handle = Handle::unpack(packed);
    env.payload_len = u32::from_le_bytes(bytes[16..20].try_into().expect("fixed header"));
    env.payload.copy_from_slice(&bytes[20..84]);
    if env.handle.pack() != packed {
        return Err(Status::Invalid);
    }
    validate_envelope(&env)?;
    Ok(env)
}
pub fn read_frame(input: &mut impl Read) -> Result<Envelope, Status> {
    let mut length = [0; 4];
    input
        .read_exact(&mut length)
        .map_err(|_| Status::Disconnected)?;
    if u32::from_le_bytes(length) != 88 {
        return Err(Status::Invalid);
    }
    let mut bytes = [0; 88];
    input
        .read_exact(&mut bytes)
        .map_err(|_| Status::Disconnected)?;
    decode_envelope_wire(&bytes)
}
pub fn write_frame(output: &mut impl Write, env: &Envelope) -> Result<(), Status> {
    let bytes = encode_envelope_wire(env)?;
    output
        .write_all(&88u32.to_le_bytes())
        .and_then(|_| output.write_all(&bytes))
        .and_then(|_| output.flush())
        .map_err(|_| Status::Disconnected)
}

struct Endpoint {
    peer: PeerContext,
    rights: u32,
    active: bool,
}
struct Plan {
    snapshot: PreviewSnapshot,
    object: [u8; 136],
    handle: u64,
    event: bool,
}
struct Session {
    owner: u64,
    generation: u64,
    selected_hash: [u8; 32],
    selected_revision: u64,
    policy_revision: u64,
    preview: Option<Plan>,
    current: Option<u64>,
}
struct Instance {
    session: u64,
    generation: u64,
    state: u32,
    expires: u64,
    peer: PeerContext,
    stop: Arc<AtomicBool>,
    evidence: ChildEvidence,
}
struct Registry {
    identity: u64,
    now: u64,
    next_session: u64,
    next_plan: u64,
    next_instance: u64,
    next_generation: u64,
    next_endpoint: u64,
    sessions: HashMap<u64, Session>,
    endpoints: HashMap<u64, Endpoint>,
    instances: HashMap<u64, Instance>,
}
type Shared = Arc<Mutex<Registry>>;

fn take_counter(counter: &mut u64) -> Result<u64, Status> {
    let value = *counter;
    *counter = value.checked_add(1).ok_or(Status::Exhausted)?;
    Ok(value)
}
impl Registry {
    fn endpoint(
        &mut self,
        owner: u64,
        session: u64,
        instance: u64,
        class: PeerClass,
        rights: u32,
    ) -> Result<PeerContext, Status> {
        if self.endpoints.len() >= MAX_ENDPOINTS {
            return Err(Status::Exhausted);
        }
        let index = take_counter(&mut self.next_endpoint)?;
        let generation = take_counter(&mut self.next_generation)?;
        if index > u16::MAX as u64 || generation > u32::MAX as u64 {
            return Err(Status::Exhausted);
        }
        let peer = PeerContext {
            registry: self.identity,
            owner,
            session,
            instance,
            endpoint: Handle {
                kind: HandleKind::Ipc,
                index: index as u32,
                generation,
            },
            class,
        };
        self.endpoints.insert(
            peer.endpoint.pack(),
            Endpoint {
                peer: peer.clone(),
                rights,
                active: true,
            },
        );
        Ok(peer)
    }
    fn authorize(
        &self,
        peer: &PeerContext,
        handle: Handle,
        session: u64,
        generation: u64,
        class: PeerClass,
        right: u32,
    ) -> Result<(), Status> {
        let endpoint = self.endpoints.get(&handle.pack()).ok_or(Status::Denied)?;
        if peer.registry != self.identity
            || handle != peer.endpoint
            || !endpoint.active
            || endpoint.peer.registry != peer.registry
            || endpoint.peer.owner != peer.owner
            || endpoint.peer.session != peer.session
            || endpoint.peer.instance != peer.instance
            || peer.class != class
            || endpoint.peer.class != class
            || peer.session != session
            || endpoint.rights & right != right
        {
            return Err(Status::Denied);
        }
        let actual = self.sessions.get(&session).ok_or(Status::Denied)?;
        if actual.owner != peer.owner {
            return Err(Status::Denied);
        }
        if actual.generation != generation {
            return Err(Status::Stale);
        }
        Ok(())
    }
    fn retire(&mut self, id: u64, state: u32) {
        if let Some(instance) = self.instances.get_mut(&id) {
            if instance.state == RUNNING {
                instance.state = state;
            }
            instance.stop.store(true, Ordering::Release);
            if let Some(endpoint) = self.endpoints.get_mut(&instance.peer.endpoint.pack()) {
                endpoint.active = false;
            }
        }
    }
}

struct PrivateExecutable {
    directory: PathBuf,
    path: PathBuf,
    hash: [u8; 32],
}
// Child::drop alone does not kill/reap. This guard also cleans up when a
// fallible supervisor-thread creation drops its unstarted closure.
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl PrivateExecutable {
    fn create(config: &DevConfig, id: u64) -> Result<Self, Status> {
        let file = File::open(&config.witness_path).map_err(|_| Status::Invalid)?;
        let metadata = file.metadata().map_err(|_| Status::Invalid)?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_EXECUTABLE {
            return Err(Status::Invalid);
        }
        let mut bytes = Vec::new();
        file.take(MAX_EXECUTABLE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Status::Invalid)?;
        if bytes.len() as u64 > MAX_EXECUTABLE {
            return Err(Status::Invalid);
        }
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        if hash != config.pinned_sha256 {
            return Err(Status::Denied);
        }
        let directory =
            std::env::temp_dir().join(format!("ramen-desktop-private-{}-{id}", std::process::id()));
        DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|_| Status::Internal)?;
        let snapshot = Self {
            path: directory.join("witness"),
            directory,
            hash,
        };
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o500)
            .open(&snapshot.path)
            .map_err(|_| Status::Internal)?;
        output
            .write_all(&bytes)
            .and_then(|_| output.sync_all())
            .map_err(|_| Status::Internal)?;
        std::fs::set_permissions(&snapshot.path, std::fs::Permissions::from_mode(0o500))
            .map_err(|_| Status::Internal)?;
        Ok(snapshot)
    }
}
impl Drop for PrivateExecutable {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

pub struct DevDesktop {
    shared: Shared,
    config: DevConfig,
    executable: PrivateExecutable,
    supervisors: Vec<JoinHandle<()>>,
}
impl DevDesktop {
    pub fn new(config: DevConfig) -> Result<Self, Status> {
        if !(1..=30_000).contains(&config.preview_ttl_ms)
            || !(1..=600_000).contains(&config.instance_ttl_ms)
            || !(1..=5_000).contains(&config.child_deadline_ms)
        {
            return Err(Status::Invalid);
        }
        let id = NEXT_FIXTURE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| Status::Exhausted)?;
        let executable = PrivateExecutable::create(&config, id)?;
        Ok(Self {
            config,
            executable,
            supervisors: Vec::new(),
            shared: Arc::new(Mutex::new(Registry {
                identity: id,
                now: 0,
                next_session: 1,
                next_plan: 1,
                next_instance: 1,
                next_generation: 1,
                next_endpoint: 1,
                sessions: HashMap::new(),
                endpoints: HashMap::new(),
                instances: HashMap::new(),
            })),
        })
    }
    pub fn register_session(
        &mut self,
        owner_id: u64,
        selected_fixture_hash: [u8; 32],
        selected_revision: u64,
    ) -> Result<SessionFixture, Status> {
        let mut registry = self.shared.lock().expect("fixture registry");
        if registry.sessions.len() >= 2 {
            return Err(Status::Exhausted);
        }
        let id = take_counter(&mut registry.next_session)?;
        let generation = take_counter(&mut registry.next_generation)?;
        let peer = registry.endpoint(owner_id, id, 0, PeerClass::Chrome, CHROME_RIGHTS)?;
        registry.sessions.insert(
            id,
            Session {
                owner: owner_id,
                generation,
                selected_hash: selected_fixture_hash,
                selected_revision,
                policy_revision: 1,
                preview: None,
                current: None,
            },
        );
        Ok(SessionFixture {
            trusted_handle: peer.endpoint,
            trusted_peer: peer,
            session_id: id,
            session_generation: generation,
        })
    }
    pub fn issue_trusted_fixture_peer(
        &mut self,
        owner_id: u64,
        session_id: u64,
        rights: u32,
    ) -> Result<(PeerContext, Handle), Status> {
        let mut registry = self.shared.lock().expect("fixture registry");
        if rights == 0
            || rights & !CHROME_RIGHTS != 0
            || registry
                .sessions
                .get(&session_id)
                .is_none_or(|s| s.owner != owner_id)
        {
            return Err(Status::Denied);
        }
        let peer = registry.endpoint(owner_id, session_id, 0, PeerClass::Chrome, rights)?;
        Ok((peer.clone(), peer.endpoint))
    }
    pub fn update_policy_revision(&mut self, session_id: u64, revision: u64) -> Result<(), Status> {
        let mut registry = self.shared.lock().expect("fixture registry");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .ok_or(Status::Denied)?;
        if revision <= session.policy_revision {
            return Err(Status::Stale);
        }
        session.policy_revision = revision;
        session.preview = None;
        Ok(())
    }
    pub fn update_selected_identity(
        &mut self,
        session_id: u64,
        hash: [u8; 32],
        revision: u64,
    ) -> Result<(), Status> {
        let mut registry = self.shared.lock().expect("fixture registry");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .ok_or(Status::Denied)?;
        if revision <= session.selected_revision {
            return Err(Status::Stale);
        }
        session.selected_hash = hash;
        session.selected_revision = revision;
        session.preview = None;
        Ok(())
    }
    pub fn inject_chrome_confirmation(
        &mut self,
        peer: &PeerContext,
        session_id: u64,
        session_generation: u64,
        plan_id: u64,
        preview_revision: u64,
        now_ms: u64,
    ) -> Result<(), Status> {
        let mut registry = self.shared.lock().expect("fixture registry");
        registry.authorize(
            peer,
            peer.endpoint,
            session_id,
            session_generation,
            PeerClass::Chrome,
            2,
        )?;
        registry.now = registry.now.max(now_ms);
        let now_ms = registry.now;
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("authorized session");
        let plan = session.preview.as_mut().ok_or(Status::Stale)?;
        if plan.snapshot.plan_id != plan_id
            || plan.snapshot.preview_revision != preview_revision
            || now_ms >= plan.snapshot.expires_at_ms
        {
            session.preview = None;
            return Err(Status::Stale);
        }
        if plan.event {
            return Err(Status::Stale);
        }
        plan.event = true;
        Ok(())
    }
    pub fn preview_snapshot(
        &self,
        peer: &PeerContext,
        preview_shm: u64,
    ) -> Result<PreviewSnapshot, Status> {
        let registry = self.shared.lock().expect("fixture registry");
        if registry.sessions.iter().any(|(id, session)| {
            *id != peer.session
                && session
                    .preview
                    .as_ref()
                    .is_some_and(|plan| plan.handle == preview_shm)
        }) {
            return Err(Status::Denied);
        }
        let session = registry.sessions.get(&peer.session).ok_or(Status::Denied)?;
        registry.authorize(
            peer,
            peer.endpoint,
            peer.session,
            session.generation,
            PeerClass::Chrome,
            1,
        )?;
        let plan = session.preview.as_ref().ok_or(Status::Stale)?;
        if plan.handle != preview_shm {
            return Err(Status::Denied);
        }
        if registry.now >= plan.snapshot.expires_at_ms {
            return Err(Status::Stale);
        }
        decode_preview(&plan.object)
    }
    pub fn child_evidence(&self, instance_id: u64) -> Option<ChildEvidence> {
        self.shared
            .lock()
            .expect("fixture registry")
            .instances
            .get(&instance_id)
            .map(|i| i.evidence.clone())
    }
    pub fn is_endpoint_active(&self, handle: Handle) -> bool {
        self.shared
            .lock()
            .expect("fixture registry")
            .endpoints
            .get(&handle.pack())
            .is_some_and(|e| e.active && e.peer.endpoint == handle)
    }
    pub fn dispatch_recorded_child(
        &mut self,
        instance_id: u64,
        env: &Envelope,
        now_ms: u64,
    ) -> Envelope {
        let peer = self
            .shared
            .lock()
            .expect("fixture registry")
            .instances
            .get(&instance_id)
            .map(|i| i.peer.clone());
        peer.map_or_else(
            || error_reply(env, Status::Denied),
            |p| dispatch_child(&self.shared, &p, env, now_ms),
        )
    }
    pub fn maintenance(&mut self, now_ms: u64) {
        let expired = {
            let mut registry = self.shared.lock().expect("fixture registry");
            registry.now = registry.now.max(now_ms);
            let now_ms = registry.now;
            for session in registry.sessions.values_mut() {
                if session
                    .preview
                    .as_ref()
                    .is_some_and(|p| now_ms >= p.snapshot.expires_at_ms)
                {
                    session.preview = None;
                }
            }
            let ids: Vec<_> = registry
                .instances
                .iter()
                .filter(|(_, i)| i.state == RUNNING && now_ms >= i.expires)
                .map(|(id, _)| *id)
                .collect();
            for id in &ids {
                registry.retire(*id, EXPIRED);
            }
            ids
        };
        for id in expired {
            self.wait_reaped(id);
        }
    }
    fn wait_reaped(&self, id: u64) {
        // Independent supervisor owns Child; no registry lock is held waiting.
        while self
            .shared
            .lock()
            .expect("fixture registry")
            .instances
            .get(&id)
            .is_some_and(|i| !i.evidence.reaped)
        {
            thread::sleep(Duration::from_millis(1));
        }
    }
    pub fn dispatch(&mut self, peer: &PeerContext, env: &Envelope, now_ms: u64) -> Envelope {
        if peer.class == PeerClass::Child {
            return dispatch_child(&self.shared, peer, env, now_ms);
        }
        if let Err(status) = validate_request(env) {
            return error_reply(env, status);
        }
        if !matches!(env.msg_type, 1 | 3 | 5 | 7 | 9 | 11 | 13) {
            return error_reply(env, Status::Denied);
        }
        let session_id = field_u64(env, 8);
        let generation = field_u64(env, 16);
        let right = match env.msg_type {
            1 => 1,
            3 => 4,
            5 => 2,
            7 => 8,
            9 => 16,
            11 => 32,
            13 => 64,
            _ => unreachable!(),
        };
        {
            let mut registry = self.shared.lock().expect("fixture registry");
            if let Err(status) = registry.authorize(
                peer,
                env.handle,
                session_id,
                generation,
                PeerClass::Chrome,
                right,
            ) {
                return error_reply(env, status);
            }
            registry.now = registry.now.max(now_ms);
        }
        match env.msg_type {
            1 => {
                let request = idl::PrepareLaunch::from_envelope(env).expect("validated request");
                if request.requested_rights != OBSERVE
                    || request.application_hash != self.executable.hash
                {
                    return error_reply(env, Status::Denied);
                }
                self.prepare_plan(env, session_id, now_ms)
            }
            3 => {
                let request = idl::CancelPreview::from_envelope(env).expect("validated request");
                let mut registry = self.shared.lock().expect("fixture registry");
                let session = registry
                    .sessions
                    .get_mut(&session_id)
                    .expect("authorized session");
                if session
                    .preview
                    .as_ref()
                    .is_none_or(|p| p.snapshot.plan_id != request.plan_id)
                {
                    return error_reply(env, Status::Stale);
                }
                session.preview = None;
                error_reply(env, Status::Ok)
            }
            5 => self.confirm_plan(env, peer, session_id, now_ms),
            7 | 9 | 11 | 13 => {
                let id = field_u64(env, 24);
                let instance_generation = field_u64(env, 32);
                {
                    let registry = self.shared.lock().expect("fixture registry");
                    let Some(instance) = registry.instances.get(&id) else {
                        return error_reply(env, Status::Denied);
                    };
                    if instance.session != session_id {
                        return error_reply(env, Status::Denied);
                    }
                    if instance.generation != instance_generation {
                        return error_reply(env, Status::Stale);
                    }
                }
                match env.msg_type {
                    7 => {
                        let registry = self.shared.lock().expect("fixture registry");
                        status_response(env, registry.instances.get(&id).expect("checked instance"))
                    }
                    9 | 11 => {
                        {
                            self.shared
                                .lock()
                                .expect("fixture registry")
                                .retire(id, if env.msg_type == 9 { EXITED } else { REVOKED });
                        }
                        self.wait_reaped(id);
                        error_reply(env, Status::Ok)
                    }
                    13 => self.prepare_plan(env, session_id, now_ms),
                    _ => unreachable!(),
                }
            }
            _ => error_reply(env, Status::Unsupported),
        }
    }
    fn prepare_plan(&self, env: &Envelope, session_id: u64, now: u64) -> Envelope {
        let mut registry = self.shared.lock().expect("fixture registry");
        registry.now = registry.now.max(now);
        let Some(expires) = registry.now.checked_add(self.config.preview_ttl_ms) else {
            return error_reply(env, Status::Invalid);
        };
        let session = registry
            .sessions
            .get(&session_id)
            .expect("authorized session");
        if registry.instances.len() >= MAX_INSTANCES
            || session.preview.is_some()
            || session.current.is_some_and(|id| {
                registry
                    .instances
                    .get(&id)
                    .is_some_and(|i| i.state == RUNNING)
            })
        {
            return error_reply(env, Status::Exhausted);
        }
        let plan_id = match take_counter(&mut registry.next_plan) {
            Ok(id) => id,
            Err(status) => return error_reply(env, status),
        };
        if plan_id > u16::MAX as u64 {
            return error_reply(env, Status::Exhausted);
        }
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("authorized session");
        let snapshot = PreviewSnapshot {
            session_id,
            session_generation: session.generation,
            plan_id,
            preview_revision: plan_id,
            selected_revision: session.selected_revision,
            policy_revision: session.policy_revision,
            expires_at_ms: expires,
            application_hash: self.executable.hash,
            selected_fixture_hash: session.selected_hash,
            schema_version: 1,
            granted_rights: OBSERVE,
        };
        let handle = Handle {
            kind: HandleKind::Shmem,
            index: plan_id as u32,
            generation: 1,
        }
        .pack();
        let object = encode_preview(&snapshot);
        session.preview = Some(Plan {
            snapshot: snapshot.clone(),
            object,
            handle,
            event: false,
        });
        preview_response(env, &snapshot, handle)
    }
    fn confirm_plan(
        &mut self,
        env: &Envelope,
        peer: &PeerContext,
        session_id: u64,
        now: u64,
    ) -> Envelope {
        let request = idl::ConfirmLaunch::from_envelope(env).expect("validated request");
        let (id, generation, child_peer, stop, bootstrap, expires) = {
            let mut registry = self.shared.lock().expect("fixture registry");
            registry.now = registry.now.max(now);
            let now = registry.now;
            let Some(expires) = now.checked_add(self.config.instance_ttl_ms) else {
                return error_reply(env, Status::Invalid);
            };
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("authorized session");
            let Some(plan) = &session.preview else {
                return error_reply(env, Status::Stale);
            };
            if plan.snapshot.plan_id != request.plan_id
                || plan.snapshot.preview_revision != request.preview_revision
                || plan.snapshot.application_hash != self.executable.hash
                || plan.snapshot.policy_revision != session.policy_revision
                || plan.snapshot.selected_fixture_hash != session.selected_hash
                || plan.snapshot.selected_revision != session.selected_revision
                || now >= plan.snapshot.expires_at_ms
            {
                session.preview = None;
                return error_reply(env, Status::Stale);
            }
            if !plan.event {
                return error_reply(env, Status::Denied);
            }
            session.preview = None;
            if registry.instances.len() >= MAX_INSTANCES {
                return error_reply(env, Status::Exhausted);
            }
            let id = match take_counter(&mut registry.next_instance) {
                Ok(id) => id,
                Err(status) => return error_reply(env, status),
            };
            let generation = match take_counter(&mut registry.next_generation) {
                Ok(id) => id,
                Err(status) => return error_reply(env, status),
            };
            let child_peer =
                match registry.endpoint(peer.owner, session_id, id, PeerClass::Child, OBSERVE) {
                    Ok(p) => p,
                    Err(status) => return error_reply(env, status),
                };
            let bootstrap = idl::InstanceBootstrap {
                session_id,
                session_generation: request.session_generation,
                instance_id: id,
                instance_generation: generation,
                observation_handle: child_peer.endpoint.pack(),
                granted_rights: OBSERVE,
                reserved: 0,
            };
            (
                id,
                generation,
                child_peer,
                Arc::new(AtomicBool::new(false)),
                bootstrap,
                expires,
            )
        };
        // The path is a private, read-only snapshot of precisely the hashed bytes.
        // No caller path/environment is consulted when selecting the executable.
        let mode = match self.config.witness_mode {
            WitnessMode::Observe => "observe",
            WitnessMode::Fault => "fault",
            WitnessMode::Stall => "stall",
            WitnessMode::Flood => "flood",
        };
        // Capture the absolute deadline before spawn; supervisor scheduling or
        // bootstrap/pipe work cannot restart the child's execution budget.
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(self.config.child_deadline_ms))
            .expect("validated five-second bound");
        let mut child = match Command::new(&self.executable.path)
            .arg(mode)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                if let Some(e) = self
                    .shared
                    .lock()
                    .expect("fixture registry")
                    .endpoints
                    .get_mut(&child_peer.endpoint.pack())
                {
                    e.active = false;
                }
                return error_reply(env, Status::NotReady);
            }
        };
        let pid = child.id();
        let input = child.stdin.take().expect("owned private pipe");
        let output = child.stdout.take().expect("owned private pipe");
        {
            let mut registry = self.shared.lock().expect("fixture registry");
            registry.instances.insert(
                id,
                Instance {
                    session: session_id,
                    generation,
                    state: RUNNING,
                    expires,
                    peer: child_peer.clone(),
                    stop: stop.clone(),
                    evidence: ChildEvidence {
                        actual_pid: pid,
                        executable_sha256: self.executable.hash,
                        bootstrap,
                        exchanges: Vec::new(),
                        observation_count: 0,
                        reaped: false,
                        exchange_limit_hit: false,
                    },
                },
            );
            registry
                .sessions
                .get_mut(&session_id)
                .expect("authorized session")
                .current = Some(id);
        }
        let shared = self.shared.clone();
        let owned_child = OwnedChild(child);
        let supervisor = thread::Builder::new()
            .name(format!("desktop-watchdog-{id}"))
            .spawn(move || {
                supervise(
                    shared,
                    owned_child,
                    input,
                    output,
                    child_peer,
                    bootstrap,
                    stop,
                    deadline,
                )
            });
        match supervisor {
            Ok(supervisor) => self.supervisors.push(supervisor),
            Err(_) => {
                let mut registry = self.shared.lock().expect("fixture registry");
                registry.retire(id, FAULTED);
                registry
                    .instances
                    .get_mut(&id)
                    .expect("owned child record")
                    .evidence
                    .reaped = true;
                return error_reply(env, Status::Exhausted);
            }
        }
        encode_message(
            Handle::INVALID,
            &idl::ConfirmLaunchReply {
                request_id: request.request_id,
                instance_id: id,
                instance_generation: generation,
                observation_handle: bootstrap.observation_handle,
                granted_rights: OBSERVE,
                status: Status::Ok as u32,
                child_pid: pid,
                reserved: 0,
            },
        )
    }
}
impl Drop for DevDesktop {
    fn drop(&mut self) {
        {
            let mut registry = self.shared.lock().expect("fixture registry");
            let ids: Vec<_> = registry.instances.keys().copied().collect();
            for id in ids {
                registry.retire(id, EXITED);
            }
        }
        for supervisor in self.supervisors.drain(..) {
            let _ = supervisor.join();
        }
    }
}

fn field_u64(env: &Envelope, offset: usize) -> u64 {
    u64::from_le_bytes(
        env.payload[offset..offset + 8]
            .try_into()
            .expect("validated typed field"),
    )
}
fn error_reply(env: &Envelope, status: Status) -> Envelope {
    let request_id = field_u64(env, 0);
    if status == Status::Unsupported {
        return encode_message(
            Handle::INVALID,
            &idl::CancelPreviewReply {
                request_id,
                status: status as u32,
                reserved: 0,
            },
        );
    }
    match env.msg_type {
        1 => encode_message(
            Handle::INVALID,
            &idl::PrepareLaunchReply {
                request_id,
                plan_id: 0,
                preview_revision: 0,
                preview_shm: 0,
                preview_len: 0,
                status: status as u32,
                granted_rights: 0,
                reserved: 0,
            },
        ),
        5 => encode_message(
            Handle::INVALID,
            &idl::ConfirmLaunchReply {
                request_id,
                instance_id: 0,
                instance_generation: 0,
                observation_handle: 0,
                granted_rights: 0,
                status: status as u32,
                child_pid: 0,
                reserved: 0,
            },
        ),
        7 => encode_message(
            Handle::INVALID,
            &idl::GetStatusReply {
                request_id,
                instance_id: 0,
                instance_generation: 0,
                executable_hash: [0; 32],
                state: 0,
                status: status as u32,
            },
        ),
        9 => encode_message(
            Handle::INVALID,
            &idl::CloseInstanceReply {
                request_id,
                status: status as u32,
                reserved: 0,
            },
        ),
        11 => encode_message(
            Handle::INVALID,
            &idl::RevokeInstanceReply {
                request_id,
                status: status as u32,
                reserved: 0,
            },
        ),
        13 => encode_message(
            Handle::INVALID,
            &idl::PrepareRestartReply {
                request_id,
                plan_id: 0,
                preview_revision: 0,
                preview_shm: 0,
                preview_len: 0,
                status: status as u32,
                granted_rights: 0,
                reserved: 0,
            },
        ),
        17 => encode_message(
            Handle::INVALID,
            &idl::ObserveInstanceReply {
                request_id,
                instance_id: 0,
                instance_generation: 0,
                executable_hash: [0; 32],
                state: 0,
                status: status as u32,
            },
        ),
        _ => encode_message(
            Handle::INVALID,
            &idl::CancelPreviewReply {
                request_id,
                status: status as u32,
                reserved: 0,
            },
        ),
    }
}
fn preview_response(env: &Envelope, preview: &PreviewSnapshot, handle: u64) -> Envelope {
    let request_id = field_u64(env, 0);
    if env.msg_type == 13 {
        encode_message(
            Handle::INVALID,
            &idl::PrepareRestartReply {
                request_id,
                plan_id: preview.plan_id,
                preview_revision: preview.preview_revision,
                preview_shm: handle,
                preview_len: 136,
                status: 0,
                granted_rights: OBSERVE,
                reserved: 0,
            },
        )
    } else {
        encode_message(
            Handle::INVALID,
            &idl::PrepareLaunchReply {
                request_id,
                plan_id: preview.plan_id,
                preview_revision: preview.preview_revision,
                preview_shm: handle,
                preview_len: 136,
                status: 0,
                granted_rights: OBSERVE,
                reserved: 0,
            },
        )
    }
}
fn status_response(env: &Envelope, instance: &Instance) -> Envelope {
    if env.msg_type == 17 {
        encode_message(
            Handle::INVALID,
            &idl::ObserveInstanceReply {
                request_id: field_u64(env, 0),
                instance_id: instance.evidence.bootstrap.instance_id,
                instance_generation: instance.generation,
                executable_hash: instance.evidence.executable_sha256,
                state: instance.state,
                status: 0,
            },
        )
    } else {
        encode_message(
            Handle::INVALID,
            &idl::GetStatusReply {
                request_id: field_u64(env, 0),
                instance_id: instance.evidence.bootstrap.instance_id,
                instance_generation: instance.generation,
                executable_hash: instance.evidence.executable_sha256,
                state: instance.state,
                status: 0,
            },
        )
    }
}
fn dispatch_child(shared: &Shared, peer: &PeerContext, env: &Envelope, now: u64) -> Envelope {
    if let Err(status) = validate_request(env) {
        return error_reply(env, status);
    }
    if env.msg_type != 17 {
        return error_reply(env, Status::Denied);
    }
    let request = idl::ObserveInstance::from_envelope(env).expect("validated child message");
    let mut registry = shared.lock().expect("fixture registry");
    if let Err(status) = registry.authorize(
        peer,
        env.handle,
        request.session_id,
        request.session_generation,
        PeerClass::Child,
        OBSERVE,
    ) {
        return error_reply(env, status);
    }
    if peer.instance != request.instance_id {
        return error_reply(env, Status::Denied);
    }
    // Pipe readers and retained-context probes may carry a previously captured
    // clock. Clamp it while holding the same lock used to enforce the lease.
    registry.now = registry.now.max(now);
    let now = registry.now;
    let Some(instance) = registry.instances.get(&request.instance_id) else {
        return error_reply(env, Status::Denied);
    };
    if instance.generation != request.instance_generation {
        return error_reply(env, Status::Stale);
    }
    if instance.state != RUNNING {
        return error_reply(env, Status::Denied);
    }
    if now >= instance.expires {
        registry.retire(request.instance_id, EXPIRED);
        return error_reply(env, Status::Denied);
    }
    status_response(env, instance)
}

#[allow(clippy::too_many_arguments)]
fn supervise(
    shared: Shared,
    mut child: OwnedChild,
    mut input: std::process::ChildStdin,
    mut output: std::process::ChildStdout,
    peer: PeerContext,
    bootstrap: idl::InstanceBootstrap,
    stop: Arc<AtomicBool>,
    deadline: Instant,
) {
    let id = bootstrap.instance_id;
    let reader_shared = shared.clone();
    let reader_stop = stop.clone();
    // Pipe reads/writes may block. This thread never owns Child or holds the
    // registry lock during IO. The independent supervisor can always kill/reap.
    let reader = thread::Builder::new()
        .name(format!("desktop-pipe-{id}"))
        .spawn(move || {
            if write_frame(&mut input, &encode_message(Handle::INVALID, &bootstrap)).is_err() {
                reader_stop.store(true, Ordering::Release);
                return;
            }
            loop {
                let request = match read_frame(&mut output) {
                    Ok(request) => request,
                    Err(_) => {
                        reader_stop.store(true, Ordering::Release);
                        break;
                    }
                };
                let now = {
                    let mut registry = reader_shared.lock().expect("fixture registry");
                    let instance = registry.instances.get_mut(&id).expect("owned child");
                    if instance.evidence.exchanges.len() >= MAX_EXCHANGES {
                        instance.evidence.exchange_limit_hit = true;
                        registry.retire(id, FAULTED);
                        break;
                    }
                    registry.now
                };
                let reply = dispatch_child(&reader_shared, &peer, &request, now);
                {
                    let mut registry = reader_shared.lock().expect("fixture registry");
                    let instance = registry.instances.get_mut(&id).expect("owned child");
                    if instance.evidence.reaped {
                        break;
                    }
                    if request.msg_type == 17 && field_status(&reply) == 0 {
                        instance.evidence.observation_count += 1;
                    }
                    instance.evidence.exchanges.push((request, reply));
                }
                if write_frame(&mut input, &reply).is_err() {
                    reader_stop.store(true, Ordering::Release);
                    break;
                }
            }
        });
    if reader.is_err() {
        stop.store(true, Ordering::Release);
    }
    let success = loop {
        match child.0.try_wait() {
            Ok(Some(status)) => break status.success(),
            Err(_) => {
                let _ = child.0.kill();
                let _ = child.0.wait();
                break false;
            }
            Ok(None) => {}
        }
        if stop.load(Ordering::Acquire) || Instant::now() >= deadline {
            let _ = child.0.kill();
            let _ = child.0.wait();
            break false;
        }
        thread::sleep(Duration::from_millis(2));
    };
    {
        let mut registry = shared.lock().expect("fixture registry");
        registry.retire(id, if success { EXITED } else { FAULTED });
        registry
            .instances
            .get_mut(&id)
            .expect("owned child")
            .evidence
            .reaped = true;
    }
    if let Ok(reader) = reader {
        let _ = reader.join();
    }
}
fn field_status(env: &Envelope) -> u32 {
    let offset = if matches!(env.msg_type, 8 | 18) {
        60
    } else if matches!(env.msg_type, 2 | 6 | 14) {
        36
    } else {
        8
    };
    u32::from_le_bytes(
        env.payload[offset..offset + 4]
            .try_into()
            .expect("generated status field"),
    )
}

fn encode_preview(snapshot: &PreviewSnapshot) -> [u8; 136] {
    let mut out = [0; 136];
    let fields = [
        snapshot.session_id,
        snapshot.session_generation,
        snapshot.plan_id,
        snapshot.preview_revision,
        snapshot.selected_revision,
        snapshot.policy_revision,
        snapshot.expires_at_ms,
    ];
    for (index, value) in fields.iter().enumerate() {
        out[index * 8..index * 8 + 8].copy_from_slice(&value.to_le_bytes());
    }
    out[56..88].copy_from_slice(&snapshot.application_hash);
    out[88..120].copy_from_slice(&snapshot.selected_fixture_hash);
    out[120..124].copy_from_slice(&snapshot.schema_version.to_le_bytes());
    out[124..128].copy_from_slice(&snapshot.granted_rights.to_le_bytes());
    out[128..132].copy_from_slice(&136u32.to_le_bytes());
    out
}
fn decode_preview(bytes: &[u8; 136]) -> Result<PreviewSnapshot, Status> {
    let mut reader = MessageReader { bytes, offset: 0 };
    let session_id = reader.u64();
    let session_generation = reader.u64();
    let plan_id = reader.u64();
    let preview_revision = reader.u64();
    let selected_revision = reader.u64();
    let policy_revision = reader.u64();
    let expires_at_ms = reader.u64();
    let application_hash = reader.bytes32();
    let selected_fixture_hash = reader.bytes32();
    let schema_version = reader.u32();
    let granted_rights = reader.u32();
    let length = reader.u32();
    let reserved = reader.u32();
    if schema_version != 1 || length != 136 || reserved != 0 || granted_rights != OBSERVE {
        return Err(Status::Invalid);
    }
    Ok(PreviewSnapshot {
        session_id,
        session_generation,
        plan_id,
        preview_revision,
        selected_revision,
        policy_revision,
        expires_at_ms,
        application_hash,
        selected_fixture_hash,
        schema_version,
        granted_rights,
    })
}
