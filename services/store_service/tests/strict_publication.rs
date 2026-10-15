//! Actual ordinary Store startup and socket clients, with separate capability
//! and artifact-publisher trust. No development signature or access bypass.
use artifact_store_schema::{
    Manifest,
    signature::{ManifestSignature, SignatureAlgorithm, manifest_signing_bytes},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use store_service::{
    StoreClient,
    capability::{STORE_RIGHT_ALL, StoreCapability},
    live_authority::AuthorityCommand,
};

struct StrictStore {
    child: Option<Child>,
    temp: tempfile::TempDir,
    socket: PathBuf,
    root: PathBuf,
    publisher: SigningKey,
    authority: SigningKey,
    control_capability: StoreCapability,
    epoch_floor: Option<String>,
}

impl StrictStore {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let authority = SigningKey::from_bytes(&[62; 32]);
        let mut control_capability = StoreCapability::new(7, STORE_RIGHT_ALL, 4001);
        control_capability.expires_at = control_capability.issued_at + 300;
        control_capability.signature = authority
            .sign(&control_capability.signing_data())
            .to_bytes()
            .to_vec();
        let this = Self {
            child: None,
            socket: temp.path().join("s.sock"),
            root: temp.path().join("cas"),
            temp,
            publisher: SigningKey::from_bytes(&[61; 32]),
            authority,
            control_capability,
            epoch_floor: None,
        };
        std::fs::create_dir(&this.root).unwrap();
        std::fs::write(
            this.temp.path().join("publisher.keys"),
            STANDARD.encode(this.publisher.verifying_key().as_bytes()),
        )
        .unwrap();
        this
    }

    fn start(&mut self) {
        assert!(self.child.is_none());
        self.child = Some(self.spawn_at(&self.socket, "server.log"));
        let end = Instant::now() + Duration::from_secs(5);
        let mut manager = loop {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "strict Store exited: {}",
                self.log()
            );
            assert!(
                Instant::now() < end,
                "strict Store startup timed out: {}",
                self.log()
            );
            // Binding creates the filesystem entry before the listener is
            // necessarily connectable. Readiness retries only connection setup;
            // the signed management mutation below is sent exactly once.
            if let Ok(manager) = StoreClient::connect(&self.socket) {
                break manager;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        // The fixture is an external authority client too; strict startup never
        // fabricates an initial grant. Repeated startup uses the identical token.
        let command = signed_authority_command(
            self,
            1,
            self.control_capability.capability_id,
            bincode::serialize(&self.control_capability).unwrap(),
        );
        manager
            .apply_authority_update(&command)
            .unwrap_or_else(|error| panic!("strict fixture grant failed: {error}; {}", self.log()));
    }

    fn spawn_at(&self, socket: &Path, log_name: &str) -> Child {
        let log = std::fs::File::create(self.temp.path().join(log_name)).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_store_service"));
        command
            .env("RAMEN_STORE_SOCKET", socket)
            .env("RAMEN_STORE_ROOT", &self.root)
            .env(
                "RAMEN_STORE_AUDIT_LOG",
                self.temp.path().join("audit.jsonl"),
            )
            .env(
                "RAMEN_STORE_TRUSTED_KEYS",
                self.temp.path().join("publisher.keys"),
            )
            .env(
                "RAMEN_STORE_CAP_TRUSTED_KEYS",
                STANDARD.encode(self.authority.verifying_key().as_bytes()),
            )
            .env("RAMEN_STORE_REQUEST_TIMEOUT_MS", "2000")
            .env_remove("RAMEN_STORE_DEV_MODE")
            .env_remove("RAMEN_STORE_ACCESS_POLICY")
            .env_remove("RAMEN_STORE_PROJECTION_INDEX")
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log));
        if let Some(floor) = &self.epoch_floor {
            command.env("RAMEN_STORE_AUTHORITY_EPOCH_FLOOR", floor);
        } else {
            command.env_remove("RAMEN_STORE_AUTHORITY_EPOCH_FLOOR");
        }
        command.spawn().unwrap()
    }
    fn assert_start_denies(&self) {
        let mut child = self.spawn_at(&self.temp.path().join("denied.sock"), "denied.log");
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(!status.success(), "invalid authority startup succeeded");
                return;
            }
            if Instant::now() >= end {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("invalid authority startup remained live");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.temp.path().join("server.log")).unwrap_or_default()
    }
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().unwrap().is_none() {
                child.kill().unwrap();
            }
            child.wait().unwrap();
        }
        if self.socket.exists() {
            std::fs::remove_file(&self.socket).unwrap();
        }
    }
    fn capability(&self) -> StoreCapability {
        self.control_capability.clone()
    }
    fn client(&self) -> StoreClient {
        StoreClient::connect_with_capability(&self.socket, 7, Some(self.capability())).unwrap()
    }
    fn manifest(&self, bytes: &[u8]) -> Manifest {
        let mut manifest = Manifest {
            schema_version: 1,
            content_id: artifact_store_core::hash_bytes(bytes),
            size_bytes: bytes.len() as u64,
            kind: "config".into(),
            channels: vec!["test".into()],
            signatures: vec![],
        };
        let signature = ManifestSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            signature_data: STANDARD.encode(
                self.publisher
                    .sign(&manifest_signing_bytes(&manifest).unwrap())
                    .to_bytes(),
            ),
            key_id: format!(
                "sha256:{}",
                hex::encode(self.publisher.verifying_key().as_bytes())
            ),
            timestamp: None,
            signer: None,
        };
        manifest
            .signatures
            .push(serde_json::to_string(&signature).unwrap());
        manifest
    }
    fn seed_control(&self, bytes: &[u8]) -> String {
        let manifest = self.manifest(bytes);
        let id = artifact_store_schema::ContentId::parse(&manifest.content_id).unwrap();
        std::fs::write(self.root.join(format!("{}.blob", id.hash_hex())), bytes).unwrap();
        std::fs::write(
            self.root.join(format!("{}.manifest.json", id.hash_hex())),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        store_service::domain_visibility::DomainArtifactRegistry::new(&self.root)
            .unwrap()
            .register_artifact(&id, 7, false)
            .unwrap();
        manifest.content_id
    }
    fn assert_absent(&self, bytes: &[u8]) {
        let hash = artifact_store_core::hash_bytes(bytes);
        let hex = hash.strip_prefix("sha256:").unwrap();
        assert!(
            !std::fs::read_dir(&self.root).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(hex)),
            "rejected content left publication state"
        );
    }
}
impl Drop for StrictStore {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read(store: &StrictStore, client: &mut StoreClient, content_id: &str) -> Vec<u8> {
    let reply = client
        .get_blob(content_id)
        .unwrap_or_else(|error| panic!("strict read failed: {error}; {}", store.log()));
    std::fs::read(Path::new(&reply.blob_path)).unwrap()
}

#[test]
fn review_strict_unknown_signed_capability_denies_and_preserves_live_control() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing live-authority control");
    store.start();
    let mut legitimate = store.client();
    assert_eq!(
        read(&store, &mut legitimate, &control),
        b"signed existing live-authority control"
    );

