//! Ordinary host Store authority. Signed tokens require an exact durable live
//! grant; revocation permanently retires the ID. This is not DomainManager or
//! protection from a same-UID actor modifying the Store process/files directly.
use crate::capability::{CAPABILITY_VERSION, STORE_RIGHT_ALL, StoreCapability};
use anyhow::{Context, Result, ensure};
use bincode::Options;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::ops::Deref;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AUTHORITY_UPDATE_OPCODE: u8 = 9;
pub const AUTHORITY_VERSION: u16 = 1;
pub const AUTHORITY_TRANSITION_VERSION: u16 = 1;
const EPOCH_DATABASE_VERSION: u16 = 2;
pub const MAX_RETIRED_AUTHORITY_KEYS: usize = 64;
pub const MAX_AUTHORITY_TRANSITION_BYTES: u64 = 32768;
pub const AUTHORITY_GRANT: u8 = 1;
pub const AUTHORITY_REVOKE: u8 = 2;
pub const MAX_AUTHORITY_IDS: usize = 1024;
pub const LIVE_AUTHORITY_FILE: &str = ".live-authority.v1";
pub const LIVE_AUTHORITY_LOCK: &str = ".live-authority.lock.v1";
pub const LIVE_AUTHORITY_MARKER: &str = ".live-authority.initialized.v1";
const MARKER: &[u8] = b"RamenOS Store live authority v1\n";
const MAX_CAPABILITY_BYTES: usize = 512;
pub const MAX_AUTHORITY_REQUEST_BYTES: u64 = 1024;
// Each bounded ID reserves room for both its grant and its future revoke.
const MAX_DATABASE_BYTES: u64 = 2 * 1024 * 1024;

/// Offline, revocation-all transition. Both issuers sign the same canonical
/// bytes, including the complete sorted retired-key history. Private keys never
/// enter Store. Issuers must use exclusive keys across roots and epochs, and
/// choose a unique root ID at first enrollment; Store cannot enforce that global
/// property across independently operated roots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorityTransition {
    pub version: u16,
    pub root_id: [u8; 32],
    pub from_epoch: u64,
    pub from_state_digest: [u8; 32],
    pub to_epoch: u64,
    pub new_public_key: [u8; 32],
    pub retired_public_keys: Vec<[u8; 32]>,
    pub old_signature: Vec<u8>,
    pub new_signature: Vec<u8>,
}

impl AuthorityTransition {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = b"RamenOS.Store.AuthorityEpochTransition.v1\0".to_vec();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.root_id);
        bytes.extend_from_slice(&self.from_epoch.to_le_bytes());
        bytes.extend_from_slice(&self.from_state_digest);
        bytes.extend_from_slice(&self.to_epoch.to_le_bytes());
        bytes.extend_from_slice(&self.new_public_key);
        bytes.extend_from_slice(&(self.retired_public_keys.len() as u64).to_le_bytes());
        for key in &self.retired_public_keys {
            bytes.extend_from_slice(key);
        }
        bytes
    }

    pub fn checkpoint_digest(&self) -> Result<[u8; 32]> {
        self.validate_shape()?;
        Ok(Sha256::digest(bincode::serialize(self)?).into())
    }

    fn validate_shape(&self) -> Result<VerifyingKey> {
        ensure!(
            self.version == AUTHORITY_TRANSITION_VERSION,
            "invalid authority transition version"
        );
        ensure!(self.root_id != [0; 32], "authority root ID must be nonzero");
        ensure!(
            self.from_epoch.checked_add(1) == Some(self.to_epoch),
            "authority epoch must advance by one"
        );
        ensure!(
            self.retired_public_keys.len() <= MAX_RETIRED_AUTHORITY_KEYS,
            "authority retired-key history full"
        );
        ensure!(
            !self.retired_public_keys.is_empty(),
            "authority retired-key history missing"
        );
        ensure!(
            self.retired_public_keys
                .windows(2)
                .all(|keys| keys[0] < keys[1]),
            "authority retired-key history is not canonical"
        );
        ensure!(
            !self.retired_public_keys.contains(&self.new_public_key),
            "authority key was already used"
        );
        ensure!(
            self.old_signature.len() == 64 && self.new_signature.len() == 64,
            "invalid authority transition signature length"
        );
        for key in &self.retired_public_keys {
            VerifyingKey::from_bytes(key)?;
        }
        let new_key = VerifyingKey::from_bytes(&self.new_public_key)?;
        ensure!(
            !new_key.is_weak(),
            "weak authority verification key rejected"
        );
        Ok(new_key)
    }
}

/// Trusted operator configuration, stored outside the Store root. This exact
/// checkpoint pin is the rollback anchor; no hardware monotonicity is claimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityEpochFloor {
    pub root_id: [u8; 32],
    pub epoch: u64,
    pub checkpoint_digest: [u8; 32],
}

impl AuthorityEpochFloor {
    /// `roothex:epoch:checkpointSHA256hex`, lowercase hex and decimal epoch.
    pub fn encode(&self) -> String {
        format!(
            "{}:{}:{}",
            hex::encode(self.root_id),
            self.epoch,
            hex::encode(self.checkpoint_digest)
        )
    }

    pub fn parse(config: &str) -> Result<Self> {
        ensure!(config.len() <= 150, "authority epoch floor exceeds bounds");
        let fields: Vec<_> = config.split(':').collect();
        ensure!(
            fields.len() == 3 && fields[0].len() == 64 && fields[2].len() == 64,
            "invalid authority epoch floor"
        );
        let floor = Self {
            root_id: hex::decode(fields[0])?
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid root ID"))?,
            epoch: fields[1].parse()?,
            checkpoint_digest: hex::decode(fields[2])?
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid checkpoint digest"))?,
        };
        ensure!(
            floor.root_id != [0; 32] && floor.epoch > 0 && floor.encode() == config,
            "noncanonical authority epoch floor"
        );
        Ok(floor)
    }

