//! Assertion fixtures only. UIa approval remains a separately scoped volatile proof.
//! Opaque native authority is obtained only from the actual new/entry API.
#[allow(dead_code)]
#[path = "../../../desktop/tests/support/editor_host_assertions.rs"]
pub mod ui_a;

use artifact_store_schema::editor_save::{EditorTextHeaderV0, TEXT_HEADER_LEN};
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::desktop_artifact_v1 as artifact;
use kernel_api::ipc::Envelope;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use store_service::editor_store::{self as store, StoreStatus};

pub const A: &[u8] = b"note=old\n";
pub const B: &[u8] = b"other=live\n";

pub fn success<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("expected actual successful API result"),
    }
}
pub fn denied<T, E: std::fmt::Debug + PartialEq>(result: Result<T, E>, status: E) {
    match result {
        Err(actual) => assert_eq!(actual, status),
        Ok(_) => panic!("denial returned a value"),
    }
}
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct Rig {
    pub h: desktop::HostDesktop,
    pub c: desktop::FixtureController,
    pub witness: Option<desktop::RegistryWitness>,
    pub drivers: Vec<ui_a::Driver>,
    pub live: Vec<ui_a::Live>,
    pub views: Vec<desktop::ReadBindingView>,
    pub pauses: Vec<desktop::NativeReadPause>,
    released: Vec<Cell<bool>>,
}
impl Rig {
    pub fn new(count: usize) -> Self {
        assert!((1..=2).contains(&count));
        let (h, c, witness) = success(desktop::HostDesktop::new_native_read(ui_a::config(
            30_000, 600_000,
        )));
        let mut this = Self {
            h,
            c,
            witness: Some(witness),
            drivers: vec![],
            live: vec![],
            views: vec![],
            pauses: vec![],
            released: vec![],
        };
        for i in 0..count {
            this.add_session(i as u64 + 1, if i == 0 { A } else { B });
        }
        this
    }
    pub fn add_session(&mut self, owner: u64, bytes: &[u8]) {
        let mut driver = ui_a::Driver::register(&self.h, &self.c, owner, bytes, 0);
        let live = driver.launch(
            &self.h,
            &self.c,
            if self.drivers.is_empty() { 1 } else { 100 },
        );
        self.drivers.push(driver);
        self.live.push(live);
    }
    pub fn binding(&mut self, i: usize) -> desktop::ApprovedReadBinding {
        let binding = success(
            self.h
                .approved_native_read(&self.live[i].bindings.artifact.peer, 1),
        );
        let witness = self.witness.as_ref().expect("not yet transferred to Store");
        assert!(witness.matches_binding(&binding));
        let view = success(binding.admit(witness)).view();
        let boot = &self.live[i].boot;
        assert_eq!(
            (
                view.actor.session_id,
                view.actor.session_generation,
                view.actor.instance_id,
                view.actor.instance_generation
            ),
            (
                boot.session_id,
                boot.session_generation,
                boot.instance_id,
                boot.instance_generation
            )
        );
        assert_eq!(view.actor.owner_id, i as u64 + 1);
        assert_eq!(view.selected_hash, digest(if i == 0 { A } else { B }));
        self.views.push(view);
        binding
    }
    pub fn request(&self, i: usize, id: u64) -> Envelope {
        let boot = &self.live[i].boot;
        canonical(success(
            artifact::ReadSelected {
                request_id: id,
                session_id: boot.session_id,
                session_generation: boot.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
            }
            .encode(self.live[i].bindings.artifact.handle),
        ))
    }
    pub fn entry(&self, i: usize, id: u64) -> desktop::NativeReadEntry {
        success(self.h.start_native_read(
            &self.live[i].bindings.artifact.peer,
            &self.request(i, id),
            1,
        ))
    }
    pub fn pause(&mut self, i: usize) -> usize {
        let token = success(self.c.pause_next_native_read(self.drivers[i].s.session_id));
        assert!(self.pauses.len() < 32);
        self.pauses.push(token);
        self.released.push(Cell::new(false));
        self.pauses.len() - 1
    }
    pub fn release(&self, index: usize) {
        assert!(!self.released[index].get());
        success(self.c.release_native_read(&self.pauses[index]));
        self.released[index].set(true);
    }
    pub fn release_all(&self) {
        for i in 0..self.pauses.len() {
            if !self.released[i].get() {
                self.release(i);
            }
        }
    }
    pub fn join_all(&self) -> Vec<Value> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut proofs = vec![];
        let mut unique = BTreeSet::new();
        loop {
            let ids = self.h.native_producers();
            if ids.is_empty() {
                break;
            }
            for id in ids {
                match self.h.join_native_finished(&id) {
                    Ok(proof) => {
                        assert_eq!(proof.outcome(), desktop::JoinOutcome::Returned);
                        assert!(unique.insert(proof.producer_id()));
                        proofs.push(json!({"id":proof.producer_id(),"outcome":1}));
                    }
                    Err(Status::NotReady) => {}
                    Err(e) => panic!("actual owned join failed: {e:?}"),
                }
            }
            assert!(
                Instant::now() < deadline,
                "supported fixture workers did not settle/join"
            );
            std::thread::yield_now();
        }
        let counts = self.h.native_producer_counts();
        assert_eq!((counts.held, counts.io, counts.joining), (0, 0, 0));
        proofs
    }
}
// Release only this rig's owned deterministic pauses on an assertion failure.
// No forced arbitrary-thread cleanup or host process scan is claimed.
impl Drop for Rig {
    fn drop(&mut self) {
        for token in &self.pauses {
            let _ = self.c.release_native_read(token);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.h.native_producer_counts().held != 0 && Instant::now() < deadline {
            for id in self.h.native_producers() {
                let _ = self.h.join_native_finished(&id);
            }
            std::thread::yield_now();
        }
        if self.h.native_producer_counts().held != 0 {
            // Keep the real owner/charged handles on failure; this is not a
            // claimed cleanup of unsupported permanently hung fixture work.
            std::mem::forget(self.h.clone());
        }
    }
}

pub fn canonical(request: Envelope) -> Envelope {
    let wire = success(desktop::encode_envelope_wire(&request));
    let decoded = success(desktop::decode_envelope_wire(&wire));
    assert_eq!(success(desktop::encode_envelope_wire(&decoded)), wire);
    decoded
}
pub fn temporary() -> tempfile::TempDir {
    #[cfg(target_os = "linux")]
    {
        success(
            tempfile::Builder::new()
                .prefix("ramenos-native-read-")
                .tempdir(),
        )
    }
    #[cfg(target_os = "macos")]
    {
        let parent = Path::new("/System/Volumes/Data/private/tmp");
        let metadata = success(fs::symlink_metadata(parent));
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        assert_eq!(
            (metadata.uid(), metadata.gid(), metadata.mode() & 0o7777),
            (0, 0, 0o1777)
        );
        success(
            tempfile::Builder::new()
                .prefix("ramenos-native-read-")
                .tempdir_in(parent),
        )
    }
}
pub fn profile(views: &[desktop::ReadBindingView]) -> store::FixtureProfile {
    store::FixtureProfile {
        origin_ms: 1,
        initial_objects: views
            .iter()
            .enumerate()
            .map(|(i, v)| store::InitialObject {
                owner_id: v.actor.owner_id,
                object_id: v.object_id,
                generation: v.object_generation,
                revision: v.selected_revision,
                bytes: if i == 0 { A.to_vec() } else { B.to_vec() },
            })
            .collect(),
        fixture_signing_seed: [0x53; 32],
        fixture_key_id: "native-read-fixture-v0".into(),
    }
}
pub fn files(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>, total: &mut usize) {
        for row in success(fs::read_dir(at)) {
            let row = success(row);
            let path = row.path();
            let meta = success(fs::symlink_metadata(&path));
            assert!(!meta.file_type().is_symlink());
            if meta.is_dir() {
                walk(base, &path, out, total);
            } else {
                assert!(meta.is_file());
                assert!(out.len() < 512 && meta.len() <= 131_072);
                *total = total.checked_add(meta.len() as usize).unwrap();
                assert!(*total <= 2 * 1024 * 1024);
                let bytes = success(fs::read(&path));
                assert_eq!(bytes.len() as u64, meta.len());
                assert!(
                    out.insert(success(path.strip_prefix(base)).to_path_buf(), bytes)
                        .is_none()
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    let mut total = 0;
    walk(path, path, &mut out, &mut total);
    out
}

pub struct Fixture {
    pub rig: Rig,
    pub native: Option<store::NativeReadFixture>,
    pub root: Option<tempfile::TempDir>,
    pub store_path: PathBuf,
    pub initial: BTreeMap<PathBuf, Vec<u8>>,
    pub store_pauses: Vec<store::PauseToken>,
    store_released: Vec<Cell<bool>>,
    pub observations: Vec<Value>,
    pub case: &'static str,
}
impl Fixture {
    pub fn new(case: &'static str, count: usize) -> Self {
        let mut rig = Rig::new(count);
        let bindings = (0..count).map(|i| rig.binding(i)).collect();
        let root = temporary();
        let store_path = root.path().join("owner");
        let native = success(store::StoreFixture::create_native_read(
            &store_path,
            profile(&rig.views),
            rig.witness.take().unwrap(),
            bindings,
        ));
        let initial = files(&store_path);
        Self {
            rig,
            native: Some(native),
            root: Some(root),
            store_path,
            initial,
            store_pauses: vec![],
            store_released: vec![],
            observations: vec![],
            case,
        }
    }
    pub fn native(&self) -> &store::NativeReadFixture {
        self.native.as_ref().unwrap()
    }
    pub fn host(&self) -> store::StoreHost {
        self.native().host()
    }
    pub fn endpoint(&self, i: usize) -> &store::EditorEndpoint {
        success(self.native().endpoint(i))
    }
    pub fn register(&self, i: usize, entry: &desktop::NativeReadEntry) -> store::RegisteredRead {
        let call = success(entry.wait());
        assert_eq!(
            call.deadline().duration_since(call.entered()),
            Duration::from_millis(1000)
        );
        denied(entry.wait(), Status::NotReady);
        success(
            self.host()
                .register_native_read(&self.endpoint(i).peer, call),
        )
    }
    pub fn positive(&mut self, i: usize, id: u64) {
        let entry = self.rig.entry(i, id);
        let registered = self.register(i, &entry);
        let reply = success(self.host().execute_native_read(&registered));
        let bytes = checked_selected(&reply, &self.rig.views[i], if i == 0 { A } else { B }, id);
        self.observations.push(json!({"kind":"read", "request":hex(&success(desktop::encode_envelope_wire(&self.rig.request(i,id)))),
            "reply":hex(&success(desktop::encode_envelope_wire(&reply.wire()))), "selected":hex(&bytes),
            "store_producers":reply.producer_ids().len()}));
        assert_eq!(
            reply.producer_ids().len(),
            2,
            "actual successful Store dispatcher/supervisor IDs"
        );
        assert_unchanged(self);
    }
    pub fn pause_store(&mut self, i: usize) -> usize {
        let token = success(
            self.native()
                .controller()
                .pause_next(self.endpoint(i), store::PausePoint::BeforeReadReply),
        );
        assert!(self.store_pauses.len() < 32);
        self.store_pauses.push(token);
        self.store_released.push(Cell::new(false));
        self.store_pauses.len() - 1
    }
    pub fn release_store(&self, index: usize) {
        assert!(!self.store_released[index].get());
        success(
            self.native()
                .controller()
                .release(&self.store_pauses[index]),
        );
        self.store_released[index].set(true);
    }
    pub fn finish(mut self) {
        for i in 0..self.store_pauses.len() {
            if !self.store_released[i].get() {
                self.release_store(i);
            }
        }
        self.rig.release_all();
        let joins = self.rig.join_all();
        assert_eq!(self.host().native_read_producer_counts().held, 0);
        assert_unchanged(&self);
        let root = self.root.take().unwrap();
        let path = root.path().to_path_buf();
        let identity = success(fs::symlink_metadata(&path));
        self.native.take();
        // Consume the real Desktop owner/client fixtures before TempDir.close.
        let case = self.case;
        let mut observations = std::mem::take(&mut self.observations);
        observations.push(json!({"kind":"joins","actual":joins,"held":0,"io":0}));
        drop(self);
        success(root.close());
        let absence = fs::symlink_metadata(&path).expect_err("owned root survives explicit close");
        assert_eq!(absence.kind(), std::io::ErrorKind::NotFound);
        if let Some(parent) = std::env::var_os("RAMEN_DESKTOP_EDITOR_NATIVE_READ_GATE_EVIDENCE") {
            let root_name = path.file_name().unwrap().to_str().unwrap();
            assert!(
                root_name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            );
            let target = PathBuf::from(parent).join(format!(
                "{case}-{}-{}-{root_name}",
                identity.dev(),
                identity.ino()
            ));
            success(fs::create_dir(&target));
            let value = json!({"schema_version":1,"case":case,"scope":"native_read_host_fixture",
                "observations":observations,"cleanup":{"root":path,"device":identity.dev(),"inode":identity.ino(),"explicit_close":true,"actual_not_found":true},
                "claims":{"full_ui1_1c":false,"save":false,"device":false,"containment":false}});
            let bytes = success(serde_json::to_vec(&value));
            assert!(bytes.len() <= 1024 * 1024);
            let mut output = success(
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target.join("observations.json")),
            );
            success(std::io::Write::write_all(&mut output, &bytes));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(native) = &self.native {
            for token in &self.store_pauses {
                let _ = native.controller().release(token);
            }
        }
        for token in &self.rig.pauses {
            let _ = self.rig.c.release_native_read(token);
        }
        // On an assertion failure release supported fixtures and join only held
        // native handles. If settlement is uncertain, retain the private root;
        // do not report cleanup or remove a root beneath unquiesced work.
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.rig.h.native_producer_counts().held != 0 && Instant::now() < deadline {
            for id in self.rig.h.native_producers() {
                let _ = self.rig.h.join_native_finished(&id);
            }
            std::thread::yield_now();
        }
        if self.rig.h.native_producer_counts().held != 0 {
            if let Some(root) = self.root.take() {
                std::mem::forget(root);
            }
        }
    }
}

pub fn assert_unchanged(f: &Fixture) {
    assert_eq!(files(&f.store_path), f.initial);
    let evidence = success(f.native().controller().evidence());
    for object in evidence.objects {
        assert!(object.operations.is_empty());
        assert_eq!(
            (
                object.mutation_dispatch_count,
                object.permit_count,
                object.transition_count
            ),
            (0, 0, 0)
        );
    }
}
pub fn zero_reply(reply: &store::NativeReadReply, id: u64, status: StoreStatus) {
    assert!(reply.selected().is_none());
    let wire = success(desktop::encode_envelope_wire(&reply.wire()));
    let typed = success(artifact::ReadSelectedReply::decode(&success(
        desktop::decode_envelope_wire(&wire),
    )));
    assert_eq!((typed.request_id, typed.status), (id, status as u32));
    assert_eq!(
        (
            typed.revision,
            typed.data_shm,
            typed.object_generation,
            typed.byte_len
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(reply.wire().protocol, artifact::ReadSelectedReply::PROTOCOL);
    assert_eq!(reply.wire().msg_type, 2);
    assert_eq!(reply.wire().payload_len, 40);
    assert!(reply.wire().payload[40..].iter().all(|&b| b == 0));
}
pub fn checked_selected(
    reply: &store::NativeReadReply,
    view: &desktop::ReadBindingView,
    body: &[u8],
    id: u64,
) -> Vec<u8> {
    let wire = success(desktop::encode_envelope_wire(&reply.wire()));
    let decoded = success(desktop::decode_envelope_wire(&wire));
    let typed = success(artifact::ReadSelectedReply::decode(&decoded));
    assert_eq!(
        (typed.request_id, typed.status, typed.revision),
        (id, 0, view.selected_revision)
    );
    let selected = reply.selected().expect("actual authorized native lease");
    let desc = selected.descriptor();
    assert_eq!(desc.kind, store::SharedObjectKind::SelectedText);
    assert_eq!(
        (desc.handle.pack(), desc.object_generation, desc.byte_len),
        (typed.data_shm, typed.object_generation, typed.byte_len)
    );
    assert_eq!(desc.object_generation, desc.handle.generation);
    assert_ne!(typed.data_shm, 0);
    assert_eq!(desc.byte_len as usize, TEXT_HEADER_LEN + body.len());
    let mut bytes = vec![0xa5; desc.byte_len as usize];
    success(selected.copy_into(0, &mut bytes));
    let header = success(EditorTextHeaderV0::decode_le(&bytes[..TEXT_HEADER_LEN]));
    success(header.validate_bytes(&bytes[TEXT_HEADER_LEN..]));
    assert_eq!(header.selected_object_id, view.object_id);
    assert_eq!(header.revision, view.selected_revision);
    assert_eq!(header.content_hash, view.selected_hash);
    assert_eq!(&bytes[TEXT_HEADER_LEN..], body);
    assert_eq!(
        success(header.encode_le()).as_slice(),
        &bytes[..TEXT_HEADER_LEN]
    );
    bytes
}
pub fn unchanged_denial(
    selected: &store::NativeSelectedRead,
    descriptor: &store::ObjectDescriptor,
    status: StoreStatus,
) {
    let mut output = [0xa5; 8];
    denied(selected.copy_checked(descriptor, 0, &mut output), status);
    assert_eq!(output, [0xa5; 8]);
}
pub fn wait_past(deadline: Instant) {
    if let Some(left) = deadline.checked_duration_since(Instant::now()) {
        std::thread::sleep(left);
    }
    assert!(Instant::now() >= deadline);
}

// Each result comes from an actual owner-observed JoinHandle.join; numeric
// diagnostics never create a ProducerId or a proof. Only finished rows join.
pub fn join_desktop(rig: &Rig, id: &desktop::ProducerId) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match rig.h.join_native_finished(id) {
            Ok(proof) => {
                assert_eq!(proof.outcome(), desktop::JoinOutcome::Returned);
                return proof.producer_id();
            }
            Err(Status::NotReady) => {
                assert!(
                    Instant::now() < deadline,
                    "actual Desktop producer did not finish"
                );
                std::thread::yield_now();
            }
            Err(error) => panic!("Desktop producer join: {error:?}"),
        }
    }
}
pub fn join_store(host: &store::StoreHost, id: &desktop::ProducerId) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match host.join_native_read_finished(id) {
            Ok(proof) => {
                assert_eq!(proof.outcome(), desktop::JoinOutcome::Returned);
                return proof.producer_id();
            }
            Err(StoreStatus::NotReady) => {
                assert!(
                    Instant::now() < deadline,
                    "actual Store producer did not finish"
                );
                std::thread::yield_now();
            }
            Err(error) => panic!("Store producer join: {error:?}"),
        }
    }
}