    // A trusted issuer's signature proves token provenance, not that this ID
    // has a current grant. No grant for 5001 has been installed in this Store.
    let mut unknown = store.capability();
    unknown.capability_id = 5001;
    unknown.signature = store
        .authority
        .sign(&unknown.signing_data())
        .to_bytes()
        .to_vec();
    assert!(unknown.verify_signature_with_keys(&[store.authority.verifying_key()]));
    assert!(!unknown.is_expired());
    let mut unknown_client =
        StoreClient::connect_with_capability(&store.socket, 7, Some(unknown)).unwrap();
    assert!(
        unknown_client.get_blob(&control).is_err(),
        "strict Store accepted a signed but never-granted capability ID 5001"
    );
    assert_eq!(
        read(&store, &mut legitimate, &control),
        b"signed existing live-authority control"
    );
}

fn signed_authority_command(
    store: &StrictStore,
    action: u8,
    capability_id: u64,
    capability_bytes: Vec<u8>,
) -> AuthorityCommand {
    let mut command = AuthorityCommand {
        version: 1,
        action,
        reserved: 0,
        capability_id,
        capability_bytes,
        signature: vec![],
    };
    command.signature = store
        .authority
        .sign(&command.signing_bytes())
        .to_bytes()
        .to_vec();
    command
}

#[test]
fn review_strict_authenticated_grant_revoke_and_restart_preserve_live_identity() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed durable live-authority control");
    store.start();
    let mut manager = StoreClient::connect(&store.socket).unwrap();
    let mut cap = store.capability();
    cap.capability_id = 6001;
    cap.signature = store
        .authority
        .sign(&cap.signing_data())
        .to_bytes()
        .to_vec();
    let grant = signed_authority_command(
        &store,
        1,
        cap.capability_id,
        bincode::serialize(&cap).unwrap(),
    );
    manager.apply_authority_update(&grant).unwrap();
    let mut holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(cap.clone())).unwrap();
    assert_eq!(
        read(&store, &mut holder, &control),
        b"signed durable live-authority control"
    );

    let revoke = signed_authority_command(&store, 2, cap.capability_id, vec![]);
    manager.apply_authority_update(&revoke).unwrap();
    assert!(
        holder.get_blob(&control).is_err(),
        "acknowledged revoke left holder live"
    );

    let mut fresh = cap.clone();
    fresh.capability_id = 6002;
    fresh.signature = store
        .authority
        .sign(&fresh.signing_data())
        .to_bytes()
        .to_vec();
    let fresh_grant = signed_authority_command(
        &store,
        1,
        fresh.capability_id,
        bincode::serialize(&fresh).unwrap(),
    );
    manager.apply_authority_update(&fresh_grant).unwrap();
    let mut fresh_holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(fresh.clone())).unwrap();
    assert_eq!(
        read(&store, &mut fresh_holder, &control),
        b"signed durable live-authority control"
    );
    drop((manager, holder, fresh_holder));
    store.stop();
    store.start();
    let mut revoked_holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(cap)).unwrap();
    assert!(
        revoked_holder.get_blob(&control).is_err(),
        "restart resurrected revoked capability"
    );
    let mut fresh_holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(fresh)).unwrap();
    assert_eq!(
        read(&store, &mut fresh_holder, &control),
        b"signed durable live-authority control"
    );
}