    fn from_environment() -> Result<Option<Self>> {
        match std::env::var("RAMEN_STORE_AUTHORITY_EPOCH_FLOOR") {
            Ok(config) => Ok(Some(Self::parse(&config)?)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

/// Bounded regular-file JSON input for the offline tool. Serde rejects unknown
/// and duplicate fields, while from_slice rejects trailing non-whitespace data.
pub fn read_authority_transition(path: &Path) -> Result<AuthorityTransition> {
    let bytes = read_regular(path, MAX_AUTHORITY_TRANSITION_BYTES)?;
    let transition: AuthorityTransition = serde_json::from_slice(&bytes)?;
    transition.validate_shape()?;
    Ok(transition)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorityCommand {
    pub version: u16,
    pub action: u8,
    pub reserved: u8,
    pub capability_id: u64,
    pub capability_bytes: Vec<u8>,
    pub signature: Vec<u8>,
}
impl AuthorityCommand {
    /// Canonical domain-separated command signed by an external authority.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = b"RamenOS.Store.LiveAuthority.v1\0".to_vec();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.push(self.action);
        bytes.push(self.reserved);
        bytes.extend_from_slice(&self.capability_id.to_le_bytes());
        bytes.extend_from_slice(&(self.capability_bytes.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.capability_bytes);
        bytes
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorityUpdateRequest {
    pub request_id: u64,
    pub command: AuthorityCommand,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorityUpdateReply {
    pub request_id: u64,
    pub status: u32,
}

pub fn decode_authority_request(payload: &[u8]) -> Result<AuthorityUpdateRequest> {
    ensure!(
        payload.len() as u64 <= MAX_AUTHORITY_REQUEST_BYTES,
        "authority request too large"
    );
    Ok(bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(MAX_AUTHORITY_REQUEST_BYTES)
        .reject_trailing_bytes()
        .deserialize(payload)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    grant: Option<AuthorityCommand>,
    revoke: Option<AuthorityCommand>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Database {
    version: u16,
    records: BTreeMap<u64, Record>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpochDatabase {
    version: u16,
    checkpoint: AuthorityTransition,
    records: BTreeMap<u64, Record>,
}

#[derive(Default)]
struct ActiveRecord {
    leases: usize,
    draining: bool,
    #[cfg(test)]
    drain_waiters: usize,
}

pub struct LiveAuthority {
    root: Option<PathBuf>,
    keys: Vec<VerifyingKey>,
    database: Database,
    checkpoint: Option<AuthorityTransition>,
    failed: bool,
    active: BTreeMap<u64, ActiveRecord>,
    changed: Arc<Condvar>,
    _lock: Option<File>,
    #[cfg(test)]
    fail_after_rename: bool,
}

/// Admission remains live through consumption without holding the global state
/// mutex across read IO. Revocation closes admission and drains only this ID.
pub struct LiveCapability<'a> {
    capability: StoreCapability,
    authority: &'a Mutex<LiveAuthority>,
}
impl Deref for LiveCapability<'_> {
    type Target = StoreCapability;
    fn deref(&self) -> &Self::Target {
        &self.capability
    }
}

impl LiveCapability<'_> {
    /// Recheck global storage health before exposing a completed read. An
    /// unrelated management write may have become uncertain after admission.
    pub fn ensure_healthy(&self) -> Result<()> {
        drop(self.commit_guard()?);
        Ok(())
    }

    /// Publication must retain this exclusion across its actual durable commit.
    /// Management persistence cannot poison authority halfway through that commit.
    pub fn commit_guard(&self) -> Result<MutexGuard<'_, LiveAuthority>> {
        let guard = self
            .authority
            .lock()
            .map_err(|_| anyhow::anyhow!("live authority mutex poisoned"))?;
        ensure!(!guard.failed, "authority storage uncertain");
        Ok(guard)
    }
}

impl Drop for LiveCapability<'_> {
    fn drop(&mut self) {
        // Recover only to close accounting and poison authority. A poisoned
        // mutex never becomes a usable authority through lease cleanup.
        let mut guard = match self.authority.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.failed = true;
                guard
            }
        };
        if std::thread::panicking() {
            guard.failed = true;
        }
        let id = self.capability.capability_id;
        if let Some(record) = guard.active.get_mut(&id) {
            if let Some(remaining) = record.leases.checked_sub(1) {
                record.leases = remaining;
                if remaining == 0 && !record.draining {
                    guard.active.remove(&id);
                }
            } else {
                guard.failed = true;
            }
        } else {
            guard.failed = true;
        }
        guard.changed.notify_all();
    }
}

impl LiveAuthority {
    /// `development` is only the existing explicit RAMEN_STORE_DEV_MODE choice,
    /// or an explicit test fixture. It never follows a strict startup failure.
    pub fn open(root: &Path, development: bool) -> Result<Self> {
        Self::load_with_floor(
            root,
            development,
            crate::capability::load_authority_public_keys(),
            AuthorityEpochFloor::from_environment()?,
        )
    }
    #[cfg(test)]
    fn load(root: &Path, development: bool, keys: Vec<VerifyingKey>) -> Result<Self> {
        Self::load_with_floor(root, development, keys, None)
    }
    fn load_with_floor(
        root: &Path,
        development: bool,
        keys: Vec<VerifyingKey>,
        floor: Option<AuthorityEpochFloor>,
    ) -> Result<Self> {
        let mut this = Self {
            root: (!development).then(|| root.to_path_buf()),
            keys,
            database: Database {
                version: AUTHORITY_VERSION,
                records: BTreeMap::new(),
            },
            checkpoint: None,
            failed: false,
            active: BTreeMap::new(),
            changed: Arc::new(Condvar::new()),
            _lock: None,
            #[cfg(test)]
            fail_after_rename: false,
        };
        if development {
            return Ok(this);
        }
        ensure!(
            !this.keys.is_empty(),
            "explicit capability-authority keys required"
        );
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(root.join(LIVE_AUTHORITY_LOCK))?;
        ensure!(
            lock.metadata()?.is_file(),
            "authority lock is not a regular file"
        );
        // SAFETY: this File owns a live fd; flock is retained until the owning
        // LiveAuthority drops. The lock path is never unlinked during cleanup.
        let locked = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        ensure!(
            locked == 0,
            "Store root already has a live authority owner: {}",
            std::io::Error::last_os_error()
        );
        this._lock = Some(lock);
        let state = root.join(LIVE_AUTHORITY_FILE);
        let marker = root.join(LIVE_AUTHORITY_MARKER);
        let has_state = path_exists(&state)?;
        let has_marker = path_exists(&marker)?;
        ensure!(
            has_state == has_marker,
            "live authority state/initialization marker missing"
        );
        if has_state {
            ensure!(
                read_regular(&marker, MARKER.len() as u64)? == MARKER,
                "invalid live authority marker"
            );
            let bytes = read_regular(&state, MAX_DATABASE_BYTES)?;
            ensure!(bytes.len() >= 2, "truncated authority database version");
            match u16::from_le_bytes([bytes[0], bytes[1]]) {
                AUTHORITY_VERSION => {
                    ensure!(
                        floor.is_none(),
                        "pinned authority epoch rejects legacy database"
                    );
                    this.database = bincode::DefaultOptions::new()
                        .with_fixint_encoding()
                        .with_limit(MAX_DATABASE_BYTES)
                        .reject_trailing_bytes()
                        .deserialize(&bytes)
                        .context("invalid live authority database")?;
                }
                EPOCH_DATABASE_VERSION => {
                    let pinned = floor
                        .as_ref()
                        .context("explicit authority epoch floor required")?;
                    let database: EpochDatabase = bincode::DefaultOptions::new()
                        .with_fixint_encoding()
                        .with_limit(MAX_DATABASE_BYTES)
                        .reject_trailing_bytes()
                        .deserialize(&bytes)
                        .context("invalid authority epoch database")?;
                    let key = database.checkpoint.validate_shape()?;
                    ensure!(
                        this.keys.len() == 1 && this.keys[0] == key,
                        "active authority key must exactly match epoch checkpoint"
                    );
                    ensure!(
                        pinned.root_id == database.checkpoint.root_id
                            && pinned.epoch == database.checkpoint.to_epoch
                            && pinned.checkpoint_digest
                                == database.checkpoint.checkpoint_digest()?,
                        "authority epoch floor mismatch"
                    );
                    key.verify_strict(
                        &database.checkpoint.signing_bytes(),
                        &Signature::from_slice(&database.checkpoint.new_signature)?,
                    )
                    .context("authority checkpoint signature rejected")?;
                    this.database.records = database.records;
                    this.checkpoint = Some(database.checkpoint);
                }
                _ => anyhow::bail!("invalid authority database version"),
            }
            ensure!(
                this.encode_database(&this.database)? == bytes,
                "noncanonical or duplicate authority records"
            );
            ensure!(
                this.database.records.len() <= MAX_AUTHORITY_IDS,
                "authority registry too large"
            );
            for (id, record) in &this.database.records {
                ensure!(
                    record.grant.is_some() || record.revoke.is_some(),
                    "empty authority record"
                );
                for command in record.grant.iter().chain(record.revoke.iter()) {
                    ensure!(command.capability_id == *id, "authority record ID mismatch");
                    this.verify_command(command, false)?;
                }
                ensure!(
                    record
                        .grant
                        .as_ref()
                        .is_none_or(|c| c.action == AUTHORITY_GRANT),
                    "invalid stored grant"
                );
                ensure!(
                    record
                        .revoke
                        .as_ref()
                        .is_none_or(|c| c.action == AUTHORITY_REVOKE),
                    "invalid stored revoke"
                );
            }
        } else {
            ensure!(
                floor.is_none(),
                "pinned authority epoch has no durable database"
            );
            this.persist(&this.database)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&marker)?;
            file.write_all(MARKER)?;
            file.sync_all()?;
            File::open(root)?.sync_all()?;
        }
        Ok(this)
    }

    fn verify_command(&self, command: &AuthorityCommand, current: bool) -> Result<()> {
        ensure!(
            command.version == AUTHORITY_VERSION && command.reserved == 0,
            "invalid authority version/reserved"
        );
        ensure!(command.capability_id > 0, "authority ID must be positive");
        ensure!(
            command.signature.len() == 64 && command.capability_bytes.len() <= MAX_CAPABILITY_BYTES,
            "authority fields exceed bounds"
        );
        let signature = Signature::from_slice(&command.signature)?;
        ensure!(
            self.keys
                .iter()
                .any(|key| key.verify(&command.signing_bytes(), &signature).is_ok()),
            "authority signature rejected"
        );
        match command.action {
            AUTHORITY_GRANT => {
                let cap = decode_capability(&command.capability_bytes)?;
                ensure!(
                    cap.capability_id == command.capability_id,
                    "grant ID mismatch"
                );
                ensure!(
                    cap.verify_signature_with_keys(&self.keys),
                    "grant token signature rejected"
                );
                validate_capability_shape(&cap)?;
                if current {
                    ensure!(
                        !cap.is_expired() && cap.issued_at <= now(),
                        "stale or future grant"
                    );
                }
            }
            AUTHORITY_REVOKE => {
                ensure!(command.capability_bytes.is_empty(), "revoke carries token")
            }
            _ => anyhow::bail!("invalid authority action"),
        }
        Ok(())
    }

    /// Persist under the state lock; the shared entrypoint additionally drains
    /// admitted operations before acknowledging revocation.
    fn apply(&mut self, command: &AuthorityCommand) -> Result<()> {
        ensure!(!self.failed, "authority storage uncertain");
        ensure!(
            self.root.is_some(),
            "management disabled in stateless development mode"
        );
        // Historical exact replays do not extend a token's lifetime. A new
        // grant must be current; an expired exact replay can only be a no-op.
        self.verify_command(command, false)?;
        if command.action == AUTHORITY_GRANT
            && !self.database.records.contains_key(&command.capability_id)
        {
            self.verify_command(command, true)?;
        }
        let id = command.capability_id;
        let mut next = self.database.clone();
        if let Some(record) = next.records.get_mut(&id) {
            match command.action {
                AUTHORITY_GRANT => {
                    ensure!(record.revoke.is_none(), "authority ID permanently revoked");
                    ensure!(
                        record.grant.as_ref() == Some(command),
                        "authority ID already bound"
                    );
                    return Ok(());
                }
                AUTHORITY_REVOKE => {
                    if record.revoke.is_some() {
                        return Ok(());
                    }
                    record.revoke = Some(command.clone());
                }
                _ => unreachable!(),
            }
        } else {
            ensure!(
                next.records.len() < MAX_AUTHORITY_IDS,
                "authority registry full"
            );
            next.records.insert(
                id,
                Record {
                    grant: (command.action == AUTHORITY_GRANT).then(|| command.clone()),
                    revoke: (command.action == AUTHORITY_REVOKE).then(|| command.clone()),
                },
            );
        }
        if let Err(error) = self.persist(&next) {
            self.failed = true;
            return Err(error);
        }
        self.database = next;
        Ok(())
    }

    /// Persist before exposing a grant or acknowledging a revoke. Waiting for
    /// this ID's admitted work releases the global mutex, so unrelated IDs can
    /// still be managed. Every concurrent retry joins the same drain.
    pub fn apply_shared(authority: &Mutex<Self>, command: &AuthorityCommand) -> Result<()> {
        let mut guard = authority
            .lock()
            .map_err(|_| anyhow::anyhow!("live authority mutex poisoned"))?;
        if let Err(error) = guard.apply(command) {
            guard.changed.notify_all();
            return Err(error);
        }
        if command.action != AUTHORITY_REVOKE {
            return Ok(());
        }
        let id = command.capability_id;
        if let Some(record) = guard.active.get_mut(&id) {
            record.draining = true;
        }
        // Unit assertions observe both concurrent attempts inside the drain,
        // rather than inferring admission from thread scheduling or a sleep.
        #[cfg(test)]
        let counted_waiter = if let Some(record) = guard.active.get_mut(&id) {
            record.drain_waiters += 1;
            true
        } else {
            false
        };
        let changed = Arc::clone(&guard.changed);
        changed.notify_all();
        while guard
            .active
            .get(&id)
            .is_some_and(|record| record.leases != 0)
        {
            guard = changed
                .wait(guard)
                .map_err(|_| anyhow::anyhow!("live authority mutex poisoned"))?;
            ensure!(!guard.failed, "authority storage uncertain");
        }
        ensure!(!guard.failed, "authority storage uncertain");
        #[cfg(test)]
        if counted_waiter {
            if let Some(record) = guard.active.get_mut(&id) {
                record.drain_waiters -= 1;
            }
        }
        // Other waiters retain only the durable tombstone. Their wake-up must
        // not depend on this transient entry surviving the completed drain.
        guard.active.remove(&id);
        Ok(())
    }

    pub fn storage_uncertain(&self) -> bool {
        self.failed
    }

    /// Offline revocation-all rollover. The old owner must already be stopped;
    /// obtaining its root lock proves there are no leases in that old process.
    /// Source issuer keys and the current floor come only from explicit operator
    /// configuration. The returned next floor is public and must be maintained
    /// outside Store before the new epoch can start.
    pub fn rotate_offline(
        root: &Path,
        transition: &AuthorityTransition,
    ) -> Result<AuthorityEpochFloor> {
        Self::rotate_with(
            root,
            transition,
            crate::capability::load_authority_public_keys(),
            AuthorityEpochFloor::from_environment()?,
        )
    }

    fn rotate_with(
        root: &Path,
        transition: &AuthorityTransition,
        keys: Vec<VerifyingKey>,
        floor: Option<AuthorityEpochFloor>,
    ) -> Result<AuthorityEpochFloor> {
        // Migration never initializes a new root, even for an invalid proof.
        // load_with_floor then obtains the same lock used by the live service.
        read_regular(&root.join(LIVE_AUTHORITY_FILE), MAX_DATABASE_BYTES)?;
        ensure!(
            read_regular(&root.join(LIVE_AUTHORITY_MARKER), MARKER.len() as u64)? == MARKER,
            "invalid authority initialization marker"
        );
        let mut owner = Self::load_with_floor(root, false, keys, floor)?;
        owner.rotate(transition)
    }

    fn rotate(&mut self, transition: &AuthorityTransition) -> Result<AuthorityEpochFloor> {
        ensure!(!self.failed, "authority storage uncertain");
        ensure!(
            self.active.values().all(|record| record.leases == 0),
            "authority still has admitted operations"
        );
        let root = self
            .root
            .as_ref()
            .context("offline rotation requires durable authority")?;
        let new_key = transition.validate_shape()?;
        let from_epoch = self
            .checkpoint
            .as_ref()
            .map_or(0, |checkpoint| checkpoint.to_epoch);
        ensure!(
            transition.from_epoch == from_epoch,
            "authority source epoch mismatch"
        );
        if let Some(checkpoint) = &self.checkpoint {
            ensure!(
                transition.root_id == checkpoint.root_id,
                "authority root identity mismatch"
            );
        }
        let mut retired: BTreeSet<_> = self
            .checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.retired_public_keys.iter().copied().collect())
            .unwrap_or_default();
        retired.extend(self.keys.iter().map(VerifyingKey::to_bytes));
        ensure!(
            retired.len() <= MAX_RETIRED_AUTHORITY_KEYS,
            "authority retired-key history full"
        );
        ensure!(
            transition
                .retired_public_keys
                .iter()
                .copied()
                .eq(retired.iter().copied()),
            "authority retired-key history differs from source"
        );
        let signing_bytes = transition.signing_bytes();
        let old_signature = Signature::from_slice(&transition.old_signature)?;
        ensure!(
            self.keys
                .iter()
                .any(|key| key.verify_strict(&signing_bytes, &old_signature).is_ok()),
            "source authority transition signature rejected"
        );
        new_key
            .verify_strict(
                &signing_bytes,
                &Signature::from_slice(&transition.new_signature)?,
            )
            .context("new authority transition signature rejected")?;
        let source = read_regular(&root.join(LIVE_AUTHORITY_FILE), MAX_DATABASE_BYTES)?;
        ensure!(
            source == self.encode_database(&self.database)?,
            "authority source changed after load"
        );
        ensure!(
            <[u8; 32]>::from(Sha256::digest(&source)) == transition.from_state_digest,
            "authority source digest mismatch"
        );
        let next = EpochDatabase {
            version: EPOCH_DATABASE_VERSION,
            checkpoint: transition.clone(),
            records: BTreeMap::new(),
        };
        let bytes = bincode::serialize(&next)?;
        let floor = AuthorityEpochFloor {
            root_id: transition.root_id,
            epoch: transition.to_epoch,
            checkpoint_digest: transition.checkpoint_digest()?,
        };
        let result = self.persist_bytes(&bytes).and_then(|()| {
            ensure!(
                read_regular(&root.join(LIVE_AUTHORITY_FILE), MAX_DATABASE_BYTES)? == bytes,
                "authority epoch readback mismatch"
            );
            Ok(())
        });
        if let Err(error) = result {
            self.failed = true;
            return Err(error);
        }
        self.database.records.clear();
        self.checkpoint = Some(transition.clone());
        self.keys = vec![new_key];
        Ok(floor)
    }

    fn encode_database(&self, database: &Database) -> Result<Vec<u8>> {
        match &self.checkpoint {
            Some(checkpoint) => Ok(bincode::serialize(&EpochDatabase {
                version: EPOCH_DATABASE_VERSION,
                checkpoint: checkpoint.clone(),
                records: database.records.clone(),
            })?),
            None => Ok(bincode::serialize(database)?),
        }
    }

    fn persist(&self, database: &Database) -> Result<()> {
        self.persist_bytes(&self.encode_database(database)?)
    }

    fn persist_bytes(&self, bytes: &[u8]) -> Result<()> {
        let root = self
            .root
            .as_ref()
            .context("stateless authority has no persistence")?;
        let path = root.join(LIVE_AUTHORITY_FILE);
        if path_exists(&path)? {
            read_regular(&path, MAX_DATABASE_BYTES)?;
        }
        ensure!(
            bytes.len() as u64 <= MAX_DATABASE_BYTES,
            "authority database exceeds reserved bound"
        );
        let mut staging = tempfile::NamedTempFile::new_in(root)?;
        staging.write_all(bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(&path).map_err(|error| error.error)?;
        #[cfg(test)]
        ensure!(
            !self.fail_after_rename,
            "injected post-rename directory-sync failure"
        );
        File::open(root)?.sync_all()?;
        Ok(())
    }

    pub fn acquire(
        authority: &Mutex<Self>,
        capability: StoreCapability,
    ) -> Result<LiveCapability<'_>> {
        let mut guard = authority
            .lock()
            .map_err(|_| anyhow::anyhow!("live authority mutex poisoned"))?;
        ensure!(!guard.failed, "authority storage uncertain");
        if guard.root.is_some() {
            validate_capability_shape(&capability)?;
            ensure!(
                !capability.is_expired() && capability.issued_at <= now(),
                "stale capability"
            );
            ensure!(
                capability.verify_signature_with_keys(&guard.keys),
                "capability signature rejected"
            );
            let record = guard
                .database
                .records
                .get(&capability.capability_id)
                .context("unknown capability ID")?;
            ensure!(record.revoke.is_none(), "capability revoked");
            let grant = record.grant.as_ref().context("capability never granted")?;
            ensure!(
                grant.capability_bytes == bincode::serialize(&capability)?,
                "capability identity differs from live grant"
            );
        }
        ensure!(
            !guard
                .active
                .get(&capability.capability_id)
                .is_some_and(|record| record.draining),
            "capability draining"
        );
        ensure!(
            guard.active.contains_key(&capability.capability_id)
                || guard.active.len() < MAX_AUTHORITY_IDS,
            "active capability registry full"
        );
        let active = guard.active.entry(capability.capability_id).or_default();
        active.leases = active
            .leases
            .checked_add(1)
            .context("capability lease capacity reached")?;
        drop(guard);
        Ok(LiveCapability {
            capability,
            authority,
        })
    }
}

pub fn decode_capability(bytes: &[u8]) -> Result<StoreCapability> {
    ensure!(bytes.len() <= MAX_CAPABILITY_BYTES, "capability too large");
    Ok(bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(MAX_CAPABILITY_BYTES as u64)
        .reject_trailing_bytes()
        .deserialize(bytes)?)
}
fn validate_capability_shape(cap: &StoreCapability) -> Result<()> {
    ensure!(
        cap.version == CAPABILITY_VERSION && cap.capability_id > 0,
        "invalid capability version/ID"
    );
    ensure!(
        cap.rights_mask != 0 && cap.rights_mask & !STORE_RIGHT_ALL == 0,
        "invalid capability rights"
    );
    ensure!(
        cap.expires_at == 0 || cap.expires_at > cap.issued_at,
        "invalid capability lifetime"
    );
    Ok(())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn path_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn read_regular(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "authority state is not a regular file"
    );
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= maximum,
        "authority state exceeds bounds"
    );
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= maximum,
        "authority state grew beyond bound"
    );
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    fn signed_cap(key: &SigningKey, id: u64) -> StoreCapability {
        let mut cap = StoreCapability::new(7, STORE_RIGHT_ALL, id);
        cap.signature = key.sign(&cap.signing_data()).to_bytes().to_vec();
        cap
    }
    fn command(key: &SigningKey, action: u8, cap: &StoreCapability) -> AuthorityCommand {
        let mut command = AuthorityCommand {
            version: 1,
            action,
            reserved: 0,
            capability_id: cap.capability_id,
            capability_bytes: if action == AUTHORITY_GRANT {
                bincode::serialize(cap).unwrap()
            } else {
                vec![]
            },
            signature: vec![],
        };
        command.signature = key.sign(&command.signing_bytes()).to_bytes().to_vec();
        command
    }
    #[test]
    fn authority_capacity_reserves_every_existing_ids_revoke() {
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[63; 32]);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        // Build a full signed durable fixture without 1024 redundant fsyncs.
        for id in 1..MAX_AUTHORITY_IDS as u64 {
            live.database.records.insert(
                id,
                Record {
                    grant: Some(command(&key, AUTHORITY_GRANT, &signed_cap(&key, id))),
                    revoke: None,
                },
            );
        }
        live.persist(&live.database).unwrap();
        let last = signed_cap(&key, MAX_AUTHORITY_IDS as u64);
        live.apply(&command(&key, AUTHORITY_GRANT, &last)).unwrap();
        let excess = signed_cap(&key, MAX_AUTHORITY_IDS as u64 + 1);
        assert!(
            live.apply(&command(&key, AUTHORITY_GRANT, &excess))
                .is_err()
        );
        live.apply(&command(&key, AUTHORITY_REVOKE, &last)).unwrap();
        live.apply(&command(&key, AUTHORITY_REVOKE, &signed_cap(&key, 1)))
            .unwrap();
        // Preserve the exact signed token, including its original issued_at;
        // minting another token for ID 2 would intentionally be an ID alias.
        let retained = decode_capability(
            &live
                .database
                .records
                .get(&2)
                .unwrap()
                .grant
                .as_ref()
                .unwrap()
                .capability_bytes,
        )
        .unwrap();
        drop(live);
        let live =
            Mutex::new(LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap());
        assert!(LiveAuthority::acquire(&live, last).is_err());
        assert!(LiveAuthority::acquire(&live, retained).is_ok());
    }
    #[test]
    fn uncertain_revoke_io_poison_denies_until_verified_restart() {
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[64; 32]);
        let cap = signed_cap(&key, 1);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &cap)).unwrap();
        live.fail_after_rename = true;
        assert!(live.apply(&command(&key, AUTHORITY_REVOKE, &cap)).is_err());
        assert!(
            live.apply(&command(&key, AUTHORITY_GRANT, &signed_cap(&key, 2)))
                .is_err()
        );
        let live = Mutex::new(live);
        assert!(
            LiveAuthority::acquire(&live, cap.clone()).is_err(),
            "uncertain revoke left old token live"
        );
        drop(live);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        assert!(live.apply(&command(&key, AUTHORITY_GRANT, &cap)).is_err());
        let fresh = signed_cap(&key, 2);
        live.apply(&command(&key, AUTHORITY_GRANT, &fresh)).unwrap();
        let live = Mutex::new(live);
        assert!(LiveAuthority::acquire(&live, cap).is_err());
        assert!(LiveAuthority::acquire(&live, fresh).is_ok());
    }
    #[test]
    fn live_lease_drains_same_id_and_concurrent_retry_without_blocking_other_ids() {
        use std::sync::mpsc;
        use std::time::Duration;
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[65; 32]);
        let cap = signed_cap(&key, 1);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &cap)).unwrap();
        let live = Mutex::new(live);
        let revoke = command(&key, AUTHORITY_REVOKE, &cap);
        std::thread::scope(|scope| {
            let lease = LiveAuthority::acquire(&live, cap.clone()).unwrap();
            assert!(live.try_lock().is_ok(), "read lease retained global lock");
            assert_eq!(lease.capability_id, cap.capability_id);
            let (tx, rx) = mpsc::channel();
            for _ in 0..2 {
                let tx = tx.clone();
                let live = &live;
                let revoke = &revoke;
                scope.spawn(move || tx.send(LiveAuthority::apply_shared(live, revoke)).unwrap());
            }
            drop(tx);
            let mut state = live.lock().unwrap();
            let changed = Arc::clone(&state.changed);
            while !state
                .active
                .get(&cap.capability_id)
                .is_some_and(|record| record.draining && record.drain_waiters == 2)
            {
                let (next, timeout) = changed.wait_timeout(state, Duration::from_secs(5)).unwrap();
                state = next;
                assert!(
                    !timeout.timed_out(),
                    "both revoke attempts never joined drain"
                );
            }
            assert!(state.database.records[&cap.capability_id].revoke.is_some());
            assert_eq!(state.active[&cap.capability_id].leases, 1);
            assert!(
                rx.try_recv().is_err(),
                "revoke acknowledged before consumption ended"
            );
            drop(state);
            assert!(LiveAuthority::acquire(&live, cap.clone()).is_err());
            let other = signed_cap(&key, 2);
            LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_GRANT, &other)).unwrap();
            let other_lease = LiveAuthority::acquire(&live, other.clone()).unwrap();
            other_lease.ensure_healthy().unwrap();
            drop(other_lease);
            LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_REVOKE, &other)).unwrap();
            drop(lease);
            for _ in 0..2 {
                rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
            }
        });
        assert!(LiveAuthority::acquire(&live, cap.clone()).is_err());
        LiveAuthority::apply_shared(&live, &revoke).unwrap();
        assert!(LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_GRANT, &cap)).is_err());
        drop(live);
        let live =
            Mutex::new(LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap());
        assert!(LiveAuthority::acquire(&live, cap).is_err());
        let fresh = signed_cap(&key, 3);
        LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_GRANT, &fresh)).unwrap();
        LiveAuthority::acquire(&live, fresh)
            .unwrap()
            .ensure_healthy()
            .unwrap();
    }
    #[test]
    fn admitted_read_rechecks_health_after_unrelated_uncertain_persistence() {
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[68; 32]);
        let cap = signed_cap(&key, 1);
        let other = signed_cap(&key, 2);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &cap)).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &other)).unwrap();
        let live = Mutex::new(live);
        let admitted = LiveAuthority::acquire(&live, cap.clone()).unwrap();
        admitted.ensure_healthy().unwrap();
        live.lock().unwrap().fail_after_rename = true;
        assert!(
            LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_REVOKE, &other)).is_err()
        );
        assert!(
            admitted.ensure_healthy().is_err(),
            "admitted read exposed success after poison"
        );
        assert!(
            admitted.commit_guard().is_err(),
            "publication started after poison"
        );
        assert!(LiveAuthority::acquire(&live, cap.clone()).is_err());
        drop(admitted);
        drop(live);
        let live =
            Mutex::new(LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap());
        assert!(LiveAuthority::acquire(&live, other).is_err());
        LiveAuthority::acquire(&live, cap)
            .unwrap()
            .ensure_healthy()
            .unwrap();
    }
    #[test]
    fn admitted_publication_excludes_management_poison_until_commit_finishes() {
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[69; 32]);
        let cap = signed_cap(&key, 1);
        let other = signed_cap(&key, 2);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &cap)).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &other)).unwrap();
        let live = Mutex::new(live);
        let admitted = LiveAuthority::acquire(&live, cap.clone()).unwrap();
        live.lock().unwrap().fail_after_rename = true;
        let commit = admitted.commit_guard().unwrap();
        let bytes = b"publication before authority uncertainty";
        let manifest = artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: artifact_store_core::hash_bytes(bytes),
            size_bytes: bytes.len() as u64,
            kind: "config".into(),
            channels: vec!["test".into()],
            signatures: vec![],
        };
        let id = artifact_store_schema::ContentId::parse(&manifest.content_id).unwrap();
        let mut registry =
            crate::domain_visibility::DomainArtifactRegistry::new(root.path()).unwrap();
        registry
            .publish_owned_exact(root.path(), &manifest, cap.domain_id, false, |blob| {
                // The real publisher has persisted its intent before calling
                // this closure. Authority updates cannot enter that commit.
                assert!(matches!(
                    live.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                fs::write(blob, bytes)?;
                File::open(blob)?.sync_all()
            })
            .unwrap();
        assert!(!commit.storage_uncertain());
        drop(commit);
        assert!(
            LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_REVOKE, &other)).is_err()
        );
        let blob = root.path().join(format!("{}.blob", id.hash_hex()));
        assert_eq!(fs::read(&blob).unwrap(), bytes);
        assert!(registry.can_access(&id, cap.domain_id));
        artifact_store_core::verify_blob_identity(&blob, &manifest, &id).unwrap();
        assert!(admitted.ensure_healthy().is_err());
    }
    #[test]
    fn panicking_lease_drop_poison_denies_until_verified_restart() {
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[70; 32]);
        let cap = signed_cap(&key, 1);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &cap)).unwrap();
        let live = Mutex::new(live);
        let panic = std::panic::catch_unwind(|| {
            let _admitted = LiveAuthority::acquire(&live, cap.clone()).unwrap();
            panic!("admitted consumer panicked");
        });
        assert!(panic.is_err());
        assert!(
            live.lock().unwrap().active.is_empty(),
            "panic leaked its active lease"
        );
        assert!(LiveAuthority::acquire(&live, cap.clone()).is_err());
        assert!(
            LiveAuthority::apply_shared(&live, &command(&key, AUTHORITY_REVOKE, &cap)).is_err()
        );
        drop(live);
        let live =
            Mutex::new(LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap());
        LiveAuthority::acquire(&live, cap)
            .unwrap()
            .ensure_healthy()
            .unwrap();
    }
    #[test]
    fn development_active_id_capacity_is_bounded_and_drop_frees_the_next_id() {
        let root = tempfile::tempdir().unwrap();
        let live = Mutex::new(LiveAuthority::load(root.path(), true, vec![]).unwrap());
        let mut leases = Vec::new();
        for id in 1..=MAX_AUTHORITY_IDS as u64 {
            leases.push(
                LiveAuthority::acquire(&live, StoreCapability::new(7, STORE_RIGHT_ALL, id))
                    .unwrap(),
            );
        }
        let next = StoreCapability::new(7, STORE_RIGHT_ALL, MAX_AUTHORITY_IDS as u64 + 1);
        assert!(LiveAuthority::acquire(&live, next.clone()).is_err());
        let existing =
            LiveAuthority::acquire(&live, StoreCapability::new(7, STORE_RIGHT_ALL, 1)).unwrap();
        existing.ensure_healthy().unwrap();
        drop(existing);
        assert_eq!(live.lock().unwrap().active.len(), MAX_AUTHORITY_IDS);
        drop(leases.pop());
        let admitted = LiveAuthority::acquire(&live, next).unwrap();
        admitted.ensure_healthy().unwrap();
        drop(admitted);
        drop(leases);
        assert!(live.lock().unwrap().active.is_empty());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
    #[test]
    fn authority_roots_are_independent_and_missing_explicit_keys_deny() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[66; 32]);
        assert!(LiveAuthority::load(root.path(), false, vec![]).is_err());
        let _first = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        assert!(LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).is_err());
        assert!(LiveAuthority::load(other.path(), false, vec![key.verifying_key()]).is_ok());
    }
    #[test]
    fn stored_duplicate_ids_and_noncanonical_order_fail_closed() {
        #[derive(Serialize)]
        struct EncodedRecords {
            version: u16,
            records: Vec<(u64, Record)>,
        }
        let key = SigningKey::from_bytes(&[67; 32]);
        for duplicate in [true, false] {
            let root = tempfile::tempdir().unwrap();
            let live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
            drop(live);
            let cap = signed_cap(&key, 1);
            let grant = Record {
                grant: Some(command(&key, AUTHORITY_GRANT, &cap)),
                revoke: None,
            };
            let records = if duplicate {
                vec![
                    (
                        1,
                        Record {
                            grant: grant.grant.clone(),
                            revoke: Some(command(&key, AUTHORITY_REVOKE, &cap)),
                        },
                    ),
                    (1, grant),
                ]
            } else {
                vec![
                    (
                        2,
                        Record {
                            grant: Some(command(&key, AUTHORITY_GRANT, &signed_cap(&key, 2))),
                            revoke: None,
                        },
                    ),
                    (1, grant),
                ]
            };
            fs::write(
                root.path().join(LIVE_AUTHORITY_FILE),
                bincode::serialize(&EncodedRecords {
                    version: 1,
                    records,
                })
                .unwrap(),
            )
            .unwrap();
            assert!(LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).is_err());
        }
    }
}

