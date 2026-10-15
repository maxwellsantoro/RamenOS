//! Ordinary host Store authority. Signed tokens require an exact durable live
//! grant; revocation permanently retires the ID. This is not DomainManager or
//! protection from a same-UID actor modifying the Store process/files directly.
use crate::capability::{CAPABILITY_VERSION, STORE_RIGHT_ALL, StoreCapability};
use anyhow::{Context, Result, ensure};
use bincode::Options;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::ops::Deref;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AUTHORITY_UPDATE_OPCODE: u8 = 9;
pub const AUTHORITY_VERSION: u16 = 1;
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

pub struct LiveAuthority {
    root: Option<PathBuf>,
    keys: Vec<VerifyingKey>,
    database: Database,
    failed: bool,
    _lock: Option<File>,
    #[cfg(test)]
    fail_after_rename: bool,
}

/// The guard remains live through the handler's actual reads/publication.
/// Management needs the same mutex, so an acknowledged revoke precedes every
/// later handler commit; it cannot invalidate a guard halfway through a commit.
pub struct LiveCapability<'a> {
    capability: StoreCapability,
    _guard: MutexGuard<'a, LiveAuthority>,
}
impl Deref for LiveCapability<'_> {
    type Target = StoreCapability;
    fn deref(&self) -> &Self::Target {
        &self.capability
    }
}

impl LiveAuthority {
    /// `development` is only the existing explicit RAMEN_STORE_DEV_MODE choice,
    /// or an explicit test fixture. It never follows a strict startup failure.
    pub fn open(root: &Path, development: bool) -> Result<Self> {
        Self::load(
            root,
            development,
            crate::capability::load_authority_public_keys(),
        )
    }
    fn load(root: &Path, development: bool, keys: Vec<VerifyingKey>) -> Result<Self> {
        let mut this = Self {
            root: (!development).then(|| root.to_path_buf()),
            keys,
            database: Database {
                version: AUTHORITY_VERSION,
                records: BTreeMap::new(),
            },
            failed: false,
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
            this.database = bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .with_limit(MAX_DATABASE_BYTES)
                .reject_trailing_bytes()
                .deserialize(&bytes)
                .context("invalid live authority database")?;
            ensure!(
                bincode::serialize(&this.database)? == bytes,
                "noncanonical or duplicate authority records"
            );
            ensure!(
                this.database.version == AUTHORITY_VERSION,
                "invalid authority database version"
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

    /// Caller holds the server-local mutex. Persist before exposing a grant or
    /// acknowledging a revoke. Any uncertain IO poisons authority until restart.
    pub fn apply(&mut self, command: &AuthorityCommand) -> Result<()> {
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

    pub fn storage_uncertain(&self) -> bool {
        self.failed
    }

    fn persist(&self, database: &Database) -> Result<()> {
        let root = self
            .root
            .as_ref()
            .context("stateless authority has no persistence")?;
        let path = root.join(LIVE_AUTHORITY_FILE);
        if path_exists(&path)? {
            read_regular(&path, MAX_DATABASE_BYTES)?;
        }
        let bytes = bincode::serialize(database)?;
        ensure!(
            bytes.len() as u64 <= MAX_DATABASE_BYTES,
            "authority database exceeds reserved bound"
        );
        let mut staging = tempfile::NamedTempFile::new_in(root)?;
        staging.write_all(&bytes)?;
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
        let guard = authority
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
        Ok(LiveCapability {
            capability,
            _guard: guard,
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
    fn live_lease_holds_management_lock_through_consumption() {
        let root = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[65; 32]);
        let cap = signed_cap(&key, 1);
        let mut live = LiveAuthority::load(root.path(), false, vec![key.verifying_key()]).unwrap();
        live.apply(&command(&key, AUTHORITY_GRANT, &cap)).unwrap();
        let live = Mutex::new(live);
        let lease = LiveAuthority::acquire(&live, cap.clone()).unwrap();
        assert!(matches!(
            live.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
        assert_eq!(lease.capability_id, cap.capability_id);
        drop(lease);
        live.lock()
            .unwrap()
            .apply(&command(&key, AUTHORITY_REVOKE, &cap))
            .unwrap();
        assert!(LiveAuthority::acquire(&live, cap).is_err());
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