#[test]
fn review_strict_unsigned_ingest_denies_before_publication_and_preserves_reads() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing control");
    store.start();
    let mut client = store.client();
    assert_eq!(
        read(&store, &mut client, &control),
        b"signed existing control"
    );
    let bytes = b"unsigned ordinary ingest must not claim strict success";
    let source = store.temp.path().join("source");
    std::fs::write(&source, bytes).unwrap();
    assert!(
        client.ingest_artifact("config", "test", &source).is_err(),
        "strict Store accepted unsigned publication"
    );
    store.assert_absent(bytes);
    assert_eq!(
        read(&store, &mut client, &control),
        b"signed existing control"
    );
    assert_recovery(
        &store,
        &mut client,
        &control,
        b"signed success after unsigned denial",
    );
    drop(client);
    store.stop();
    store.start();
    assert_eq!(
        read(&store, &mut store.client(), &control),
        b"signed existing control"
    );
    store.assert_absent(bytes);
}

#[test]
fn review_strict_signed_ingest_preserves_exact_manifest_and_survives_restart() {
    let mut store = StrictStore::new();
    store.start();
    let bytes = b"externally signed descriptor publication";
    let source = store.temp.path().join("signed-source");
    std::fs::write(&source, bytes).unwrap();
    let manifest = store.manifest(bytes);
    let mut client = store.client();
    let reply = client.ingest_signed_artifact(&manifest, &source).unwrap();
    assert_eq!(reply.content_id, manifest.content_id);
    assert_eq!(read(&store, &mut client, &reply.content_id), bytes);
    let stored = client.get_manifest(&reply.content_id).unwrap();
    assert_eq!(stored.schema_version, manifest.schema_version);
    assert_eq!(stored.content_id, manifest.content_id);
    assert_eq!(stored.size_bytes, manifest.size_bytes);
    assert_eq!(stored.kind, manifest.kind);
    assert_eq!(stored.channels, manifest.channels.join(","));
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&stored.signatures).unwrap(),
        manifest.signatures
    );
    drop(client);
    store.stop();
    store.start();
    assert_eq!(read(&store, &mut store.client(), &reply.content_id), bytes);
}

fn resign(manifest: &mut Manifest, key: &SigningKey) {
    manifest.signatures.clear();
    let signature = ManifestSignature {
        algorithm: SignatureAlgorithm::Ed25519,
        signature_data: STANDARD.encode(
            key.sign(&manifest_signing_bytes(manifest).unwrap())
                .to_bytes(),
        ),
        key_id: format!("sha256:{}", hex::encode(key.verifying_key().as_bytes())),
        timestamp: None,
        signer: None,
    };
    manifest
        .signatures
        .push(serde_json::to_string(&signature).unwrap());
}

fn disk_state(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn assert_recovery(store: &StrictStore, client: &mut StoreClient, control: &str, bytes: &[u8]) {
    assert_recovery_with_control(store, client, control, b"signed existing control", bytes);
}

fn assert_recovery_with_control(
    store: &StrictStore,
    client: &mut StoreClient,
    control: &str,
    expected_control: &[u8],
    bytes: &[u8],
) {
    assert_eq!(read(store, client, control), expected_control);
    let source = store.temp.path().join("recovery-source");
    std::fs::write(&source, bytes).unwrap();
    let manifest = store.manifest(bytes);
    client.ingest_signed_artifact(&manifest, &source).unwrap();
    assert_eq!(read(store, client, &manifest.content_id), bytes);
    assert_eq!(read(store, client, control), expected_control);
}

#[test]
fn review_strict_signed_ingest_denials_preserve_state_and_allow_next_publication() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing control");
    store.start();
    let mut client = store.client();
    for case in 0..11 {
        let bytes = format!("rejected source {case}").into_bytes();
        let source = store.temp.path().join("rejected-source");
        std::fs::write(&source, &bytes).unwrap();
        let mut manifest = store.manifest(&bytes);
        match case {
            0 => resign(&mut manifest, &SigningKey::from_bytes(&[63; 32])),
            // Capability issuance is not artifact-publisher authority.
            1 => resign(&mut manifest, &store.authority),
            2 => manifest.kind = "tampered".into(),
            3 => {
                manifest.content_id = artifact_store_core::hash_bytes(b"different bytes");
                resign(&mut manifest, &store.publisher);
            }
            4 => {
                manifest.size_bytes += 1;
                resign(&mut manifest, &store.publisher);
            }
            5 => {
                manifest.schema_version = 2;
                resign(&mut manifest, &store.publisher);
            }
            6 => {
                manifest.kind = "x".repeat(257);
                resign(&mut manifest, &store.publisher);
            }
            7 => {
                manifest.channels = vec!["test".into(); 65];
                resign(&mut manifest, &store.publisher);
            }
            8 => {
                manifest.signatures = vec![manifest.signatures[0].clone(); 33];
            }
            9 => {
                manifest.signatures[0] = "x".repeat(64 * 1024 + 1);
            }
            10 => manifest.signatures.clear(),
            _ => unreachable!(),
        }
        let before = disk_state(&store.root);
        assert!(
            client.ingest_signed_artifact(&manifest, &source).is_err(),
            "invalid signed ingest accepted in case {case}"
        );
        assert_eq!(
            disk_state(&store.root),
            before,
            "denial changed publication in case {case}"
        );
        store.assert_absent(&bytes);
        assert_recovery(
            &store,
            &mut client,
            &control,
            format!("successful after denial {case}").as_bytes(),
        );
    }
    drop(client);
    store.stop();
    store.start();
    assert_eq!(
        read(&store, &mut store.client(), &control),
        b"signed existing control"
    );
}