#[cfg(test)]
mod epoch_tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    fn cap(key: &SigningKey, id: u64) -> StoreCapability {
        let mut cap = StoreCapability::new(7, STORE_RIGHT_ALL, id);
        cap.signature = key.sign(&cap.signing_data()).to_bytes().to_vec();
        cap
    }
    fn command(key: &SigningKey, action: u8, cap: &StoreCapability) -> AuthorityCommand {
        let mut command = AuthorityCommand {
            version: AUTHORITY_VERSION,
            action,
            reserved: 0,
            capability_id: cap.capability_id,
            capability_bytes: if action == AUTHORITY_GRANT {
                bincode::serialize(cap).unwrap()
            } else {
                vec![]
            },
            signature: vec![],
        };
        command.signature = key.sign(&command.signing_bytes()).to_bytes().to_vec();
        command
    }
    fn transition(
        root: &Path,
        old: &SigningKey,
        new: &SigningKey,
        from_epoch: u64,
        retired: Vec<[u8; 32]>,
    ) -> AuthorityTransition {
        let mut transition = AuthorityTransition {
            version: AUTHORITY_TRANSITION_VERSION,
            root_id: Sha256::digest(root.as_os_str().as_encoded_bytes()).into(),
            from_epoch,
            from_state_digest: Sha256::digest(fs::read(root.join(LIVE_AUTHORITY_FILE)).unwrap())
                .into(),
            to_epoch: from_epoch + 1,
            new_public_key: new.verifying_key().to_bytes(),
            retired_public_keys: retired,
            old_signature: vec![],
            new_signature: vec![],
        };
        resign(&mut transition, old, new);
        transition
    }
    fn resign(transition: &mut AuthorityTransition, old: &SigningKey, new: &SigningKey) {
        transition.old_signature = old.sign(&transition.signing_bytes()).to_bytes().to_vec();
        transition.new_signature = new.sign(&transition.signing_bytes()).to_bytes().to_vec();
    }
    fn initial(root: &Path, old: &SigningKey) -> StoreCapability {
        let mut live = LiveAuthority::load(root, false, vec![old.verifying_key()]).unwrap();
        let cap = cap(old, 1);
        live.apply(&command(old, AUTHORITY_GRANT, &cap)).unwrap();
        cap
    }

    #[test]
    fn epoch_v1_capacity_and_historical_key_removal_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[71; 32]);
        let new = SigningKey::from_bytes(&[72; 32]);
        let mut live = LiveAuthority::load(root.path(), false, vec![old.verifying_key()]).unwrap();
        for id in 1..=MAX_AUTHORITY_IDS as u64 {
            live.database.records.insert(
                id,
                Record {
                    grant: None,
                    revoke: Some(command(&old, AUTHORITY_REVOKE, &cap(&old, id))),
                },
            );
        }
        live.persist(&live.database).unwrap();
        assert!(
            live.apply(&command(&old, AUTHORITY_GRANT, &cap(&old, 1025)))
                .is_err()
        );
        live.apply(&command(&old, AUTHORITY_REVOKE, &cap(&old, 1)))
            .unwrap();
        drop(live);
        assert!(LiveAuthority::load(root.path(), false, vec![new.verifying_key()]).is_err());
        assert!(
            LiveAuthority::load(
                root.path(),
                false,
                vec![old.verifying_key(), new.verifying_key()]
            )
            .is_ok()
        );
    }

    #[test]
    fn epoch_offline_fresh_key_resets_capacity_and_retires_every_old_operation() {
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[73; 32]);
        let new = SigningKey::from_bytes(&[74; 32]);
        let old_cap = initial(root.path(), &old);
        let mut full = LiveAuthority::load(root.path(), false, vec![old.verifying_key()]).unwrap();
        for id in 2..=MAX_AUTHORITY_IDS as u64 {
            full.database.records.insert(
                id,
                Record {
                    grant: None,
                    revoke: Some(command(&old, AUTHORITY_REVOKE, &cap(&old, id))),
                },
            );
        }
        full.persist(&full.database).unwrap();
        assert!(
            full.apply(&command(&old, AUTHORITY_GRANT, &cap(&old, 1025)))
                .is_err()
        );
        drop(full);
        let proof = transition(
            root.path(),
            &old,
            &new,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let floor =
            LiveAuthority::rotate_with(root.path(), &proof, vec![old.verifying_key()], None)
                .unwrap();
        assert_eq!(AuthorityEpochFloor::parse(&floor.encode()).unwrap(), floor);
        assert!(
            LiveAuthority::load_with_floor(root.path(), false, vec![new.verifying_key()], None)
                .is_err()
        );
        let live = Mutex::new(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor.clone()),
            )
            .unwrap(),
        );
        assert!(live.lock().unwrap().database.records.is_empty());
        assert!(LiveAuthority::acquire(&live, old_cap.clone()).is_err());
        for action in [AUTHORITY_GRANT, AUTHORITY_REVOKE] {
            assert!(LiveAuthority::apply_shared(&live, &command(&old, action, &old_cap)).is_err());
        }
        let new_cap = cap(&new, old_cap.capability_id);
        LiveAuthority::apply_shared(&live, &command(&new, AUTHORITY_GRANT, &new_cap)).unwrap();
        LiveAuthority::acquire(&live, new_cap.clone())
            .unwrap()
            .ensure_healthy()
            .unwrap();
        drop(live);
        let live = Mutex::new(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor),
            )
            .unwrap(),
        );
        LiveAuthority::acquire(&live, new_cap)
            .unwrap()
            .ensure_healthy()
            .unwrap();
        assert!(LiveAuthority::acquire(&live, old_cap).is_err());
    }

    #[test]
    fn epoch_live_owner_stale_proof_and_pinned_old_backup_deny() {
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[75; 32]);
        let new = SigningKey::from_bytes(&[76; 32]);
        initial(root.path(), &old);
        let backup = fs::read(root.path().join(LIVE_AUTHORITY_FILE)).unwrap();
        let proof = transition(
            root.path(),
            &old,
            &new,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let owner = LiveAuthority::load(root.path(), false, vec![old.verifying_key()]).unwrap();
        assert!(
            LiveAuthority::rotate_with(root.path(), &proof, vec![old.verifying_key()], None)
                .is_err()
        );
        drop(owner);
        let floor =
            LiveAuthority::rotate_with(root.path(), &proof, vec![old.verifying_key()], None)
                .unwrap();
        assert!(
            LiveAuthority::rotate_with(
                root.path(),
                &proof,
                vec![new.verifying_key()],
                Some(floor.clone())
            )
            .is_err()
        );
        let current = fs::read(root.path().join(LIVE_AUTHORITY_FILE)).unwrap();
        fs::write(root.path().join(LIVE_AUTHORITY_FILE), backup).unwrap();
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![old.verifying_key()],
                Some(floor.clone())
            )
            .is_err()
        );
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor.clone())
            )
            .is_err()
        );
        fs::write(root.path().join(LIVE_AUTHORITY_FILE), current).unwrap();
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor)
            )
            .is_ok()
        );
    }

    #[test]
    fn epoch_transition_rejects_source_epoch_root_history_key_reuse_and_signature_errors() {
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[77; 32]);
        let current = SigningKey::from_bytes(&[78; 32]);
        let next = SigningKey::from_bytes(&[79; 32]);
        initial(root.path(), &old);
        let first = transition(
            root.path(),
            &old,
            &current,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let floor =
            LiveAuthority::rotate_with(root.path(), &first, vec![old.verifying_key()], None)
                .unwrap();
        let mut history = vec![
            old.verifying_key().to_bytes(),
            current.verifying_key().to_bytes(),
        ];
        history.sort();
        let valid = transition(root.path(), &current, &next, 1, history);
        for case in 0..10 {
            let mut bad = valid.clone();
            match case {
                0 => bad.from_state_digest[0] ^= 1,
                1 => bad.from_epoch = 0,
                2 => bad.to_epoch += 1,
                3 => bad.root_id[0] ^= 1,
                4 => bad.retired_public_keys.clear(),
                5 => bad.retired_public_keys.push(bad.retired_public_keys[0]),
                6 => bad.retired_public_keys.reverse(),
                7 => bad.new_public_key = old.verifying_key().to_bytes(),
                8 => bad.new_public_key = current.verifying_key().to_bytes(),
                _ => bad.version += 1,
            }
            resign(&mut bad, &current, &next);
            assert!(
                LiveAuthority::rotate_with(
                    root.path(),
                    &bad,
                    vec![current.verifying_key()],
                    Some(floor.clone())
                )
                .is_err(),
                "transition case {case} accepted"
            );
        }
        for old_signature in [true, false] {
            let mut bad = valid.clone();
            if old_signature {
                bad.old_signature[0] ^= 1;
            } else {
                bad.new_signature[0] ^= 1;
            }
            assert!(
                LiveAuthority::rotate_with(
                    root.path(),
                    &bad,
                    vec![current.verifying_key()],
                    Some(floor.clone())
                )
                .is_err()
            );
        }
        let second_floor = LiveAuthority::rotate_with(
            root.path(),
            &valid,
            vec![current.verifying_key()],
            Some(floor),
        )
        .unwrap();
        assert_eq!(second_floor.epoch, 2);
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![next.verifying_key()],
                Some(second_floor)
            )
            .is_ok()
        );
    }

    #[test]
    fn epoch_floor_and_active_key_must_match_checkpoint_exactly() {
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[80; 32]);
        let new = SigningKey::from_bytes(&[81; 32]);
        initial(root.path(), &old);
        let proof = transition(
            root.path(),
            &old,
            &new,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let floor =
            LiveAuthority::rotate_with(root.path(), &proof, vec![old.verifying_key()], None)
                .unwrap();
        for keys in [
            vec![old.verifying_key()],
            vec![new.verifying_key(), old.verifying_key()],
            vec![],
        ] {
            assert!(
                LiveAuthority::load_with_floor(root.path(), false, keys, Some(floor.clone()))
                    .is_err()
            );
        }
        for case in 0..3 {
            let mut bad = floor.clone();
            match case {
                0 => bad.root_id[0] ^= 1,
                1 => bad.epoch += 1,
                _ => bad.checkpoint_digest[0] ^= 1,
            }
            assert!(
                LiveAuthority::load_with_floor(
                    root.path(),
                    false,
                    vec![new.verifying_key()],
                    Some(bad)
                )
                .is_err()
            );
        }
        assert!(AuthorityEpochFloor::parse("bad-floor").is_err());
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor)
            )
            .is_ok()
        );
    }

    #[test]
    fn epoch_history_is_bounded_and_post_rename_cutover_failure_is_not_acknowledged() {
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[82; 32]);
        let new = SigningKey::from_bytes(&[83; 32]);
        initial(root.path(), &old);
        let proof = transition(
            root.path(),
            &old,
            &new,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let mut owner = LiveAuthority::load(root.path(), false, vec![old.verifying_key()]).unwrap();
        owner.fail_after_rename = true;
        assert!(owner.rotate(&proof).is_err());
        assert!(owner.storage_uncertain());
        drop(owner);
        assert!(
            LiveAuthority::load_with_floor(root.path(), false, vec![old.verifying_key()], None)
                .is_err()
        );
        assert!(
            LiveAuthority::load_with_floor(root.path(), false, vec![new.verifying_key()], None)
                .is_err()
        );
        let floor = AuthorityEpochFloor {
            root_id: proof.root_id,
            epoch: proof.to_epoch,
            checkpoint_digest: proof.checkpoint_digest().unwrap(),
        };
        let owner = LiveAuthority::load_with_floor(
            root.path(),
            false,
            vec![new.verifying_key()],
            Some(floor),
        )
        .unwrap();
        drop(owner);

        let full = tempfile::tempdir().unwrap();
        let keys: Vec<_> = (100..100 + MAX_RETIRED_AUTHORITY_KEYS as u8)
            .map(|n| SigningKey::from_bytes(&[n; 32]))
            .collect();
        drop(
            LiveAuthority::load(
                full.path(),
                false,
                keys.iter().map(SigningKey::verifying_key).collect(),
            )
            .unwrap(),
        );
        let mut history: Vec<_> = keys
            .iter()
            .map(|key| key.verifying_key().to_bytes())
            .collect();
        history.sort();
        let proof = transition(full.path(), &keys[0], &new, 0, history);
        let floor = LiveAuthority::rotate_with(
            full.path(),
            &proof,
            keys.iter().map(SigningKey::verifying_key).collect(),
            None,
        )
        .unwrap();
        let next = SigningKey::from_bytes(&[84; 32]);
        let mut history = proof.retired_public_keys.clone();
        history.push(new.verifying_key().to_bytes());
        history.sort();
        let excess = transition(full.path(), &new, &next, 1, history);
        assert!(
            LiveAuthority::rotate_with(
                full.path(),
                &excess,
                vec![new.verifying_key()],
                Some(floor.clone())
            )
            .is_err()
        );
        assert!(
            LiveAuthority::load_with_floor(
                full.path(),
                false,
                vec![new.verifying_key()],
                Some(floor)
            )
            .is_ok()
        );
    }

    #[test]
    fn epoch_proof_input_is_bounded_exact_and_cannot_initialize_an_empty_root() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[85; 32]);
        let new = SigningKey::from_bytes(&[86; 32]);
        initial(root.path(), &old);
        let proof = transition(
            root.path(),
            &old,
            &new,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let path = root.path().join("transition.json");
        let json = serde_json::to_vec(&proof).unwrap();
        fs::write(&path, &json).unwrap();
        assert_eq!(read_authority_transition(&path).unwrap(), proof);
        let mut unknown = serde_json::to_value(&proof).unwrap();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("private_key".into(), serde_json::Value::Null);
        fs::write(&path, serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert!(read_authority_transition(&path).is_err());
        let duplicate = format!(
            "{{\"version\":1,{}",
            std::str::from_utf8(&json[1..]).unwrap()
        );
        fs::write(&path, duplicate).unwrap();
        assert!(read_authority_transition(&path).is_err());
        let mut trailing = json.clone();
        trailing.extend_from_slice(b"{}");
        fs::write(&path, trailing).unwrap();
        assert!(read_authority_transition(&path).is_err());
        fs::write(
            &path,
            vec![b' '; MAX_AUTHORITY_TRANSITION_BYTES as usize + 1],
        )
        .unwrap();
        assert!(read_authority_transition(&path).is_err());
        fs::write(&path, json).unwrap();
        let link = root.path().join("linked-proof.json");
        symlink(&path, &link).unwrap();
        assert!(read_authority_transition(&link).is_err());
        let empty = tempfile::tempdir().unwrap();
        assert!(
            LiveAuthority::rotate_with(empty.path(), &proof, vec![old.verifying_key()], None)
                .is_err()
        );
        assert_eq!(fs::read_dir(empty.path()).unwrap().count(), 0);
    }

    #[test]
    fn epoch_v2_duplicate_records_and_checkpoint_tampering_deny_without_erasing_history() {
        #[derive(Serialize)]
        struct DuplicateRecords {
            version: u16,
            checkpoint: AuthorityTransition,
            records: Vec<(u64, Record)>,
        }
        let root = tempfile::tempdir().unwrap();
        let old = SigningKey::from_bytes(&[87; 32]);
        let new = SigningKey::from_bytes(&[88; 32]);
        initial(root.path(), &old);
        let proof = transition(
            root.path(),
            &old,
            &new,
            0,
            vec![old.verifying_key().to_bytes()],
        );
        let floor =
            LiveAuthority::rotate_with(root.path(), &proof, vec![old.verifying_key()], None)
                .unwrap();
        let state = root.path().join(LIVE_AUTHORITY_FILE);
        let original = fs::read(&state).unwrap();
        let current = cap(&new, 1);
        let grant = Record {
            grant: Some(command(&new, AUTHORITY_GRANT, &current)),
            revoke: None,
        };
        let encoded = DuplicateRecords {
            version: EPOCH_DATABASE_VERSION,
            checkpoint: proof.clone(),
            records: vec![
                (
                    1,
                    Record {
                        grant: grant.grant.clone(),
                        revoke: Some(command(&new, AUTHORITY_REVOKE, &current)),
                    },
                ),
                (1, grant),
            ],
        };
        fs::write(&state, bincode::serialize(&encoded).unwrap()).unwrap();
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor.clone())
            )
            .is_err()
        );
        let mut tampered = proof;
        tampered.retired_public_keys.clear();
        resign(&mut tampered, &old, &new);
        fs::write(
            &state,
            bincode::serialize(&EpochDatabase {
                version: EPOCH_DATABASE_VERSION,
                checkpoint: tampered,
                records: BTreeMap::new(),
            })
            .unwrap(),
        )
        .unwrap();
        assert!(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor.clone())
            )
            .is_err()
        );
        fs::write(&state, original).unwrap();
        let live = Mutex::new(
            LiveAuthority::load_with_floor(
                root.path(),
                false,
                vec![new.verifying_key()],
                Some(floor),
            )
            .unwrap(),
        );
        LiveAuthority::apply_shared(&live, &command(&new, AUTHORITY_GRANT, &current)).unwrap();
        LiveAuthority::acquire(&live, current)
            .unwrap()
            .ensure_healthy()
            .unwrap();
    }
}
