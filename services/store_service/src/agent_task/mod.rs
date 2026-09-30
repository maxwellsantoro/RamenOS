//! Opt-in SW0 host service proof. Trusted launcher binds connected transports to
//! domains; no public listener or production broker registration is installed.
//! Service objects and host mappings enforce this named boundary, not the target kernel.
mod worker;
use artifact_store_core::{blob_path, hash_bytes, publish_cas_artifact, write_blob_bytes_atomic};
use artifact_store_schema::agent_task::*;
pub use artifact_store_schema::agent_task::{TaskPolicyV1, TaskSnapshotV1};
use artifact_store_schema::{ContentId, Manifest};
use kernel_api::agent_task_protocol::*;
use kernel_api::cap::{Handle, HandleKind};
use kernel_api::generated::agent_task_v1::*;
use kernel_api::ipc::Envelope;
use kernel_api::wire::write_payload;
use memmap2::MmapMut;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const SCHEMA_RESOURCE: u64 = 100;
pub const NOTES_RESOURCE: u64 = 101;

const MAX_OBJECTS: usize = 256;
const MAX_AUDIT: usize = 2048;
const MAX_JOURNAL: u64 = 4_000_000;
const MAX_STATE: usize = 4096;
#[derive(Clone)]
pub struct TaskFixture {
    pub contract: TaskContractV0,
    pub resource_id: u64,
    pub input: Vec<u8>,
    pub schema: Vec<u8>,
    pub policy: Vec<u8>,
    pub notes: Vec<u8>,
    pub validator: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    cap: u64,
    id: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CommitBinding {
    request_id: u64,
    candidate_cap: u64,
    expected_revision: u64,
    expected_hash: [u8; 32],
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedReceipt {
    binding: CommitBinding,
    cap: u64,
    receipt: CommitReceiptV0,
    validation: ValidationEvidenceV0,
    grant: TaskGrantV0,
    committed_at_ms: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Audit {
    seq: u64,
    domain: u64,
    op: u32,
    request: u64,
    status: u32,
    generation: u64,
    revision: u64,
    content_id: String,
    effect: bool,
    previous: String,
    digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    resource_id: u64,
    initial_id: String,
    notes_id: String,
    last_validation: Option<ValidationEvidenceV0>,
    state: TaskStateV0,
    candidates: Vec<Candidate>,
    receipts: Vec<SavedReceipt>,
    audit: Vec<Audit>,
}
#[derive(Clone)]
struct Grant {
    domain: u64,
    rights: u32,
    generation: u64,
    expires: u64,
}
struct Mapping {
    domain: u64,
    generation: u64,
    expires: u64,
    grant: Option<u64>,
    writable: bool,
    len: usize,
    map: MmapMut,
    _file: File,
}
struct Subscription {
    connection: u64,
    domain: u64,
    grant: u64,
    mask: u32,
    pending: HashMap<u32, u64>,
    pull: bool,
}

pub struct TaskService {
    root: PathBuf,
    cas: PathBuf,
    worker: PathBuf,
    fixture: TaskFixture,
    policy: TaskPolicyV1,
    journal: Journal,
    policy_cap: u64,
    grants: HashMap<u64, Grant>,
    mappings: HashMap<u64, Mapping>,
    subscriptions: HashMap<u64, Subscription>,
    next_mapping: u16,
    clock: Instant,
    poisoned: bool,
    active_workers: usize,
    _lock: File,
}
pub fn content_hash(bytes: &[u8]) -> [u8; 32] {
    hash_array(&hash_bytes(bytes))
}
fn hash_array(id: &str) -> [u8; 32] {
    let mut b = [0; 32];
    hex::decode_to_slice(&id[7..], &mut b).expect("validated content id");
    b
}
fn hash_id(hash: [u8; 32]) -> String {
    format!("sha256:{}", hex::encode(hash))
}
fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}
fn status(error: ContractError) -> u32 {
    match error {
        ContractError::Conflict => STATUS_CONFLICT,
        ContractError::RequestReuse => STATUS_REQUEST_REUSE,
        ContractError::Capacity | ContractError::RevisionExhausted => STATUS_CAPACITY,
        ContractError::Expired => STATUS_EXPIRED,
        ContractError::WrongAuthority
        | ContractError::WrongKind
        | ContractError::Revoked
        | ContractError::ValidationMismatch => STATUS_DENIED,
        ContractError::NotStaged
        | ContractError::InvalidCandidate
        | ContractError::ExecutionLimit => STATUS_VALIDATION_FAILED,
        _ => STATUS_INVALID,
    }
}
fn envelope<T: Copy>(ty: u32, value: T) -> Envelope {
    let mut e = Envelope::empty(14, ty);
    write_payload(&mut e, &value).expect("IDL size gate");
    e
}
fn error_reply(request: &Envelope, code: u32) -> io::Result<Envelope> {
    let (len, offset) = match request.msg_type {
        1 => (56, 48),
        3 => (40, 32),
        5 => (56, 48),
        7 => (40, 24),
        9 | 11 => (64, 56),
        13 => (32, 24),
        15 => (24, 16),
        17 | 21 => (32, 24),
        23 => (40, 32),
        25 => (16, 8),
        _ => return Err(invalid("unknown request operation")),
    };
    let mut e = Envelope::empty(14, request.msg_type + 1);
    e.payload_len = len;
    if request.payload_len >= 16 && request.payload_len <= 64 {
        e.payload[..8].copy_from_slice(&request.payload[8..16]);
    }
    e.payload[offset..offset + 4].copy_from_slice(&code.to_le_bytes());
    Ok(e)
}
impl TaskService {
    pub fn open(root: &Path, fixture: TaskFixture, worker: PathBuf) -> io::Result<Self> {
        fixture
            .contract
            .validate()
            .map_err(|_| invalid("task contract"))?;
        if fixture.resource_id == 0
            || fixture.input.is_empty()
            || fixture.input.len() > 65_536
            || fixture.schema.len() > 65_536
            || fixture.policy.len() > 65_536
            || fixture.validator.len() > 1_048_576
            || hash_bytes(&fixture.schema) != fixture.contract.schema_id
            || hash_bytes(&fixture.policy) != fixture.contract.policy_id
            || hash_bytes(&fixture.validator) != fixture.contract.validator_id
        {
            return Err(invalid("fixture bounds or pin mismatch"));
        }
        let policy: TaskPolicyV1 = serde_json::from_slice(&fixture.policy)?;
        if policy.schema_version != 1
            || policy.task_id != fixture.contract.task_id
            || policy.domain_id != fixture.contract.domain_id
            || policy.resource_id != fixture.resource_id
            || policy.allowed_rights == 0
            || policy.allowed_rights & !RIGHT_ALL != 0
            || policy.max_grant_ms == 0
            || policy.max_grant_ms > MAX_GRANT_LIFETIME_MS
            || fixture.resource_id == SCHEMA_RESOURCE
            || fixture.resource_id == NOTES_RESOURCE
            || fixture.notes.is_empty()
            || fixture.notes.len() > 65536
        {
            return Err(invalid("task policy"));
        }
        fs::create_dir_all(root)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("writer.lock"))?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let cas = root.join("cas");
        fs::create_dir_all(&cas)?;
        let journal_path = root.join("task.json");
        let journal = if journal_path.try_exists()? {
            let mut b = Vec::new();
            File::open(&journal_path)?
                .take(MAX_JOURNAL + 1)
                .read_to_end(&mut b)?;
            if b.len() as u64 > MAX_JOURNAL {
                return Err(invalid("oversized journal"));
            }
            let j: Journal = serde_json::from_slice(&b)?;
            j.state
                .validate_journal()
                .map_err(|_| invalid("corrupt transaction journal"))?;
            if j.version != 1
                || j.resource_id != fixture.resource_id
                || j.initial_id != hash_bytes(&fixture.input)
                || j.notes_id != hash_bytes(&fixture.notes)
                || j.state.contract() != &fixture.contract
                || j.candidates.len() > 64
                || j.receipts.len() > 64
                || j.audit.len() > MAX_AUDIT
            {
                return Err(invalid("journal identity/bounds"));
            }
            j
        } else {
            Journal {
                version: 1,
                resource_id: fixture.resource_id,
                initial_id: hash_bytes(&fixture.input),
                notes_id: hash_bytes(&fixture.notes),
                last_validation: None,
                state: TaskStateV0::new(fixture.contract.clone(), &hash_bytes(&fixture.input))
                    .map_err(|_| invalid("initial task state"))?,
                candidates: Vec::new(),
                receipts: Vec::new(),
                audit: Vec::new(),
            }
        };
        let mut service = Self {
            root: root.into(),
            cas,
            worker,
            fixture,
            policy,
            journal,
            policy_cap: rand::random(),
            grants: HashMap::new(),
            mappings: HashMap::new(),
            subscriptions: HashMap::new(),
            next_mapping: 0,
            clock: Instant::now(),
            poisoned: false,
            active_workers: 0,
            _lock: lock,
        };
        if service.policy_cap == 0 {
            service.policy_cap = 1;
        }
        service.publish(&service.fixture.input)?;
        service.publish(&service.fixture.schema)?;
        service.publish(&service.fixture.policy)?;
        service.publish(&service.fixture.validator)?;
        service.publish(&service.fixture.notes)?;
        service.verify_audit()?;
        for c in &service.journal.candidates {
            if c.cap == 0
                || service
                    .journal
                    .candidates
                    .iter()
                    .filter(|other| other.cap == c.cap)
                    .count()
                    != 1
            {
                return Err(invalid("candidate registry"));
            }
            service.blob(&c.id)?;
        }
        for r in &service.journal.receipts {
            let candidate = service
                .journal
                .candidates
                .iter()
                .find(|c| c.cap == r.binding.candidate_cap)
                .ok_or_else(|| invalid("receipt candidate"))?;
            if candidate.id != r.receipt.accepted.content_id
                || r.cap == 0
                || r.binding.request_id != r.receipt.request_id
                || r.binding.expected_revision != r.receipt.prior.revision
                || hash_id(r.binding.expected_hash) != r.receipt.prior.content_id
            {
                return Err(invalid("receipt binding"));
            }
        }
        // Restart never resurrects old grants or validations. Durable candidates
        // and successful receipts survive so an authorized retry remains exact.
        service
            .journal
            .state
            .revoke()
            .map_err(|_| invalid("generation exhausted"))?;
        service.persist()?;
        Ok(service)
    }
    fn now(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }
    pub fn policy_cap(&self) -> u64 {
        self.policy_cap
    }
    fn object_cap(&self) -> u64 {
        loop {
            let c = rand::random();
            if c != 0
                && c != self.policy_cap
                && !self.grants.contains_key(&c)
                && !self.subscriptions.contains_key(&c)
                && !self.journal.candidates.iter().any(|o| o.cap == c)
                && !self.journal.receipts.iter().any(|o| o.cap == c)
            {
                return c;
            }
        }
    }
    fn publish(&self, b: &[u8]) -> io::Result<String> {
        let id = hash_bytes(b);
        let parsed = ContentId::parse(&id).map_err(|_| invalid("content id"))?;
        let mut registry =
            crate::DomainArtifactRegistry::new(&self.cas).map_err(io::Error::other)?;
        registry
            .check_publication(&self.cas, &parsed, self.fixture.contract.domain_id, false)
            .map_err(io::Error::other)?;
        let manifest = Manifest {
            schema_version: 1,
            content_id: id.clone(),
            size_bytes: b.len() as u64,
            kind: "agent-task-private".into(),
            channels: vec![],
            signatures: vec![],
        };
        publish_cas_artifact(&self.cas, &manifest, |path| {
            write_blob_bytes_atomic(path, b)
        })?;
        if registry.get_owner(&parsed).is_none() {
            registry
                .register_artifact(&parsed, self.fixture.contract.domain_id, false)
                .map_err(io::Error::other)?;
        }
        Ok(id)
    }
    fn blob(&self, id: &str) -> io::Result<Vec<u8>> {
        ContentId::parse(id).map_err(|_| invalid("invalid stored id"))?;
        let path = blob_path(&self.cas, id);
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(invalid("nonregular CAS blob"));
        }
        let mut b = Vec::new();
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(blob_path(&self.cas, id))?
            .take(1_048_577)
            .read_to_end(&mut b)?;
        if b.len() > 1_048_576 {
            return Err(invalid("oversized CAS blob"));
        }
        if hash_bytes(&b) != id {
            return Err(invalid("CAS hash mismatch"));
        }
        Ok(b)
    }
    pub fn accepted_bytes(&self) -> io::Result<Vec<u8>> {
        if self.poisoned {
            return Err(invalid("service requires recovery"));
        }
        self.blob(&self.journal.state.accepted().content_id)
    }
    fn persist(&mut self) -> io::Result<()> {
        let result = (|| {
            let b = serde_json::to_vec(&self.journal)?;
            if b.len() as u64 > MAX_JOURNAL {
                return Err(invalid("journal capacity"));
            }
            let mut file = File::create(self.root.join("task.next"))?;
            file.write_all(&b)?;
            file.sync_all()?;
            fs::rename(self.root.join("task.next"), self.root.join("task.json"))?;
            File::open(&self.root)?.sync_all()
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn authorize(&self, domain: u64, cap: u64, right: u32) -> Result<Grant, u32> {
        if self.poisoned {
            return Err(STATUS_IO);
        }
        let g = self.grants.get(&cap).ok_or(STATUS_DENIED)?;
        if g.domain != domain
            || g.generation != self.journal.state.generation()
            || self.now() >= g.expires
            || g.rights & right != right
        {
            return Err(STATUS_DENIED);
        }
        Ok(g.clone())
    }
    fn grant_model(&self, g: &Grant) -> TaskGrantV0 {
        TaskGrantV0 {
            task_id: self.fixture.contract.task_id,
            domain_id: g.domain,
            resource: self.fixture.contract.resource.clone(),
            generation: g.generation,
            expires_at_ms: g.expires,
            kind: GrantKindV0::Mutation,
        }
    }
    fn candidate(&self, cap: u64) -> Result<String, u32> {
        self.journal
            .candidates
            .iter()
            .find(|c| c.cap == cap)
            .map(|c| c.id.clone())
            .ok_or(STATUS_DENIED)
    }
    fn clean(&mut self) {
        let now = self.now();
        let gen = self.journal.state.generation();
        self.grants
            .retain(|_, g| g.expires > now && g.generation == gen);
        self.mappings.retain(|_, m| {
            m.expires > now
                && m.generation == gen
                && m.grant.is_none_or(|g| self.grants.contains_key(&g))
        });
        self.subscriptions
            .retain(|_, s| self.grants.contains_key(&s.grant));
    }
    fn mapping(
        &mut self,
        domain: u64,
        bytes: &[u8],
        writable: bool,
        grant: Option<u64>,
    ) -> Result<u64, u32> {
        self.clean();
        if self.mappings.len() >= MAX_OBJECTS || bytes.is_empty() || bytes.len() > 65_536 {
            return Err(STATUS_CAPACITY);
        }
        self.next_mapping = self.next_mapping.checked_add(1).ok_or(STATUS_CAPACITY)?;
        let file = tempfile::tempfile().map_err(|_| STATUS_IO)?;
        file.set_len(bytes.len() as u64).map_err(|_| STATUS_IO)?;
        // Backing file lives for the entire map lifetime; no caller paths or fd
        // reuse enter this registry. Host launcher mediates access to the map.
        let mut map = unsafe { MmapMut::map_mut(&file) }.map_err(|_| STATUS_IO)?;
        map.copy_from_slice(bytes);
        if self.journal.state.generation() > u32::MAX as u64 {
            return Err(STATUS_CAPACITY);
        }
        let cap = Handle {
            kind: HandleKind::Shmem,
            index: self.next_mapping as u32,
            generation: self.journal.state.generation(),
        }
        .pack();
        let expires = grant
            .and_then(|c| self.grants.get(&c))
            .map_or(self.now() + 300_000, |g| g.expires);
        self.mappings.insert(
            cap,
            Mapping {
                domain,
                generation: self.journal.state.generation(),
                expires,
                grant,
                writable,
                len: bytes.len(),
                map,
                _file: file,
            },
        );
        Ok(cap)
    }
    /// Trusted host launcher installs caller data-plane mappings, never wire bytes.
    pub fn install_source(&mut self, domain: u64, bytes: &[u8]) -> Result<u64, u32> {
        self.mapping(domain, bytes, true, None)
    }
    fn check_mapping(&self, domain: u64, cap: u64) -> Result<&Mapping, u32> {
        if self.poisoned {
            return Err(STATUS_IO);
        }
        let m = self.mappings.get(&cap).ok_or(STATUS_DENIED)?;
        if m.domain != domain
            || m.generation != self.journal.state.generation()
            || self.now() >= m.expires
        {
            return Err(STATUS_DENIED);
        }
        if let Some(g) = m.grant {
            self.authorize(domain, g, 0)?;
        }
        Ok(m)
    }
    pub fn read_mapping(&self, domain: u64, cap: u64) -> Result<Vec<u8>, u32> {
        Ok(self.check_mapping(domain, cap)?.map[..].to_vec())
    }
    pub fn write_source(&mut self, domain: u64, cap: u64, bytes: &[u8]) -> Result<(), u32> {
        let m = self.check_mapping(domain, cap)?;
        if !m.writable || m.len != bytes.len() {
            return Err(STATUS_DENIED);
        }
        self.mappings
            .get_mut(&cap)
            .ok_or(STATUS_DENIED)?
            .map
            .copy_from_slice(bytes);
        Ok(())
    }
    pub fn release_mapping(&mut self, domain: u64, cap: u64) -> Result<(), u32> {
        self.check_mapping(domain, cap)?;
        self.mappings.remove(&cap);
        Ok(())
    }
    pub fn mapping_count(&mut self) -> usize {
        self.clean();
        self.mappings.len()
    }
    fn snapshot(&self, g: &Grant) -> Result<Vec<u8>, u32> {
        let b = serde_json::to_vec(&TaskSnapshotV1 {
            schema_version: 1,
            task_id: self.fixture.contract.task_id,
            resource_id: self.fixture.resource_id,
            revision: self.journal.state.accepted().revision,
            content_id: self.journal.state.accepted().content_id.clone(),
            grant_generation: g.generation,
            granted_rights: g.rights,
            grant_expires_at_ms: g.expires,
            now_ms: self.now(),
            schema_id: self.fixture.contract.schema_id.clone(),
            policy_id: self.fixture.contract.policy_id.clone(),
            validator_id: self.fixture.contract.validator_id.clone(),
            input_resources: [self.fixture.resource_id, SCHEMA_RESOURCE, NOTES_RESOURCE],
            validation: self.journal.last_validation.clone(),
            validation_current: self.journal.last_validation.as_ref().is_some_and(|e| {
                e.grant_generation == g.generation && self.now() < e.valid_until_ms
            }),
        })
        .map_err(|_| STATUS_IO)?;
        if b.len() > MAX_STATE {
            return Err(STATUS_CAPACITY);
        }
        Ok(b)
    }
    fn signal(&mut self, event: u32) {
        let revision = self.journal.state.accepted().revision;
        self.clean();
        for s in self.subscriptions.values_mut() {
            if s.mask & event != 0 {
                s.pending.insert(event, revision);
            }
        }
    }
    pub fn take_events(&mut self, domain: u64) -> Vec<Envelope> {
        self.connection_events(domain, None)
    }
    fn connection_events(&mut self, domain: u64, connection: Option<u64>) -> Vec<Envelope> {
        self.clean();
        let caps: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|(_, s)| {
                !s.pull && s.domain == domain && connection.is_none_or(|id| s.connection == id)
            })
            .map(|(&cap, _)| cap)
            .collect();
        let mut events = Vec::new();
        for cap in caps {
            let grant = self.subscriptions[&cap].grant;
            if let Ok(g) = self.authorize(domain, grant, RIGHT_OBSERVE) {
                let pending = std::mem::take(
                    &mut self
                        .subscriptions
                        .get_mut(&cap)
                        .expect("known subscription")
                        .pending,
                );
                for (ty, _) in pending {
                    let revision = self.journal.state.accepted().revision;
                    if let Ok(b) = self.snapshot(&g) {
                        if let Ok(map) = self.mapping(domain, &b, false, Some(grant)) {
                            events.push(envelope(
                                19,
                                TaskChangedEvent {
                                    subscription_cap: cap,
                                    revision,
                                    state_shm_cap: map,
                                    state_len: b.len() as u32,
                                    event_type: ty,
                                },
                            ));
                        }
                    }
                }
            }
        }
        events
    }
    pub fn verify_audit(&self) -> io::Result<()> {
        verify_journal(&self.journal)
    }
    pub fn evidence(&self) -> io::Result<Vec<u8>> {
        self.verify_audit()?;
        Ok(serde_json::to_vec(&self.journal)?)
    }
    fn audit(&mut self, domain: u64, env: &Envelope, code: u32, effect: bool) -> Result<(), u32> {
        if self.journal.audit.len() >= MAX_AUDIT {
            return Err(STATUS_CAPACITY);
        }
        let request = if env.payload_len >= 16 && env.payload_len <= 64 {
            u64::from_le_bytes(env.payload[8..16].try_into().expect("fixed slice"))
        } else {
            0
        };
        let mut a = Audit {
            seq: self.journal.audit.len() as u64 + 1,
            domain,
            op: env.msg_type,
            request,
            status: code,
            generation: self.journal.state.generation(),
            revision: self.journal.state.accepted().revision,
            content_id: self.journal.state.accepted().content_id.clone(),
            effect,
            previous: self
                .journal
                .audit
                .last()
                .map_or(String::new(), |a| a.digest.clone()),
            digest: String::new(),
        };
        a.digest = hash_bytes(&serde_json::to_vec(&a).map_err(|_| STATUS_IO)?);
        self.journal.audit.push(a);
        Ok(())
    }
    fn precheck(&self, domain: u64, request: &TaskRequest) -> Result<(), u32> {
        match request {
            TaskRequest::Grant(r) => {
                if domain != self.fixture.contract.domain_id
                    || r.policy_cap != self.policy_cap
                    || r.task_id != self.fixture.contract.task_id
                    || r.resource_id != self.fixture.resource_id
                    || r.rights & !self.policy.allowed_rights != 0
                    || r.lifetime_ms > self.policy.max_grant_ms
                {
                    return Err(STATUS_DENIED);
                }
            }
            TaskRequest::Revoke(r) => {
                if domain != self.fixture.contract.domain_id
                    || r.policy_cap != self.policy_cap
                    || !self.grants.contains_key(&r.task_cap)
                {
                    return Err(STATUS_DENIED);
                }
            }
            TaskRequest::Read(r) => {
                self.authorize(domain, r.task_cap, RIGHT_READ)?;
                if ![self.fixture.resource_id, SCHEMA_RESOURCE, NOTES_RESOURCE]
                    .contains(&r.resource_id)
                {
                    return Err(STATUS_DENIED);
                }
            }
            TaskRequest::Stage(r) => {
                self.authorize(domain, r.task_cap, RIGHT_STAGE)?;
                self.check_mapping(domain, r.source_shm_cap)?;
            }
            TaskRequest::Validate(r) => {
                self.authorize(domain, r.task_cap, RIGHT_VALIDATE)?;
                self.candidate(r.candidate_cap)?;
                if r.validator_content_id_hash != hash_array(&self.fixture.contract.validator_id) {
                    return Err(STATUS_DENIED);
                }
            }
            TaskRequest::Commit(r) => {
                self.authorize(domain, r.task_cap, RIGHT_COMMIT)?;
                self.candidate(r.candidate_cap)?;
            }
            TaskRequest::Receipt(r) => {
                self.authorize(domain, r.task_cap, RIGHT_COMMIT)?;
            }
            TaskRequest::State(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
            }
            TaskRequest::Subscribe(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
            }
            TaskRequest::SubscribePull(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
            }
            TaskRequest::Poll(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
            }
            TaskRequest::Unsubscribe(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
            }
        }
        Ok(())
    }
    pub fn active_worker_count(&self) -> usize {
        self.active_workers
    }
    fn operate(&mut self, domain: u64, req: TaskRequest, connection: u64) -> Result<Envelope, u32> {
        match req {
            TaskRequest::Grant(r) => {
                if domain != self.fixture.contract.domain_id
                    || r.policy_cap != self.policy_cap
                    || r.task_id != self.fixture.contract.task_id
                    || r.resource_id != self.fixture.resource_id
                    || r.rights & !self.policy.allowed_rights != 0
                    || r.lifetime_ms > self.policy.max_grant_ms
                {
                    return Err(STATUS_DENIED);
                }
                self.clean();
                if self.grants.len() >= MAX_OBJECTS {
                    return Err(STATUS_CAPACITY);
                }
                let cap = self.object_cap();
                let expires = self
                    .now()
                    .checked_add(r.lifetime_ms as u64)
                    .ok_or(STATUS_INVALID)?;
                self.grants.insert(
                    cap,
                    Grant {
                        domain,
                        rights: r.rights,
                        generation: self.journal.state.generation(),
                        expires,
                    },
                );
                Ok(envelope(
                    4,
                    RequestGrantReply {
                        request_id: r.request_id,
                        task_cap: cap,
                        generation: self.journal.state.generation(),
                        expires_at_ms: expires,
                        status: 0,
                        rights: r.rights,
                    },
                ))
            }
            TaskRequest::Read(r) => {
                self.authorize(domain, r.task_cap, RIGHT_READ)?;
                let b = if r.resource_id == self.fixture.resource_id {
                    self.accepted_bytes().map_err(|_| STATUS_IO)?
                } else if r.resource_id == SCHEMA_RESOURCE {
                    self.blob(&self.fixture.contract.schema_id)
                        .map_err(|_| STATUS_IO)?
                } else if r.resource_id == NOTES_RESOURCE {
                    self.blob(&self.journal.notes_id).map_err(|_| STATUS_IO)?
                } else {
                    return Err(STATUS_DENIED);
                };
                let map = self.mapping(domain, &b, false, Some(r.task_cap))?;
                Ok(envelope(
                    2,
                    ReadInputReply {
                        request_id: r.request_id,
                        input_shm_cap: map,
                        content_id_hash: content_hash(&b),
                        status: 0,
                        input_len: b.len() as u32,
                    },
                ))
            }
            TaskRequest::Stage(r) => {
                let g = self.authorize(domain, r.task_cap, RIGHT_STAGE)?;
                let m = self.check_mapping(domain, r.source_shm_cap)?;
                if r.source_len as usize > m.len {
                    return Err(STATUS_DENIED);
                }
                let bytes = m.map[..r.source_len as usize].to_vec();
                let id = hash_bytes(&bytes);
                if self.journal.candidates.len() >= 64
                    && !self.journal.candidates.iter().any(|c| c.id == id)
                {
                    return Err(STATUS_CAPACITY);
                }
                self.publish(&bytes).map_err(|_| STATUS_IO)?;
                self.journal
                    .state
                    .stage(&self.grant_model(&g), &id, self.now())
                    .map_err(status)?;
                let cap = if let Some(c) = self.journal.candidates.iter().find(|c| c.id == id) {
                    c.cap
                } else {
                    let cap = self.object_cap();
                    self.journal.candidates.push(Candidate {
                        cap,
                        id: id.clone(),
                    });
                    cap
                };
                Ok(envelope(
                    6,
                    StageCandidateReply {
                        request_id: r.request_id,
                        candidate_cap: cap,
                        content_id_hash: hash_array(&id),
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::Commit(r) => {
                let g = self.authorize(domain, r.task_cap, RIGHT_COMMIT)?;
                let id = self.candidate(r.candidate_cap)?;
                let binding = CommitBinding {
                    request_id: r.request_id,
                    candidate_cap: r.candidate_cap,
                    expected_revision: r.expected_revision,
                    expected_hash: r.expected_content_id_hash,
                };
                let receipt = if let Some(saved) = self
                    .journal
                    .receipts
                    .iter()
                    .find(|s| s.binding.request_id == r.request_id)
                {
                    if saved.binding != binding {
                        return Err(STATUS_REQUEST_REUSE);
                    }
                    saved.clone()
                } else {
                    self.blob(&id).map_err(|_| STATUS_IO)?;
                    let request = CommitRequestV0 {
                        request_id: r.request_id,
                        task_id: self.fixture.contract.task_id,
                        domain_id: domain,
                        resource: self.fixture.contract.resource.clone(),
                        candidate_id: id,
                        expected_revision: r.expected_revision,
                        expected_content_id: hash_id(r.expected_content_id_hash),
                    };
                    let committed_at_ms = self.now();
                    let validation = self
                        .journal
                        .state
                        .candidate_validation(&request.candidate_id)
                        .cloned()
                        .ok_or(STATUS_VALIDATION_FAILED)?;
                    let model_grant = self.grant_model(&g);
                    let receipt = self
                        .journal
                        .state
                        .commit(&model_grant, &request, committed_at_ms)
                        .map_err(status)?;
                    let saved = SavedReceipt {
                        binding,
                        cap: self.object_cap(),
                        receipt,
                        validation,
                        grant: model_grant,
                        committed_at_ms,
                    };
                    self.journal.receipts.push(saved.clone());
                    self.signal(EVENT_OUTPUT_CHANGED);
                    saved
                };
                Ok(envelope(
                    10,
                    CommitCandidateReply {
                        request_id: r.request_id,
                        receipt_cap: receipt.cap,
                        revision: receipt.receipt.accepted.revision,
                        content_id_hash: hash_array(&receipt.receipt.accepted.content_id),
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::Receipt(r) => {
                self.authorize(domain, r.task_cap, RIGHT_COMMIT)?;
                let saved = self
                    .journal
                    .receipts
                    .iter()
                    .find(|s| s.binding.request_id == r.commit_request_id)
                    .ok_or(STATUS_NOT_FOUND)?;
                Ok(envelope(
                    12,
                    GetReceiptReply {
                        request_id: r.request_id,
                        commit_request_id: r.commit_request_id,
                        revision: saved.receipt.accepted.revision,
                        content_id_hash: hash_array(&saved.receipt.accepted.content_id),
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::State(r) => {
                let g = self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
                let b = self.snapshot(&g)?;
                let cap = self.mapping(domain, &b, false, Some(r.task_cap))?;
                Ok(envelope(
                    14,
                    GetTaskStateReply {
                        request_id: r.request_id,
                        state_shm_cap: cap,
                        state_len: b.len() as u32,
                        reserved: 0,
                        status: 0,
                        reserved2: 0,
                    },
                ))
            }
            TaskRequest::Revoke(r) => {
                self.clean();
                if domain != self.fixture.contract.domain_id
                    || r.policy_cap != self.policy_cap
                    || !self.grants.contains_key(&r.task_cap)
                {
                    return Err(STATUS_DENIED);
                }
                let count = self.grants.len() as u32;
                self.journal.state.revoke().map_err(status)?;
                self.clean();
                Ok(envelope(
                    16,
                    RevokeGrantReply {
                        request_id: r.request_id,
                        generation: self.journal.state.generation(),
                        status: 0,
                        revoked_count: count,
                    },
                ))
            }
            TaskRequest::Subscribe(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
                self.clean();
                if self.subscriptions.len() >= MAX_OBJECTS {
                    return Err(STATUS_CAPACITY);
                }
                let cap = self.object_cap();
                self.subscriptions.insert(
                    cap,
                    Subscription {
                        connection,
                        domain,
                        grant: r.task_cap,
                        mask: r.event_mask,
                        pending: HashMap::new(),
                        pull: false,
                    },
                );
                Ok(envelope(
                    18,
                    SubscribeTaskReply {
                        request_id: r.request_id,
                        subscription_cap: cap,
                        revision: self.journal.state.accepted().revision,
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::SubscribePull(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
                self.clean();
                if self.subscriptions.len() >= MAX_OBJECTS
                    || self
                        .subscriptions
                        .values()
                        .filter(|s| s.pull && s.connection == connection)
                        .count()
                        >= 16
                {
                    return Err(STATUS_CAPACITY);
                }
                let cap = self.object_cap();
                self.subscriptions.insert(
                    cap,
                    Subscription {
                        connection,
                        domain,
                        grant: r.task_cap,
                        mask: r.event_mask,
                        pending: HashMap::new(),
                        pull: true,
                    },
                );
                Ok(envelope(
                    22,
                    SubscribeTaskPullReply {
                        request_id: r.request_id,
                        subscription_cap: cap,
                        revision: self.journal.state.accepted().revision,
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::Poll(r) => {
                let g = self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
                let sub = self
                    .subscriptions
                    .get(&r.subscription_cap)
                    .ok_or(STATUS_NOT_FOUND)?;
                if !sub.pull
                    || sub.domain != domain
                    || sub.connection != connection
                    || sub.grant != r.task_cap
                {
                    return Err(STATUS_DENIED);
                }
                let mask = sub.pending.keys().fold(0, |bits, ty| bits | ty);
                let (cap, len, revision) = if mask == 0 {
                    (0, 0, 0)
                } else {
                    let b = self.snapshot(&g)?;
                    let cap = self.mapping(domain, &b, false, Some(r.task_cap))?;
                    (cap, b.len() as u32, self.journal.state.accepted().revision)
                };
                // Drain only after a snapshot mapping succeeds. Delivery is
                // at-most-once; a lost reply requires an explicit state read.
                self.subscriptions
                    .get_mut(&r.subscription_cap)
                    .ok_or(STATUS_NOT_FOUND)?
                    .pending
                    .clear();
                Ok(envelope(
                    24,
                    PollTaskReply {
                        request_id: r.request_id,
                        state_shm_cap: cap,
                        revision,
                        state_len: len,
                        event_mask: mask,
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::Unsubscribe(r) => {
                self.authorize(domain, r.task_cap, RIGHT_OBSERVE)?;
                let sub = self
                    .subscriptions
                    .get(&r.subscription_cap)
                    .ok_or(STATUS_NOT_FOUND)?;
                if !sub.pull
                    || sub.domain != domain
                    || sub.connection != connection
                    || sub.grant != r.task_cap
                {
                    return Err(STATUS_DENIED);
                }
                self.subscriptions.remove(&r.subscription_cap);
                Ok(envelope(
                    26,
                    UnsubscribeTaskReply {
                        request_id: r.request_id,
                        status: 0,
                        reserved: 0,
                    },
                ))
            }
            TaskRequest::Validate(_) => Err(STATUS_INVALID),
        }
    }
}

/// Trusted launcher supplies domain from the connected transport. There is no
/// wire identity negotiation. All handlers recheck the service registry.
pub fn dispatch(
    service: &Arc<Mutex<TaskService>>,
    domain: u64,
    env: &Envelope,
) -> io::Result<Envelope> {
    dispatch_connection(service, domain, env, 0)
}
fn dispatch_connection(
    service: &Arc<Mutex<TaskService>>,
    domain: u64,
    env: &Envelope,
    connection: u64,
) -> io::Result<Envelope> {
    let invocation_start = Instant::now();
    let req = if env.handle != Handle::INVALID {
        Err(ProtocolError::InvalidFields)
    } else {
        parse_request(env)
    };
    let mut s = service.lock().map_err(|_| invalid("service lock"))?;
    if s.poisoned {
        return error_reply(env, STATUS_IO);
    }
    if s.journal.audit.len() >= MAX_AUDIT {
        let code = match &req {
            Ok(r) => s.precheck(domain, r).err().unwrap_or(STATUS_CAPACITY),
            Err(_) => STATUS_INVALID,
        };
        return error_reply(env, code);
    }
    let prior = s.journal.clone();
    let count = prior.receipts.len();
    let result = if let Ok(TaskRequest::Validate(r)) = req {
        let prepared = (|| -> Result<_, u32> {
            let g = s.authorize(domain, r.task_cap, RIGHT_VALIDATE)?;
            let id = s.candidate(r.candidate_cap)?;
            if r.validator_content_id_hash != hash_array(&s.fixture.contract.validator_id) {
                return Err(STATUS_DENIED);
            }
            if s.active_workers >= 2 {
                return Err(STATUS_CAPACITY);
            }
            Ok((
                g,
                id.clone(),
                s.worker.clone(),
                ValidatorJobV0 {
                    schema_version: 1,
                    store_root: s.cas.to_str().ok_or(STATUS_IO)?.to_owned(),
                    validator_id: s.fixture.contract.validator_id.clone(),
                    candidate_id: id.clone(),
                    schema_id: s.fixture.contract.schema_id.clone(),
                    budget: s.fixture.contract.execution_budget.clone(),
                },
            ))
        })();
        match prepared {
            Err(e) => Err(e),
            Ok((g, id, path, job)) => {
                // No service lock across compilation, guest execution or IPC.
                s.active_workers += 1;
                drop(s);
                let mut worker_job = job.clone();
                worker_job.budget.wall_ms = job
                    .budget
                    .wall_ms
                    .saturating_sub(invocation_start.elapsed().as_millis() as u64);
                worker_job.budget.guest_ms =
                    worker_job.budget.guest_ms.min(worker_job.budget.wall_ms);
                worker_job.budget.host_call_ms = worker_job
                    .budget
                    .host_call_ms
                    .min(worker_job.budget.wall_ms);
                let result = if worker_job.budget.wall_ms == 0 {
                    Err(STATUS_TIMEOUT)
                } else {
                    worker::supervise(&path, &worker_job)
                };
                s = service.lock().map_err(|_| invalid("service lock"))?;
                s.active_workers -= 1;
                (|| -> Result<Envelope, u32> {
                    let current = s.authorize(domain, r.task_cap, RIGHT_VALIDATE)?;
                    if current.generation != g.generation {
                        return Err(STATUS_DENIED);
                    }
                    let (mut outcome, diagnostics, truncated, _elapsed, guest_elapsed, mut code) =
                        match result {
                            Ok((result, elapsed)) => (
                                result.outcome,
                                result.diagnostics,
                                result.truncated,
                                elapsed,
                                result.guest_elapsed_ms,
                                STATUS_OK,
                            ),
                            Err(STATUS_CAPACITY) => (
                                ValidationOutcomeV0::HostFailure,
                                Vec::new(),
                                true,
                                0,
                                0,
                                STATUS_CAPACITY,
                            ),
                            Err(STATUS_TIMEOUT) => (
                                ValidationOutcomeV0::Timeout,
                                Vec::new(),
                                false,
                                job.budget.wall_ms,
                                0,
                                STATUS_TIMEOUT,
                            ),
                            Err(code) => (
                                ValidationOutcomeV0::HostFailure,
                                Vec::new(),
                                false,
                                0,
                                0,
                                code,
                            ),
                        };
                    let elapsed = invocation_start.elapsed().as_millis() as u64;
                    if elapsed > job.budget.wall_ms {
                        outcome = ValidationOutcomeV0::Timeout;
                        code = STATUS_TIMEOUT;
                    }
                    let now = s.now();
                    let valid_until = current.expires.min(now.saturating_add(job.budget.guest_ms));
                    let model_grant = s.grant_model(&current);
                    let evidence = ValidationEvidenceV0 {
                        task_id: s.fixture.contract.task_id,
                        domain_id: domain,
                        resource: s.fixture.contract.resource.clone(),
                        grant_generation: current.generation,
                        candidate_id: id,
                        schema_id: s.fixture.contract.schema_id.clone(),
                        policy_id: s.fixture.contract.policy_id.clone(),
                        validator_id: s.fixture.contract.validator_id.clone(),
                        valid_until_ms: valid_until,
                        wall_elapsed_ms: elapsed,
                        guest_elapsed_ms: guest_elapsed,
                        diagnostics_bytes: diagnostics.len() as u64,
                        diagnostics_truncated: truncated,
                        outcome,
                    };
                    s.journal
                        .state
                        .record_validation(&model_grant, evidence.clone(), now)
                        .map_err(status)?;
                    s.journal.last_validation = Some(evidence);
                    let map = if diagnostics.is_empty() {
                        0
                    } else {
                        s.mapping(domain, &diagnostics, false, Some(r.task_cap))?
                    };
                    s.signal(EVENT_VALIDATION_CHANGED);
                    let outcome = match outcome {
                        ValidationOutcomeV0::Valid => OUTCOME_VALID,
                        ValidationOutcomeV0::Invalid => OUTCOME_INVALID,
                        ValidationOutcomeV0::Timeout => OUTCOME_TIMEOUT,
                        ValidationOutcomeV0::HostFailure => OUTCOME_HOST_FAILURE,
                    };
                    Ok(envelope(
                        8,
                        ValidateCandidateReply {
                            request_id: r.request_id,
                            diagnostics_shm_cap: map,
                            valid_until_ms: valid_until,
                            status: code,
                            outcome,
                            diagnostics_len: diagnostics.len() as u32,
                            diagnostics_flags: if truncated { DIAGNOSTICS_TRUNCATED } else { 0 },
                        },
                    ))
                })()
            }
        }
    } else {
        match req {
            Ok(req) => s.operate(domain, req, connection),
            Err(_) => Err(STATUS_INVALID),
        }
    };
    // Another call may have changed state while validation ran. Never restore
    // an earlier journal across that unlocked interval.
    let code = result.as_ref().map_or_else(
        |e| *e,
        |reply| {
            let offset = match env.msg_type {
                1 | 5 => 48,
                3 => 32,
                7 | 13 | 17 | 21 => 24,
                23 => 32,
                25 => 8,
                9 | 11 => 56,
                15 => 16,
                _ => 0,
            };
            u32::from_le_bytes(
                reply.payload[offset..offset + 4]
                    .try_into()
                    .expect("status field"),
            )
        },
    );
    if result.is_err() && env.msg_type != 7 {
        s.journal = prior;
    }
    let effect = s.journal.receipts.len() > count && env.msg_type == 9 && result.is_ok();
    s.audit(domain, env, code, effect)
        .map_err(|_| invalid("audit capacity"))?;
    if s.persist().is_err() {
        return error_reply(env, STATUS_IO);
    }
    match result {
        Ok(reply) => Ok(reply),
        Err(code) => error_reply(env, code),
    }
}

// Existing bridge layout: explicit little-endian 88-byte frame, zero pad.
fn encode(e: &Envelope) -> [u8; 88] {
    let mut b = [0; 88];
    b[..4].copy_from_slice(&e.protocol.to_le_bytes());
    b[4..8].copy_from_slice(&e.msg_type.to_le_bytes());
    b[8..16].copy_from_slice(&e.handle.pack().to_le_bytes());
    b[16..20].copy_from_slice(&e.payload_len.to_le_bytes());
    b[20..84].copy_from_slice(&e.payload);
    b
}
fn receive(stream: &mut UnixStream, budget: Duration) -> io::Result<Envelope> {
    let deadline = Instant::now() + budget;
    let mut b = [0; 88];
    let mut read = 0;
    while read < 88 {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "frame deadline"))?;
        stream.set_read_timeout(Some(remaining))?;
        let n = match stream.read(&mut b[read..]) {
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(io::Error::new(
                    if read == 0 {
                        io::ErrorKind::WouldBlock
                    } else {
                        io::ErrorKind::TimedOut
                    },
                    "frame deadline",
                ));
            }
            Err(e) => return Err(e),
        };
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        read += n;
    }
    if b[84..] != [0; 4] {
        return Err(invalid("frame padding"));
    }
    let packed = u64::from_le_bytes(b[8..16].try_into().expect("fixed slice"));
    let handle = Handle::unpack(packed);
    if handle.pack() != packed {
        return Err(invalid("noncanonical endpoint"));
    }
    Ok(Envelope {
        protocol: u32::from_le_bytes(b[..4].try_into().expect("fixed slice")),
        msg_type: u32::from_le_bytes(b[4..8].try_into().expect("fixed slice")),
        handle,
        payload_len: u32::from_le_bytes(b[16..20].try_into().expect("fixed slice")),
        payload: b[20..84].try_into().expect("fixed slice"),
    })
}
pub fn exchange(stream: &mut UnixStream, env: &Envelope) -> io::Result<Envelope> {
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    stream.write_all(&encode(env))?;
    let deadline = Instant::now() + Duration::from_secs(6);
    let reply = loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "reply deadline"))?;
        let frame = receive(stream, remaining)?;
        if frame.msg_type == 19 {
            validate_event(&frame).map_err(|_| invalid("event preflight"))?;
        } else {
            break frame;
        }
    };
    let id = u64::from_le_bytes(env.payload[8..16].try_into().expect("request id"));
    validate_reply(&reply, env.msg_type, id).map_err(|_| invalid("reply preflight"))?;
    Ok(reply)
}
pub fn serve_connection(
    service: Arc<Mutex<TaskService>>,
    domain: u64,
    mut stream: UnixStream,
) -> io::Result<()> {
    let connection: u64 = rand::random();
    let result = (|| loop {
        let budget = service
            .lock()
            .map_err(|_| invalid("service lock"))?
            .fixture
            .contract
            .execution_budget
            .host_call_ms;
        match receive(&mut stream, Duration::from_millis(budget)) {
            Ok(env) => {
                let reply = dispatch_connection(&service, domain, &env, connection)?;
                stream.set_write_timeout(Some(Duration::from_millis(budget)))?;
                stream.write_all(&encode(&reply))?;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
        // Serialize authorized event delivery with revocation. Coalesced events
        // carry a fresh state snapshot; clients can always resynchronize by read.
        let mut locked = service.lock().map_err(|_| invalid("service lock"))?;
        for event in locked.connection_events(domain, Some(connection)) {
            stream.set_write_timeout(Some(Duration::from_millis(budget)))?;
            stream.write_all(&encode(&event))?;
        }
    })();
    if let Ok(mut locked) = service.lock() {
        locked
            .subscriptions
            .retain(|_, s| s.connection != connection);
    }
    result
}

pub fn next_event(stream: &mut UnixStream) -> io::Result<Envelope> {
    let event = receive(stream, Duration::from_secs(2))?;
    validate_event(&event).map_err(|_| invalid("event preflight"))?;
    Ok(event)
}

fn verify_journal(journal: &Journal) -> io::Result<()> {
    if journal.receipts.is_empty() && journal.state.accepted().content_id != journal.initial_id {
        return Err(invalid("initial output"));
    }
    if let Some((first, _)) = journal.state.committed_receipts().next() {
        if first.expected_content_id != journal.initial_id {
            return Err(invalid("initial receipt"));
        }
    }
    for candidate in &journal.candidates {
        if candidate.cap == 0
            || journal
                .candidates
                .iter()
                .filter(|c| c.cap == candidate.cap || c.id == candidate.id)
                .count()
                != 1
        {
            return Err(invalid("candidate registry replay"));
        }
    }
    for saved in &journal.receipts {
        if saved.cap == 0
            || journal.candidates.iter().any(|c| c.cap == saved.cap)
            || journal
                .receipts
                .iter()
                .filter(|s| s.cap == saved.cap)
                .count()
                != 1
            || !journal.candidates.iter().any(|c| {
                c.cap == saved.binding.candidate_cap && c.id == saved.receipt.accepted.content_id
            })
        {
            return Err(invalid("object registry replay"));
        }
    }
    if let Some(e) = &journal.last_validation {
        if journal.state.candidate_validation(&e.candidate_id) != Some(e) {
            return Err(invalid("latest validation replay"));
        }
    }

    journal
        .state
        .validate_journal()
        .map_err(|_| invalid("transaction replay"))?;
    if journal.state.committed_receipts().count() != journal.receipts.len() {
        return Err(invalid("receipt count"));
    }
    for ((request, receipt), saved) in journal.state.committed_receipts().zip(&journal.receipts) {
        if &saved.receipt != receipt
            || saved.binding.request_id != request.request_id
            || saved.binding.expected_revision != request.expected_revision
            || hash_id(saved.binding.expected_hash) != request.expected_content_id
        {
            return Err(invalid("receipt replay"));
        }
    }
    // Replay each persisted effect through independent reference semantics.
    // Generations/revisions are normalized only after their original bindings
    // and journal ordering have been checked; this avoids unbounded revoke loops.
    for ((original, receipt), saved) in journal.state.committed_receipts().zip(&journal.receipts) {
        if saved.grant.generation != saved.validation.grant_generation
            || saved.grant.generation != receipt.grant_generation
        {
            return Err(invalid("validation generation replay"));
        }
        let mut state =
            TaskStateV0::new(journal.state.contract().clone(), &receipt.prior.content_id)
                .map_err(|_| invalid("replay contract"))?;
        let mut grant = saved.grant.clone();
        grant.generation = 1;
        let mut validation = saved.validation.clone();
        validation.grant_generation = 1;
        state
            .stage(&grant, &original.candidate_id, saved.committed_at_ms)
            .map_err(|_| invalid("replay stage"))?;
        state
            .record_validation(&grant, validation, saved.committed_at_ms)
            .map_err(|_| invalid("replay validation"))?;
        let mut request = original.clone();
        request.expected_revision = 0;
        state
            .commit(&grant, &request, saved.committed_at_ms)
            .map_err(|_| invalid("replay commit"))?;
    }
    let mut previous = String::new();
    let mut effects = 0;
    for (index, event) in journal.audit.iter().enumerate() {
        let mut expected = event.clone();
        expected.digest.clear();
        if event.seq != index as u64 + 1
            || event.previous != previous
            || hash_bytes(&serde_json::to_vec(&expected)?) != event.digest
        {
            return Err(invalid("audit chain"));
        }
        if event.effect {
            effects += 1;
            let r = journal
                .receipts
                .get(effects - 1)
                .ok_or_else(|| invalid("audit effect"))?;
            if event.op != 9
                || event.status != STATUS_OK
                || event.request != r.receipt.request_id
                || event.domain != r.receipt.domain_id
                || event.revision != r.receipt.accepted.revision
                || event.content_id != r.receipt.accepted.content_id
            {
                return Err(invalid("audit receipt mismatch"));
            }
        }
        previous = event.digest.clone();
    }
    if effects != journal.receipts.len() {
        return Err(invalid("missing audit effect"));
    }
    Ok(())
}

pub fn verify_evidence(
    bytes: &[u8],
    contract: &TaskContractV0,
    initial: &[u8],
) -> io::Result<OutputRevisionV0> {
    if bytes.len() as u64 > MAX_JOURNAL {
        return Err(invalid("evidence bounds"));
    }
    let journal: Journal = serde_json::from_slice(bytes)?;
    if journal.version != 1
        || journal.state.contract() != contract
        || journal.initial_id != hash_bytes(initial)
        || journal.audit.len() > MAX_AUDIT
    {
        return Err(invalid("evidence identity"));
    }
    if let Some((first, _)) = journal.state.committed_receipts().next() {
        if first.expected_content_id != journal.initial_id {
            return Err(invalid("initial revision binding"));
        }
    }
    verify_journal(&journal)?;
    Ok(journal.state.accepted().clone())
}