#[test]
fn review_strict_signed_ingest_exact_dedup_rejects_metadata_conflict() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing control");
    store.start();
    let source = store.temp.path().join("dedup-source");
    let bytes = b"deduplicated signed content";
    std::fs::write(&source, bytes).unwrap();
    let manifest = store.manifest(bytes);
    let mut client = store.client();
    client.ingest_signed_artifact(&manifest, &source).unwrap();
    client.ingest_signed_artifact(&manifest, &source).unwrap();
    let before = disk_state(&store.root);
    let mut conflict = manifest.clone();
    conflict.channels = vec!["other".into()];
    resign(&mut conflict, &store.publisher);
    assert!(client.ingest_signed_artifact(&conflict, &source).is_err());
    assert_eq!(
        disk_state(&store.root),
        before,
        "conflict altered CAS/owner/projection"
    );
    assert_eq!(read(&store, &mut client, &manifest.content_id), bytes);
    assert_recovery(
        &store,
        &mut client,
        &control,
        b"success after dedup conflict",
    );
    drop(client);
    store.stop();
    store.start();
    assert_eq!(
        read(&store, &mut store.client(), &manifest.content_id),
        bytes
    );
}

#[test]
fn review_strict_signed_ingest_rejects_wire_version_trailing_bytes_and_write_denial() {
    use std::os::unix::net::UnixStream;
    use store_service::{
        client::{IngestArtifactReply, SignedIngestArtifactRequest},
        frame,
    };
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing control");
    store.start();
    let source = store.temp.path().join("wire-source");
    let bytes = b"wire-rejected signed source";
    std::fs::write(&source, bytes).unwrap();
    for case in 0..3 {
        let mut capability = store.capability();
        if case == 2 {
            capability.rights_mask = store_service::capability::STORE_RIGHT_READ;
            capability.signature = store
                .authority
                .sign(&capability.signing_data())
                .to_bytes()
                .to_vec();
        }
        let request = SignedIngestArtifactRequest {
            version: if case == 0 { 3 } else { 2 },
            request_id: 100 + case,
            manifest: store.manifest(bytes),
            src_path: source.to_string_lossy().into_owned(),
            capability_bytes: bincode::serialize(&capability).unwrap(),
        };
        let mut payload = vec![8];
        payload.extend(bincode::serialize(&request).unwrap());
        if case == 1 {
            payload.push(0);
        }
        let before = disk_state(&store.root);
        let mut stream = UnixStream::connect(&store.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        frame::write_message(&mut stream, &payload).unwrap();
        // Invalid version/encoding and denied authority must close without
        // waiting for a descriptor, even when the caller supplies none.
        let started = Instant::now();
        match frame::read_message(&mut stream) {
            Ok(reply) => {
                let reply: IngestArtifactReply = bincode::deserialize(&reply).unwrap();
                assert_ne!(reply.status, store_service::status::STATUS_OK);
                assert!(frame::read_message(&mut stream).is_err());
            }
            Err(error) => assert!(!error.to_string().contains("timed out")),
        }
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "rejected request waited for a descriptor"
        );
        assert_eq!(disk_state(&store.root), before);
        store.assert_absent(bytes);
        assert_recovery(
            &store,
            &mut store.client(),
            &control,
            format!("success after wire rejection {case}").as_bytes(),
        );
    }
}

fn cap_with_id(store: &StrictStore, id: u64) -> StoreCapability {
    let mut cap = store.capability();
    cap.capability_id = id;
    cap.signature = store
        .authority
        .sign(&cap.signing_data())
        .to_bytes()
        .to_vec();
    cap
}

#[test]
fn review_strict_offline_epoch_cli_retires_old_authority_and_restarts_with_pinned_fresh_key() {
    use sha2::{Digest, Sha256};
    use store_service::live_authority::{
        AUTHORITY_TRANSITION_VERSION, AuthorityEpochFloor, AuthorityTransition, LIVE_AUTHORITY_FILE,
    };

    fn rotate_cli(store: &StrictStore, proof: &Path) -> (std::process::ExitStatus, String, String) {
        let stdout_path = store.temp.path().join("rotate.stdout");
        let stderr_path = store.temp.path().join("rotate.stderr");
        let mut command = Command::new(env!("CARGO_BIN_EXE_store_authority_rotate"));
        command
            .arg(&store.root)
            .arg(proof)
            .env(
                "RAMEN_STORE_CAP_TRUSTED_KEYS",
                STANDARD.encode(store.authority.verifying_key().as_bytes()),
            )
            .env_remove("RAMEN_STORE_DEV_MODE")
            .stdout(Stdio::from(std::fs::File::create(&stdout_path).unwrap()))
            .stderr(Stdio::from(std::fs::File::create(&stderr_path).unwrap()));
        if let Some(floor) = &store.epoch_floor {
            command.env("RAMEN_STORE_AUTHORITY_EPOCH_FLOOR", floor);
        } else {
            command.env_remove("RAMEN_STORE_AUTHORITY_EPOCH_FLOOR");
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("offline authority CLI did not finish before its deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        for path in [&stdout_path, &stderr_path] {
            assert!(std::fs::metadata(path).unwrap().len() <= 32768);
        }
        (
            status,
            std::fs::read_to_string(stdout_path).unwrap(),
            std::fs::read_to_string(stderr_path).unwrap(),
        )
    }

    let mut store = StrictStore::new();
    let bytes = b"signed artifact survives revocation-all offline authority rollover";
    let control = store.seed_control(bytes);
    store.start();
    let old_token = store.capability();
    let old_grant = signed_authority_command(
        &store,
        1,
        old_token.capability_id,
        bincode::serialize(&old_token).unwrap(),
    );
    let old_revoke = signed_authority_command(&store, 2, old_token.capability_id, vec![]);
    let new_key = SigningKey::from_bytes(&[89; 32]);
    let source = std::fs::read(store.root.join(LIVE_AUTHORITY_FILE)).unwrap();
    let mut transition = AuthorityTransition {
        version: AUTHORITY_TRANSITION_VERSION,
        root_id: Sha256::digest(store.root.as_os_str().as_encoded_bytes()).into(),
        from_epoch: 0,
        from_state_digest: Sha256::digest(&source).into(),
        to_epoch: 1,
        new_public_key: new_key.verifying_key().to_bytes(),
        retired_public_keys: vec![store.authority.verifying_key().to_bytes()],
        old_signature: vec![],
        new_signature: vec![],
    };
    transition.old_signature = store
        .authority
        .sign(&transition.signing_bytes())
        .to_bytes()
        .to_vec();
    transition.new_signature = new_key
        .sign(&transition.signing_bytes())
        .to_bytes()
        .to_vec();
    let proof_path = store.temp.path().join("public-transition.json");
    std::fs::write(&proof_path, serde_json::to_vec(&transition).unwrap()).unwrap();
    let (status, stdout, stderr) = rotate_cli(&store, &proof_path);
    assert!(!status.success(), "live-owner rotation succeeded");
    assert!(
        stdout.is_empty(),
        "rejected rotation printed a success floor"
    );
    assert!(
        stderr.contains("live authority owner"),
        "wrong live-owner denial: {stderr}"
    );
    assert_eq!(
        std::fs::read(store.root.join(LIVE_AUTHORITY_FILE)).unwrap(),
        source
    );
    assert_eq!(read(&store, &mut store.client(), &control), bytes);
    store.stop();

    let (status, stdout, stderr) = rotate_cli(&store, &proof_path);
    assert!(status.success(), "offline rotation failed: {stderr}");
    assert!(stderr.is_empty());
    let floor = AuthorityEpochFloor::parse(stdout.trim_end_matches('\n')).unwrap();
    assert_eq!(stdout, format!("{}\n", floor.encode()));
    assert_eq!(floor.root_id, transition.root_id);
    assert_eq!(floor.epoch, 1);
    assert_eq!(
        floor.checkpoint_digest,
        transition.checkpoint_digest().unwrap()
    );
    store.assert_start_denies();
    store.epoch_floor = Some(floor.encode());
    store.assert_start_denies();

    // Only the external fixture owns signing keys. The CLI receives public
    // proof JSON and public source trust; it receives no private-key arguments.
    store.authority = new_key;
    store.control_capability.signature = store
        .authority
        .sign(&store.control_capability.signing_data())
        .to_bytes()
        .to_vec();
    assert_eq!(
        store.control_capability.capability_id,
        old_token.capability_id
    );
    store.epoch_floor = None;
    store.assert_start_denies();
    store.epoch_floor = Some(floor.encode());
    store.start();
    assert_eq!(read(&store, &mut store.client(), &control), bytes);
    let mut old_holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(old_token)).unwrap();
    assert!(old_holder.get_blob(&control).is_err());
    let mut manager = StoreClient::connect(&store.socket).unwrap();
    assert!(manager.apply_authority_update(&old_grant).is_err());
    assert!(manager.apply_authority_update(&old_revoke).is_err());
    assert_eq!(read(&store, &mut store.client(), &control), bytes);
    store.stop();
    store.start();
    assert_eq!(read(&store, &mut store.client(), &control), bytes);
    assert_recovery_with_control(
        &store,
        &mut store.client(),
        &control,
        bytes,
        b"signed publication with fresh authority after pinned epoch restart",
    );
}

#[test]
fn review_strict_unrelated_revoke_completes_while_admitted_get_blob_io_is_held() {
    use std::{
        ffi::CString,
        io::Write,
        os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        sync::mpsc,
    };

    let mut store = StrictStore::new();
    let bytes = b"held authenticated GetBlob control";
    let control = store.seed_control(bytes);
    store.start();
    let other = cap_with_id(&store, 6501);
    let mut manager = StoreClient::connect(&store.socket).unwrap();
    manager
        .apply_authority_update(&signed_authority_command(
            &store,
            1,
            other.capability_id,
            bincode::serialize(&other).unwrap(),
        ))
        .unwrap();

    let id = artifact_store_schema::ContentId::parse(&control).unwrap();
    let manifest_path = store.root.join(format!("{}.manifest.json", id.hash_hex()));
    let manifest = std::fs::read(&manifest_path).unwrap();
    std::fs::remove_file(&manifest_path).unwrap();
    let fifo_path = CString::new(manifest_path.as_os_str().as_bytes()).unwrap();
    // SAFETY: fifo_path is a live NUL-terminated pathname, and this fixture owns
    // the private Store root. The ordinary service receives no test-only hook.
    assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);

    let mut holder = store.client();
    let read_id = control.clone();
    let reader = std::thread::spawn(move || holder.get_blob(&read_id));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut writer = loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&manifest_path)
        {
            Ok(writer) => break writer,
            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {
                assert!(
                    Instant::now() < deadline,
                    "GetBlob never entered manifest IO"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("FIFO writer failed: {error}"),
        }
    };
    // A writer can open only after the server has entered the FIFO read. With
    // no bytes and this writer still open, that admitted GetBlob cannot finish.
    let revoke = signed_authority_command(&store, 2, other.capability_id, vec![]);
    let (completed_tx, completed_rx) = mpsc::channel();
    let revoker = std::thread::spawn(move || {
        completed_tx
            .send(manager.apply_authority_update(&revoke))
            .unwrap();
    });
    let completed_while_held = completed_rx.recv_timeout(Duration::from_secs(2));
    // Always release the real IO before checking the ordering assertion, so the
    // negative control also cleans up its admitted request and management call.
    writer.write_all(&manifest).unwrap();
    drop(writer);
    let reply = reader.join().unwrap().unwrap();
    assert_eq!(std::fs::read(&reply.blob_path).unwrap(), bytes);
    revoker.join().unwrap();
    let outcome = completed_while_held.expect("unrelated revoke waited for held GetBlob IO");
    outcome.unwrap();
    std::fs::remove_file(&manifest_path).unwrap();
    std::fs::write(&manifest_path, manifest).unwrap();

    let mut revoked = StoreClient::connect_with_capability(&store.socket, 7, Some(other)).unwrap();
    assert!(revoked.get_blob(&control).is_err());
    assert_recovery_with_control(
        &store,
        &mut store.client(),
        &control,
        bytes,
        b"successful signed publication after unrelated revoke and released IO",
    );
}

#[test]
fn review_strict_admitted_get_blob_denies_success_after_unrelated_authority_poison() {
    use std::{
        ffi::CString,
        io::Write,
        os::unix::{ffi::OsStrExt, fs::OpenOptionsExt, fs::symlink},
        sync::mpsc,
    };
    use store_service::live_authority::LIVE_AUTHORITY_FILE;

    let mut store = StrictStore::new();
    let bytes = b"admitted read must not expose success after authority poison";
    let control = store.seed_control(bytes);
    store.start();
    let id = artifact_store_schema::ContentId::parse(&control).unwrap();
    let manifest_path = store.root.join(format!("{}.manifest.json", id.hash_hex()));
    let manifest = std::fs::read(&manifest_path).unwrap();
    std::fs::remove_file(&manifest_path).unwrap();
    let fifo_path = CString::new(manifest_path.as_os_str().as_bytes()).unwrap();
    // SAFETY: the private fixture owns this live NUL-terminated pathname.
    assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
    let mut holder = store.client();
    let read_id = control.clone();
    let reader = std::thread::spawn(move || holder.get_blob(&read_id));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut writer = loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&manifest_path)
        {
            Ok(writer) => break writer,
            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {
                assert!(
                    Instant::now() < deadline,
                    "GetBlob never entered manifest IO"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("FIFO writer failed: {error}"),
        }
    };

    // Induce a genuine rejected persistence path after admission. This fixture
    // intentionally controls the same-UID files; it makes no isolation claim.
    let state_path = store.root.join(LIVE_AUTHORITY_FILE);
    let durable_state = std::fs::read(&state_path).unwrap();
    let retained_state = store.temp.path().join("retained-authority-state");
    std::fs::write(&retained_state, &durable_state).unwrap();
    std::fs::remove_file(&state_path).unwrap();
    symlink(&retained_state, &state_path).unwrap();
    let revoke = signed_authority_command(&store, 2, 6502, vec![]);
    let mut manager = StoreClient::connect(&store.socket).unwrap();
    let (completed_tx, completed_rx) = mpsc::channel();
    let update = std::thread::spawn(move || {
        completed_tx
            .send(manager.apply_authority_update(&revoke))
            .unwrap();
    });
    let poisoned_while_held = completed_rx.recv_timeout(Duration::from_secs(2));
    writer.write_all(&manifest).unwrap();
    drop(writer);
    let read_result = reader.join().unwrap();
    update.join().unwrap();
    assert!(
        poisoned_while_held
            .expect("authority failure waited for held read")
            .is_err()
    );
    assert!(
        read_result.is_err(),
        "admitted GetBlob returned success after authority poison"
    );
    std::fs::remove_file(&manifest_path).unwrap();
    std::fs::write(&manifest_path, manifest).unwrap();
    assert!(store.client().get_blob(&control).is_err());

    // The verified original database is restored only after the failed owner
    // stops. No revocation is claimed for the rejected pre-rename mutation.
    store.stop();
    std::fs::remove_file(&state_path).unwrap();
    std::fs::write(&state_path, durable_state).unwrap();
    store.start();
    assert_recovery_with_control(
        &store,
        &mut store.client(),
        &control,
        bytes,
        b"successful publication after verified authority restart",
    );
}

#[test]
fn review_strict_revocation_denies_every_ordinary_operation_and_allows_fresh_publication() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing control");
    store.start();
    let mut manager = StoreClient::connect(&store.socket).unwrap();
    let cap = cap_with_id(&store, 6101);
    let grant = signed_authority_command(
        &store,
        1,
        cap.capability_id,
        bincode::serialize(&cap).unwrap(),
    );
    manager.apply_authority_update(&grant).unwrap();
    manager.apply_authority_update(&grant).unwrap();
    let mut holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(cap.clone())).unwrap();
    let accepted = b"signed ingest with authenticated live grant";
    let source = store.temp.path().join("live-source");
    std::fs::write(&source, accepted).unwrap();
    let manifest = store.manifest(accepted);
    holder.ingest_signed_artifact(&manifest, &source).unwrap();
    assert_eq!(read(&store, &mut holder, &manifest.content_id), accepted);
    let revoke = signed_authority_command(&store, 2, cap.capability_id, vec![]);
    manager.apply_authority_update(&revoke).unwrap();
    manager.apply_authority_update(&revoke).unwrap();
    assert!(holder.get_manifest(&control).is_err());
    assert!(holder.get_blob(&control).is_err());
    assert!(holder.verify_artifact(&control).is_err());
    assert!(
        holder
            .query_projection_by_path(source.to_str().unwrap())
            .is_err()
    );
    assert!(holder.query_projection_by_tag("test").is_err());
    let rejected = b"revoked signed ingest must leave no publication";
    std::fs::write(&source, rejected).unwrap();
    assert!(
        holder
            .ingest_signed_artifact(&store.manifest(rejected), &source)
            .is_err()
    );
    assert!(holder.ingest_artifact("config", "test", &source).is_err());
    store.assert_absent(rejected);
    assert!(
        manager.apply_authority_update(&grant).is_err(),
        "revoked ID regranted"
    );
    let fresh = cap_with_id(&store, 6102);
    manager
        .apply_authority_update(&signed_authority_command(
            &store,
            1,
            fresh.capability_id,
            bincode::serialize(&fresh).unwrap(),
        ))
        .unwrap();
    let mut fresh_holder =
        StoreClient::connect_with_capability(&store.socket, 7, Some(fresh)).unwrap();
    assert_recovery(
        &store,
        &mut fresh_holder,
        &control,
        b"fresh publication after complete revocation",
    );
}

#[test]
fn review_strict_management_rejects_wrong_roles_tampering_aliases_stale_and_tombstone_replay() {
    let mut store = StrictStore::new();
    let control = store.seed_control(b"management denial control");
    store.start();
    let mut manager = StoreClient::connect(&store.socket).unwrap();
    let cap = cap_with_id(&store, 6201);
    let grant = signed_authority_command(
        &store,
        1,
        cap.capability_id,
        bincode::serialize(&cap).unwrap(),
    );
    let mut bad = grant.clone();
    bad.signature = store
        .publisher
        .sign(&bad.signing_bytes())
        .to_bytes()
        .to_vec();
    assert!(
        manager.apply_authority_update(&bad).is_err(),
        "publisher managed authority"
    );
    bad = grant.clone();
    bad.capability_id += 1;
    assert!(
        manager.apply_authority_update(&bad).is_err(),
        "tampered command accepted"
    );
    for (version, action, reserved) in [(2, 1, 0), (1, 3, 0), (1, 1, 1)] {
        let mut bad = grant.clone();
        bad.version = version;
        bad.action = action;
        bad.reserved = reserved;
        bad.signature = store
            .authority
            .sign(&bad.signing_bytes())
            .to_bytes()
            .to_vec();
        assert!(manager.apply_authority_update(&bad).is_err());
    }
    let mut stale = cap_with_id(&store, 6202);
    stale.issued_at = 1;
    stale.expires_at = 2;
    stale.signature = store
        .authority
        .sign(&stale.signing_data())
        .to_bytes()
        .to_vec();
    assert!(
        manager
            .apply_authority_update(&signed_authority_command(
                &store,
                1,
                stale.capability_id,
                bincode::serialize(&stale).unwrap()
            ))
            .is_err()
    );
    manager.apply_authority_update(&grant).unwrap();
    for field in 0..5 {
        let mut alias = cap.clone();
        match field {
            0 => alias.domain_id += 1,
            1 => alias.rights_mask = store_service::capability::STORE_RIGHT_READ,
            2 => alias.issued_at -= 1,
            3 => alias.expires_at += 1,
            _ => alias.version += 1,
        }
        alias.signature = store
            .authority
            .sign(&alias.signing_data())
            .to_bytes()
            .to_vec();
        assert!(
            manager
                .apply_authority_update(&signed_authority_command(
                    &store,
                    1,
                    alias.capability_id,
                    bincode::serialize(&alias).unwrap()
                ))
                .is_err(),
            "ID alias replaced original grant"
        );
        let mut holder =
            StoreClient::connect_with_capability(&store.socket, alias.domain_id, Some(alias))
                .unwrap();
        assert!(
            holder.get_blob(&control).is_err(),
            "ID alias used original grant"
        );
    }
    let tombstone = signed_authority_command(&store, 2, 6203, vec![]);
    manager.apply_authority_update(&tombstone).unwrap();
    manager.apply_authority_update(&tombstone).unwrap();
    let tombstoned = cap_with_id(&store, 6203);
    assert!(
        manager
            .apply_authority_update(&signed_authority_command(
                &store,
                1,
                6203,
                bincode::serialize(&tombstoned).unwrap()
            ))
            .is_err()
    );
    let mut holder = StoreClient::connect_with_capability(&store.socket, 7, Some(cap)).unwrap();
    assert_eq!(
        read(&store, &mut holder, &control),
        b"management denial control"
    );
}

#[test]
fn review_strict_live_authority_rejects_second_owner_and_corrupt_missing_symlink_state() {
    use store_service::live_authority::{LIVE_AUTHORITY_FILE, LIVE_AUTHORITY_MARKER};
    for case in 0..5 {
        let mut store = StrictStore::new();
        let control = store.seed_control(b"durability recovery control");
        store.start();
        // A separate actual process cannot retain an independently stale view.
        store.assert_start_denies();
        assert_eq!(
            read(&store, &mut store.client(), &control),
            b"durability recovery control"
        );
        store.stop();
        let state = store.root.join(LIVE_AUTHORITY_FILE);
        let marker = store.root.join(LIVE_AUTHORITY_MARKER);
        let saved = std::fs::read(&state).unwrap();
        let saved_marker = std::fs::read(&marker).unwrap();
        match case {
            0 => std::fs::write(&state, &saved[..saved.len() / 2]).unwrap(),
            1 => std::fs::remove_file(&state).unwrap(),
            2 => {
                std::fs::remove_file(&state).unwrap();
                let target = store.temp.path().join("foreign-state");
                std::fs::write(&target, &saved).unwrap();
                std::os::unix::fs::symlink(target, &state).unwrap();
            }
            3 => std::fs::remove_file(&marker).unwrap(),
            _ => std::fs::write(&marker, b"corrupt marker").unwrap(),
        }
        store.assert_start_denies();
        for path in [&state, &marker] {
            if std::fs::symlink_metadata(path).is_ok() {
                std::fs::remove_file(path).unwrap();
            }
        }
        std::fs::write(&state, saved).unwrap();
        std::fs::write(&marker, saved_marker).unwrap();
        store.start();
        assert_eq!(
            read(&store, &mut store.client(), &control),
            b"durability recovery control"
        );
    }
}

#[test]
fn review_strict_management_wire_rejects_malformed_trailing_and_oversized_requests() {
    use std::os::unix::net::UnixStream;
    use store_service::{
        frame,
        live_authority::{AUTHORITY_UPDATE_OPCODE, AuthorityUpdateRequest},
    };
    let mut store = StrictStore::new();
    let control = store.seed_control(b"management wire recovery control");
    store.start();
    let cap = cap_with_id(&store, 6301);
    let request = AuthorityUpdateRequest {
        request_id: 991,
        command: signed_authority_command(&store, 1, 6301, bincode::serialize(&cap).unwrap()),
    };
    let valid = bincode::serialize(&request).unwrap();
    let mut trailing = valid.clone();
    trailing.push(0);
    for payload in [
        vec![],
        valid[..valid.len() / 2].to_vec(),
        trailing,
        vec![0; 1025],
    ] {
        let mut stream = UnixStream::connect(&store.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut msg = vec![AUTHORITY_UPDATE_OPCODE];
        msg.extend_from_slice(&payload);
        frame::write_message(&mut stream, &msg).unwrap();
        assert!(
            frame::read_message(&mut stream).is_err(),
            "malformed authority request accepted"
        );
        assert_eq!(
            read(&store, &mut store.client(), &control),
            b"management wire recovery control"
        );
    }
    let mut unknown =
        StoreClient::connect_with_capability(&store.socket, 7, Some(cap.clone())).unwrap();
    assert!(
        unknown.get_blob(&control).is_err(),
        "rejected wire installed a grant"
    );
    StoreClient::connect(&store.socket)
        .unwrap()
        .apply_authority_update(&request.command)
        .unwrap();
    assert_eq!(
        read(&store, &mut unknown, &control),
        b"management wire recovery control"
    );
}

#[test]
fn review_strict_rejected_second_startup_preserves_active_publication_intent() {
    use store_service::domain_visibility::DomainArtifactRegistry;
    let mut store = StrictStore::new();
    let control = store.seed_control(b"signed existing control");
    store.start();
    assert_eq!(
        read(&store, &mut store.client(), &control),
        b"signed existing control"
    );

    // The first strict process already owns the Store root. Exercise the public
    // publisher at its real durable-intent boundary, before the blob exists.
    let bytes = b"active publication must survive rejected second Store startup";
    let manifest = store.manifest(bytes);
    let id = artifact_store_schema::ContentId::parse(&manifest.content_id).unwrap();
    let intent = store
        .root
        .join(format!("{}.publication.json", id.hash_hex()));
    let mut owner = DomainArtifactRegistry::new(&store.root).unwrap();
    owner
        .publish_owned_exact(&store.root, &manifest, 7, false, |blob| {
            let held_intent =
                std::fs::read(&intent).expect("publisher did not establish durable intent");
            assert!(
                !blob.exists(),
                "publication pause occurred after blob creation"
            );
            store.assert_start_denies();
            let denied_log = std::fs::read_to_string(store.temp.path().join("denied.log")).unwrap();
            assert!(
                denied_log.contains("live authority owner"),
                "second startup did not reject root ownership: {denied_log}"
            );
            assert_eq!(
                std::fs::read(&intent)
                    .expect("rejected second startup removed the first publisher's active intent"),
                held_intent,
                "rejected second startup changed the first publisher's active intent"
            );
            std::fs::write(blob, bytes)
        })
        .unwrap();
    assert!(
        owner.can_access(&id, 7),
        "successful publication lost its owning registry visibility"
    );
    assert!(!owner.can_access(&id, 8));
    assert_eq!(owner.get_owner(&id).unwrap().domain_id, 7);
    assert_eq!(
        std::fs::read(store.root.join(format!("{}.blob", id.hash_hex()))).unwrap(),
        bytes
    );
    assert!(
        !intent.exists(),
        "completed publication left its intent behind"
    );
    assert_eq!(
        read(&store, &mut store.client(), &control),
        b"signed existing control"
    );
    drop(owner);
    store.stop();
    store.start();
    assert_eq!(
        read(&store, &mut store.client(), &manifest.content_id),
        bytes
    );
    assert_eq!(
        read(&store, &mut store.client(), &control),
        b"signed existing control"
    );
}
