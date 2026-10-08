//! Assertion-only NPR fixtures. These use no volatile EditorClient or issuer setter.
#[allow(dead_code)]
#[path = "../../../desktop/tests/support/editor_host_assertions.rs"]
pub mod ui;

use artifact_store_schema::editor_preview::{
    EditorPreviewChromeStateV0, EditorPreviewChromeV0, EditorPreviewDataV0, EditorPreviewPhaseV0,
};
use artifact_store_schema::editor_save::{EditorTextHeaderV0, TEXT_HEADER_LEN};
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as editor,
    desktop_focus_v1 as focus, desktop_surface_v1 as surface,
};
use kernel_api::ipc::Envelope;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use store_service::editor_store::{self as store, StoreStatus};

pub const A: &[u8] = b"note=old\n";
pub const B: &[u8] = b"other=live\n";
pub fn success<T, E>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("expected actual successful API result"),
    }
}
pub fn denied<T, E: std::fmt::Debug + PartialEq>(r: Result<T, E>, expected: E) {
    match r {
        Err(e) => assert_eq!(e, expected),
        Ok(_) => panic!("denial exposed a value"),
    }
}
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn canonical(q: Envelope) -> Envelope {
    let w = success(desktop::encode_envelope_wire(&q));
    let r = success(desktop::decode_envelope_wire(&w));
    assert_eq!(success(desktop::encode_envelope_wire(&r)), w);
    r
}
pub fn counts(h: &desktop::HostDesktop) -> [u32; 10] {
    let c = success(h.preview_counts());
    [
        c.live_pins,
        c.pin_rows,
        c.inactive,
        c.active,
        c.retired,
        c.retained_view_leases,
        c.ticket_rows,
        c.live_tickets,
        c.read_contexts,
        c.query_slots,
    ]
}
pub fn temporary() -> tempfile::TempDir {
    #[cfg(target_os = "linux")]
    {
        success(
            tempfile::Builder::new()
                .prefix("ramenos-native-preview-")
                .tempdir(),
        )
    }
    #[cfg(target_os = "macos")]
    {
        let parent = Path::new("/System/Volumes/Data/private/tmp");
        let m = success(fs::symlink_metadata(parent));
        assert!(m.is_dir() && !m.file_type().is_symlink());
        assert_eq!((m.uid(), m.gid(), m.mode() & 0o7777), (0, 0, 0o1777));
        success(
            tempfile::Builder::new()
                .prefix("ramenos-native-preview-")
                .tempdir_in(parent),
        )
    }
}
pub fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, p: &Path, rows: &mut BTreeMap<PathBuf, Vec<u8>>, total: &mut usize) {
        for e in success(fs::read_dir(p)) {
            let p = success(e).path();
            let m = success(fs::symlink_metadata(&p));
            assert!(!m.file_type().is_symlink());
            if m.is_dir() {
                walk(root, &p, rows, total);
            } else {
                assert!(m.is_file() && m.len() <= 131_072 && rows.len() < 512);
                *total = total.checked_add(m.len() as usize).unwrap();
                assert!(*total <= 2 * 1024 * 1024);
                let b = success(fs::read(&p));
                assert_eq!(b.len() as u64, m.len());
                assert!(
                    rows.insert(success(p.strip_prefix(root)).to_path_buf(), b)
                        .is_none()
                );
            }
        }
    }
    let mut rows = BTreeMap::new();
    let mut total = 0;
    walk(root, root, &mut rows, &mut total);
    rows
}
pub fn profile(count: usize) -> store::FixtureProfile {
    store::FixtureProfile {
        origin_ms: 1,
        initial_objects: (0..count)
            .map(|i| store::InitialObject {
                owner_id: i as u64 + 1,
                object_id: 101 + i as u64,
                generation: 1,
                revision: 1,
                bytes: if i == 0 { A.to_vec() } else { B.to_vec() },
            })
            .collect(),
        fixture_signing_seed: [0x53; 32],
        fixture_key_id: "native-preview-read-fixture-v0".into(),
    }
}

pub struct Fixture {
    pub h: desktop::HostDesktop,
    pub c: Arc<desktop::FixtureController>,
    pub drivers: Vec<ui::Driver>,
    pub store: Option<Arc<store::PreparedPreviewStore>>,
    pub root: Option<tempfile::TempDir>,
    pub path: PathBuf,
    pub initial: BTreeMap<PathBuf, Vec<u8>>,
    pub preview_pauses: Vec<Arc<desktop::PreviewPause>>,
    pub read_pauses: Vec<desktop::NativeReadPause>,
    pub store_pauses: Vec<Arc<store::PauseToken>>,
    pub joined: BTreeSet<u64>,
}
impl Fixture {
    pub fn new(count: usize) -> Self {
        Self::ttl(count, 30_000, 30_000)
    }
    pub fn ttl(count: usize, preview_ttl: u64, instance_ttl: u64) -> Self {
        assert!((1..=2).contains(&count));
        let (h, c, witness, enrollment) = success(desktop::HostDesktop::new_store_preview_read(
            ui::config(preview_ttl, instance_ttl),
        ));
        let mut drivers = Vec::new();
        for i in 0..count {
            let s = success(c.register_store_session(
                i as u64 + 1,
                ui::APP,
                ui::MANIFEST,
                101 + i as u64,
                1,
                1,
            ));
            let mut d = ui::Driver {
                s,
                queue: 0,
                next: 1,
                mods: 0,
            };
            d.attach(&h, 1);
            drivers.push(d);
        }
        let root = temporary();
        let path = root.path().join("owner");
        let store = Arc::new(success(store::StoreFixture::prepare_preview_read(
            &path,
            profile(count),
            witness,
            enrollment,
        )));
        let initial = files(&path);
        Self {
            h,
            c: Arc::new(c),
            drivers,
            store: Some(store),
            root: Some(root),
            path,
            initial,
            preview_pauses: vec![],
            read_pauses: vec![],
            store_pauses: vec![],
            joined: BTreeSet::new(),
        }
    }
    pub fn prepared(&self) -> &store::PreparedPreviewStore {
        self.store.as_ref().unwrap()
    }
    pub fn host(&self) -> store::StoreHost {
        self.prepared().host()
    }
    pub fn selector(&mut self, i: usize, now: u64) -> desktop::PreviewUiTicket {
        self.drivers[i].launcher(&self.h, now);
        self.drivers[i].send(&self.h, 40, 1, now);
        let ticket = success(self.h.take_preview_action(&self.drivers[i].s.chrome.peer));
        let v = ticket.view();
        assert_eq!(v.kind, desktop::PreviewUiTicketKind::Selector);
        assert_eq!(v.input_sequence, self.drivers[i].next - 1);
        assert_eq!(v.session_id, self.drivers[i].s.session_id);
        assert_eq!(v.session_generation, self.drivers[i].s.session_generation);
        assert_eq!(
            v.deadline.duration_since(v.entered),
            Duration::from_millis(1000)
        );
        let q = success(editor::PrepareLaunch::decode(&canonical(*ticket.request())));
        assert_eq!(q.request_id, v.input_sequence);
        assert_eq!(q.requested_rights, 63);
        assert_eq!(q.reserved, 0);
        assert_eq!(q.application_hash, ui::APP);
        denied(
            self.h.take_preview_action(&self.drivers[i].s.chrome.peer),
            Status::NotReady,
        );
        ticket
    }
    pub fn pin(&self, i: usize, ttl: u64) -> desktop::StoreSelectionPin {
        success(
            self.prepared()
                .pin_selection(&self.drivers[i].s.chrome.peer, ttl),
        )
    }
    pub fn prepare(
        &self,
        i: usize,
        ticket: &mut desktop::PreviewUiTicket,
        pin: &desktop::StoreSelectionPin,
        now: u64,
    ) -> editor::PrepareLaunchReply {
        let q = canonical(*ticket.request());
        let wire = canonical(success(self.h.prepare_store_preview(
            &self.drivers[i].s.chrome.peer,
            ticket,
            pin,
            now,
        )));
        assert_eq!(
            (wire.protocol, wire.msg_type, wire.handle),
            (q.protocol, 2, kernel_api::cap::Handle::INVALID)
        );
        let r = success(editor::PrepareLaunchReply::decode(&wire));
        assert_eq!(r.status, 0);
        assert_eq!(r.request_id, ticket.view().input_sequence);
        assert!(r.plan_id > 0 && r.preview_revision > 0 && r.preview_len == 464);
        denied(
            self.h
                .prepare_store_preview(&self.drivers[i].s.chrome.peer, ticket, pin, now),
            Status::NotReady,
        );
        r
    }
    pub fn preview_bytes(&self, i: usize, r: &editor::PrepareLaunchReply, now: u64) -> Vec<u8> {
        ui::read(
            &self.h,
            &self.drivers[i].s.chrome,
            &ui::desc(r.preview_shm, desktop::ObjectKind::Preview, r.preview_len),
            now,
        )
    }
    pub fn approval(
        &mut self,
        i: usize,
        r: &editor::PrepareLaunchReply,
        now: u64,
    ) -> desktop::PreviewUiTicket {
        self.drivers[i].send(&self.h, 40, 2, now);
        self.drivers[i].send(&self.h, 40, 1, now);
        let ticket = success(self.h.take_preview_action(&self.drivers[i].s.chrome.peer));
        let v = ticket.view();
        assert_eq!(v.kind, desktop::PreviewUiTicketKind::Approval);
        let q = success(editor::ConfirmLaunch::decode(&canonical(*ticket.request())));
        assert_eq!(
            (q.plan_id, q.preview_revision),
            (r.plan_id, r.preview_revision)
        );
        assert_eq!(
            (v.plan_id, v.preview_revision),
            (r.plan_id, r.preview_revision)
        );
        assert_eq!(q.request_id, self.drivers[i].next - 1);
        assert_eq!(
            v.deadline.duration_since(v.entered),
            Duration::from_millis(1000)
        );
        ticket
    }
    pub fn pending(
        &self,
        i: usize,
        t: &mut desktop::PreviewUiTicket,
        now: u64,
    ) -> desktop::PendingStoreInstance {
        let p = success(
            self.h
                .confirm_store_preview(&self.drivers[i].s.chrome.peer, t, now),
        );
        denied(
            self.h
                .confirm_store_preview(&self.drivers[i].s.chrome.peer, t, now),
            Status::NotReady,
        );
        p
    }
    pub fn launch(&mut self, i: usize, now: u64) -> store::PairedStoreLaunch {
        let mut t = self.selector(i, now);
        let pin = self.pin(i, 30_000);
        let preview = self.prepare(i, &mut t, &pin, now);
        let mut approval = self.approval(i, &preview, now);
        let pending = self.pending(i, &mut approval, now);
        let activation = success(self.prepared().activate_read(&pending));
        let mut holder = Some(success(self.h.take_store_launch(
            &self.drivers[i].s.chrome.peer,
            &pending,
            activation.stamp(),
            now,
        )));
        let live = success(activation.pair_launch(&mut holder));
        assert!(holder.is_none());
        match activation.pair_launch(&mut holder) {
            Err(e) => assert_eq!(e.status(), StoreStatus::NotReady),
            Ok(_) => panic!("paired twice"),
        }
        denied(
            self.h
                .take_store_launch(
                    &self.drivers[i].s.chrome.peer,
                    &pending,
                    activation.stamp(),
                    now,
                )
                .map_err(|e| e.status()),
            Status::NotReady,
        );
        self.drivers[i].send(&self.h, 40, 2, now);
        live
    }
    pub fn preview_pause(
        &mut self,
        i: usize,
        point: desktop::PreviewPausePoint,
    ) -> Arc<desktop::PreviewPause> {
        let p = Arc::new(success(
            self.c
                .pause_next_store_preview(self.drivers[i].s.session_id, point),
        ));
        assert!(self.preview_pauses.len() < 32);
        self.preview_pauses.push(p.clone());
        p
    }
    pub fn read_pause(&mut self, i: usize) -> usize {
        let p = success(self.c.pause_next_store_read(self.drivers[i].s.session_id));
        assert!(self.read_pauses.len() < 32);
        self.read_pauses.push(p);
        self.read_pauses.len() - 1
    }
    pub fn store_pause(&mut self, live: &store::PairedStoreLaunch) -> Arc<store::PauseToken> {
        let pause = Arc::new(success(
            self.prepared()
                .controller()
                .pause_next(live.endpoint(), store::PausePoint::BeforeReadReply),
        ));
        assert!(self.store_pauses.len() < 32);
        self.store_pauses.push(pause.clone());
        pause
    }
    pub fn unchanged(&self) {
        assert_eq!(files(&self.path), self.initial);
        let e = success(self.prepared().controller().evidence());
        assert!(e.exchanges.len() <= 256);
        assert_eq!(e.ledger.active_io_workers, 0);
        for object in e.objects {
            assert_eq!(
                (
                    object.mutation_dispatch_count,
                    object.permit_count,
                    object.transition_count
                ),
                (0, 0, 0)
            );
            assert!(object.operations.is_empty());
            assert_eq!(object.selected.revision, 1);
        }
    }
    pub fn join(&mut self, id: &desktop::ProducerId) -> u64 {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match self.h.join_native_finished(id) {
                Ok(p) => {
                    assert_eq!(p.outcome(), desktop::JoinOutcome::Returned);
                    assert!(self.joined.insert(p.producer_id()));
                    return p.producer_id();
                }
                Err(Status::NotReady) => {
                    assert!(
                        Instant::now() < deadline,
                        "actual installed producer failed to join"
                    );
                    std::thread::yield_now();
                }
                Err(e) => panic!("owned producer join {e:?}"),
            }
        }
    }
    pub fn join_all(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.h.native_producer_counts().held != 0 {
            assert!(Instant::now() < deadline);
            for id in self.h.native_producers() {
                match self.h.join_native_finished(&id) {
                    Ok(p) => {
                        assert_eq!(p.outcome(), desktop::JoinOutcome::Returned);
                        assert!(self.joined.insert(p.producer_id()));
                    }
                    Err(Status::NotReady) => {}
                    Err(e) => panic!("actual join {e:?}"),
                }
            }
            std::thread::yield_now();
        }
        let p = self.h.native_producer_counts();
        assert_eq!((p.held, p.io, p.joining), (0, 0, 0));
    }
    pub fn positive(
        &mut self,
        i: usize,
        live: &store::PairedStoreLaunch,
        id: u64,
        now: u64,
    ) -> Vec<u8> {
        let q = read_request(live, id);
        let before = Instant::now();
        let entry = success(self.h.begin_preview_read(
            live.desktop().bindings().artifact_origin(),
            &q,
            now,
        ));
        let call = success(entry.wait());
        assert!(call.entered() >= before && call.entered() <= Instant::now());
        assert_eq!(
            call.deadline().duration_since(call.entered()),
            Duration::from_millis(1000)
        );
        assert_eq!(
            success(desktop::encode_envelope_wire(&call.request())),
            success(desktop::encode_envelope_wire(&q))
        );
        denied(entry.wait(), Status::NotReady);
        denied(
            call.reserve_store_producer(desktop::ProducerKind::StoreIo),
            Status::Unsupported,
        );
        let registered = success(
            self.host()
                .register_preview_read(&live.endpoint().peer, call),
        );
        let before = self.h.native_producer_counts();
        let own_before = self.host().preview_read_producers().len();
        let reply = success(self.host().execute_preview_read(&registered));
        assert_eq!(self.host().preview_read_producers().len(), own_before + 2);
        assert_eq!(self.h.native_producer_counts().held, before.held + 2);
        assert_eq!(reply.producer_ids().len(), 2);
        let bytes = selected(&reply, 101 + i as u64, if i == 0 { A } else { B }, id);
        denied(
            self.host().dispatch(&live.endpoint().peer, &q),
            StoreStatus::Denied,
        );
        let descriptor = reply.selected().unwrap().descriptor();
        denied(
            self.host().read_lease(&live.endpoint().peer, &descriptor),
            StoreStatus::Denied,
        );
        denied(
            self.host().write_lease(&live.endpoint().peer, &descriptor),
            StoreStatus::Denied,
        );
        let after = self.h.native_producer_counts();
        let duplicate = success(self.host().execute_preview_read(&registered));
        assert_eq!(self.h.native_producer_counts(), after);
        assert!(duplicate.producer_ids().is_empty());
        status_reply(&duplicate.wire(), &q, 2, 36, StoreStatus::NotReady as u32);
        assert!(duplicate.selected().is_none());
        for id in entry.producer_ids() {
            self.join(id);
        }
        for id in reply.producer_ids() {
            self.join(id);
        }
        self.unchanged();
        bytes
    }
    pub fn finish(mut self) {
        for p in &self.preview_pauses {
            let _ = self.c.release_store_preview(p);
        }
        for p in &self.read_pauses {
            let _ = self.c.release_store_read(p);
        }
        for p in &self.store_pauses {
            let _ = self.prepared().controller().release(p);
        }
        self.join_all();
        self.unchanged();
        self.store.take();
        let root = self.root.take().unwrap();
        let path = root.path().to_owned();
        root.close().expect("own root removed after actual joins");
        assert_eq!(
            fs::symlink_metadata(path).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }
}
// Failure cleanup releases only our actual supported pauses and joins installed rows.
// It does not claim to kill an arbitrary hung Rust thread or treat Drop as evidence.
impl Drop for Fixture {
    fn drop(&mut self) {
        for p in &self.preview_pauses {
            let _ = self.c.release_store_preview(p);
        }
        for p in &self.read_pauses {
            let _ = self.c.release_store_read(p);
        }
        if let Some(store) = self.store.as_ref() {
            for p in &self.store_pauses {
                let _ = store.controller().release(p);
            }
        }
        let end = Instant::now() + Duration::from_secs(5);
        while self.h.native_producer_counts().held > 0 && Instant::now() < end {
            for id in self.h.native_producers() {
                let _ = self.h.join_native_finished(&id);
            }
            std::thread::yield_now();
        }
        if self.h.native_producer_counts().held != 0 {
            std::mem::forget(self.h.clone());
            if let Some(root) = self.root.take() {
                std::mem::forget(root);
            }
        }
    }
}

pub struct OwnedCaller<T> {
    handle: Option<std::thread::JoinHandle<T>>,
    controller: Arc<desktop::FixtureController>,
    pause: Arc<desktop::PreviewPause>,
}
impl<T> OwnedCaller<T> {
    pub fn new(
        handle: std::thread::JoinHandle<T>,
        controller: Arc<desktop::FixtureController>,
        pause: Arc<desktop::PreviewPause>,
    ) -> Self {
        Self {
            handle: Some(handle),
            controller,
            pause,
        }
    }
    pub fn join(mut self) -> T {
        self.handle
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| panic!("actual supported external caller panicked"))
    }
}
impl<T> Drop for OwnedCaller<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.controller.release_store_preview(&self.pause);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !handle.is_finished() && Instant::now() < deadline {
                std::thread::yield_now();
            }
            if handle.is_finished() {
                let _ = handle.join();
            } else {
                // Preserve the actual holder on a failing unsupported hung leg.
                // This is neither detach/reap evidence nor containment.
                std::mem::forget(handle);
            }
        }
    }
}
pub struct OwnedStoreCaller<T> {
    handle: Option<std::thread::JoinHandle<T>>,
    store: Arc<store::PreparedPreviewStore>,
    pause: Arc<store::PauseToken>,
}
impl<T> OwnedStoreCaller<T> {
    pub fn new(
        handle: std::thread::JoinHandle<T>,
        store: Arc<store::PreparedPreviewStore>,
        pause: Arc<store::PauseToken>,
    ) -> Self {
        Self {
            handle: Some(handle),
            store,
            pause,
        }
    }
    pub fn join(mut self) -> T {
        self.handle
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| panic!("actual Store caller panicked"))
    }
}
impl<T> Drop for OwnedStoreCaller<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.store.controller().release(&self.pause);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !handle.is_finished() && Instant::now() < deadline {
                std::thread::yield_now();
            }
            if handle.is_finished() {
                let _ = handle.join();
            } else {
                std::mem::forget(handle);
            }
        }
    }
}
pub fn boot(live: &store::PairedStoreLaunch) -> editor::InstanceBootstrap {
    success(editor::InstanceBootstrap::decode(&canonical(
        *live.desktop().bindings().bootstrap(),
    )))
}
pub fn read_request(live: &store::PairedStoreLaunch, id: u64) -> Envelope {
    let b = boot(live);
    canonical(success(
        artifact::ReadSelected {
            request_id: id,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
        }
        .encode(live.endpoint().handle),
    ))
}
pub fn status_reply(r: &Envelope, q: &Envelope, msg: u32, offset: usize, status: u32) {
    let r = canonical(*r);
    assert_eq!(
        (r.protocol, r.msg_type, r.handle),
        (q.protocol, msg, kernel_api::cap::Handle::INVALID)
    );
    assert_eq!(&r.payload[..8], &q.payload[..8]);
    assert_eq!(ui::u32_at(&r.payload, offset), status);
    for (i, b) in r.payload.iter().enumerate() {
        if !(0..8).contains(&i) && !(offset..offset + 4).contains(&i) {
            assert_eq!(*b, 0, "redacted reply byte {i}");
        }
    }
}
pub fn selected(reply: &store::PreviewReadReply, object: u64, body: &[u8], id: u64) -> Vec<u8> {
    let r = success(artifact::ReadSelectedReply::decode(&canonical(
        reply.wire(),
    )));
    assert_eq!((r.request_id, r.status, r.revision), (id, 0, 1));
    assert_eq!(r.byte_len as usize, TEXT_HEADER_LEN + body.len());
    let lease = reply.selected().expect("actual authorized selected lease");
    let d = lease.descriptor();
    assert_eq!(
        (d.handle.pack(), d.object_generation, d.byte_len),
        (r.data_shm, r.object_generation, r.byte_len)
    );
    assert_eq!(d.handle.generation, d.object_generation);
    assert!(d.kind == store::SharedObjectKind::SelectedText);
    let mut bytes = vec![0xa5; r.byte_len as usize];
    for variant in 0..4 {
        let mut altered = d.clone();
        match variant {
            0 => altered.byte_len = altered.byte_len.checked_add(1).unwrap(),
            1 => altered.handle.index = altered.handle.index.checked_add(1 << 16).unwrap(),
            2 => {
                altered.handle.generation = altered.handle.generation.checked_add(1 << 32).unwrap()
            }
            3 => altered.object_generation = altered.object_generation.checked_add(1).unwrap(),
            _ => unreachable!(),
        }
        denied(
            lease.copy_checked(&altered, 0, &mut bytes),
            StoreStatus::Denied,
        );
        assert!(bytes.iter().all(|b| *b == 0xa5));
    }
    denied(
        lease.copy_into(d.byte_len, &mut bytes[..1]),
        StoreStatus::Invalid,
    );
    assert!(bytes.iter().all(|b| *b == 0xa5));
    success(lease.copy_checked(&d, 0, &mut bytes));
    let h = success(EditorTextHeaderV0::decode_le(&bytes[..TEXT_HEADER_LEN]));
    success(h.validate_bytes(&bytes[TEXT_HEADER_LEN..]));
    assert_eq!(
        (h.selected_object_id, h.revision, h.byte_len, h.reserved),
        (object, 1, body.len() as u32, 0)
    );
    assert_eq!(h.content_hash, digest(body));
    assert_eq!(&bytes[TEXT_HEADER_LEN..], body);
    bytes
}
pub fn data(bytes: &[u8], i: usize, phase: EditorPreviewPhaseV0) -> EditorPreviewDataV0 {
    assert_eq!(bytes.len(), 464);
    let d = success(EditorPreviewDataV0::decode_le(bytes));
    assert_eq!(d.phase, phase);
    assert_eq!(
        (
            d.selected_object_id,
            d.selected_object_generation,
            d.selected_revision
        ),
        (101 + i as u64, 1, 1)
    );
    assert_eq!(d.application_hash, ui::APP);
    assert_eq!(d.manifest_hash, ui::MANIFEST);
    assert_eq!(d.selected_content_hash, digest(if i == 0 { A } else { B }));
    assert_eq!(d.records.map(|r| r.rights), [1, 1, 15, 1]);
    assert!(d.desktop_service_epoch > 0 && d.records[3].service_epoch > 0);
    if phase == EditorPreviewPhaseV0::Preview {
        assert!(d.records.iter().all(|r| r.handle == 0));
        assert_eq!((d.instance_id, d.instance_generation), (0, 0));
    } else {
        assert!(d.records.iter().all(|r| r.handle != 0));
    }
    d
}
pub fn active_data(
    f: &Fixture,
    i: usize,
    live: &store::PairedStoreLaunch,
    now: u64,
) -> EditorPreviewDataV0 {
    let b = boot(live);
    let r = success(editor::ConfirmLaunchReply::decode(&canonical(
        *live.desktop().confirm_reply(),
    )));
    assert_eq!(
        (
            r.session_generation,
            r.instance_id,
            r.instance_generation,
            r.grants_shm,
            r.grants_len,
            r.status
        ),
        (
            b.session_generation,
            b.instance_id,
            b.instance_generation,
            b.grants_shm,
            b.grants_len,
            b.status
        )
    );
    assert_eq!(b.status, 0);
    assert_eq!(b.grants_len, 464);
    let bytes = ui::read(
        &f.h,
        live.desktop().bindings().self_status(),
        &ui::desc(b.grants_shm, desktop::ObjectKind::Grants, 464),
        now,
    );
    let d = data(&bytes, i, EditorPreviewPhaseV0::Active);
    assert_eq!(
        (
            d.session_id,
            d.session_generation,
            d.instance_id,
            d.instance_generation
        ),
        (
            b.session_id,
            b.session_generation,
            b.instance_id,
            b.instance_generation
        )
    );
    assert_eq!(
        d.records[0].handle,
        live.desktop().bindings().self_status().handle.pack()
    );
    assert_eq!(
        d.records[1].handle,
        live.desktop().bindings().focus_read().handle.pack()
    );
    assert_eq!(
        d.records[2].handle,
        live.desktop().bindings().surface().handle.pack()
    );
    assert_eq!(d.records[3].handle, live.endpoint().handle.pack());
    d
}
pub fn cancel(f: &Fixture, i: usize, plan: u64, now: u64, status: Status) {
    let s = &f.drivers[i].s;
    let q = canonical(success(
        editor::CancelPreview {
            request_id: 77,
            session_id: s.session_id,
            session_generation: s.session_generation,
            plan_id: plan,
        }
        .encode(s.chrome.handle),
    ));
    let r = f.h.dispatch_store_preview_chrome(&s.chrome.peer, &q, now);
    status_reply(&r, &q, 4, 8, status as u32);
}
pub fn retire(f: &Fixture, i: usize, live: &store::PairedStoreLaunch, now: u64) {
    let b = boot(live);
    let s = &f.drivers[i].s;
    let q = canonical(success(
        editor::RevokeInstance {
            request_id: 78,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
        }
        .encode(s.chrome.handle),
    ));
    status_reply(
        &f.h.dispatch_store_preview_chrome(&s.chrome.peer, &q, now),
        &q,
        12,
        8,
        Status::Ok as u32,
    );
}
pub fn freeze(f: &Fixture, i: usize, live: &store::PairedStoreLaunch, sequence: u64, now: u64) {
    let b = boot(live);
    let endpoint = live.desktop().bindings().surface();
    let create: surface::CreateReply = ui::call(
        &f.h,
        endpoint,
        surface::Create {
            request_id: 90,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
            width: 640,
            height: 480,
            format: 1,
            reserved: 0,
        },
        now,
    );
    assert_eq!(create.status, 0);
    assert_eq!((create.stride, create.format), (2560, 1));
    let focus: focus::AssignReply = ui::call(
        &f.h,
        &f.drivers[i].s.chrome,
        focus::Assign {
            request_id: 89,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
            surface_id: create.surface_id,
            surface_generation: create.surface_generation,
        },
        now,
    );
    assert_eq!(focus.status, 0);
    assert!(focus.focus_epoch > 0);
    let acquire: surface::AcquireReply = ui::call(
        &f.h,
        endpoint,
        surface::Acquire {
            request_id: 91,
            surface_id: create.surface_id,
            surface_generation: create.surface_generation,
            buffer_index: 0,
            reserved: 0,
        },
        now,
    );
    assert_eq!(acquire.status, 0);
    assert_eq!(acquire.byte_len, 640 * 480 * 4);
    let d = ui::desc(
        acquire.buffer_shm,
        desktop::ObjectKind::SurfaceBuffer,
        acquire.byte_len,
    );
    let lease = success(f.h.write_lease(&endpoint.peer, &d, acquire.mapping_generation, now));
    success(lease.copy_from(0, &vec![255; 640 * 480 * 4], now));
    denied(
        lease.copy_from(acquire.byte_len - 1, &[0; 4], now),
        Status::Invalid,
    );
    let r: surface::PresentReply = ui::call(
        &f.h,
        endpoint,
        surface::Present {
            request_id: 92,
            surface_id: create.surface_id,
            surface_generation: create.surface_generation,
            mapping_generation: acquire.mapping_generation,
            sequence,
            buffer_index: 0,
            reserved: 0,
        },
        now,
    );
    assert_eq!((r.status, r.sequence), (0, sequence));
    denied(lease.copy_from(0, &[0; 4], now), Status::Stale);
}
pub fn present(
    f: &Fixture,
    i: usize,
    live: &store::PairedStoreLaunch,
    sequence: u64,
    now: u64,
) -> desktop::ComposedFrame {
    freeze(f, i, live, sequence, now);
    success(f.h.compose_next(&f.drivers[i].s.compositor, now))
}
pub fn frame(f: &desktop::ComposedFrame, unavailable: bool, blank: bool) {
    assert!(f.session_id > 0);
    assert_eq!(f.bgra.len(), 640 * 568 * 4);
    let mut protected = Vec::with_capacity(225280);
    protected.extend_from_slice(&f.bgra[..48 * 2560]);
    protected.extend_from_slice(&f.bgra[528 * 2560..]);
    assert_eq!(protected.len(), 225280);
    assert_eq!(
        ui::hash(&protected),
        if unavailable {
            "06c45d415ba0ffa22b05c50daff5cfaf1f5eecc0f7dd6537a1a1ac2f9d6e5621"
        } else {
            "ed29bf9aa5cce83a0421707f9bc8eecce32086154b9ce9b58c3994fdfb8d5206"
        }
    );
    if blank {
        assert_eq!(
            ui::hash(&f.bgra[48 * 2560..528 * 2560]),
            "f7f3eed6bad2c170eb0bccd6c368b320688a207a59c27321b412dd0774bc4df1"
        );
        assert_eq!(
            ui::hash(&f.bgra),
            if unavailable {
                "58b89ef26ea3176b5115832c08b6551bf8cc7b8f131184743ee9cc0e14b44faa"
            } else {
                "9026cb19fa72aae4d1c36d397fc353f73f81fcf20649fca2c4d2471cd7a876c6"
            }
        );
    }
    // Fixed literal indicator samples, independent of the service formatter.
    assert_eq!(
        &f.bgra[(8 * 640 + 8) * 4..(8 * 640 + 8) * 4 + 4],
        &[0, 0, 0, 255]
    );
    assert_eq!(
        &f.bgra[(9 * 640 + 9) * 4..(9 * 640 + 9) * 4 + 4],
        &[0, 192, 0, 255]
    );
}
pub fn chrome(
    f: &Fixture,
    i: usize,
    pixels: &desktop::ComposedFrame,
    d: &EditorPreviewDataV0,
    unavailable: bool,
) -> desktop::PreviewChromeRecordObservation {
    let o = success(f.c.preview_chrome_observation(&f.drivers[i].s.compositor));
    assert_eq!(
        (o.session_id, o.instance_id, o.focus_epoch, o.sequence),
        (
            pixels.session_id,
            pixels.instance_id,
            pixels.focus_epoch,
            pixels.sequence
        )
    );
    let c = success(EditorPreviewChromeV0::decode_le(&o.bytes));
    assert_eq!(
        c.state,
        if unavailable {
            EditorPreviewChromeStateV0::Unavailable
        } else {
            EditorPreviewChromeStateV0::NoSave
        }
    );
    assert_eq!(
        (
            c.actor.owner_id,
            c.actor.session_id,
            c.actor.session_generation,
            c.actor.instance_id,
            c.actor.instance_generation
        ),
        (
            i as u64 + 1,
            d.session_id,
            d.session_generation,
            d.instance_id,
            d.instance_generation
        )
    );
    assert_eq!(
        (
            c.selected_object_id,
            c.selected_object_generation,
            c.current_store_epoch
        ),
        (
            d.selected_object_id,
            d.selected_object_generation,
            d.records[3].service_epoch
        )
    );
    assert_eq!(
        (c.attachment_version, c.status_version),
        (o.attachment_version, o.status_version)
    );
    assert!(o.attachment_version > 0 && o.status_version > 0);
    assert_eq!(
        (
            c.original_backend_epoch,
            c.operation_id,
            c.source_handle,
            c.source_generation,
            c.expected_revision,
            c.result_revision,
            c.source_body_len
        ),
        (0, 0, 0, 0, 0, 0, 0)
    );
    assert_eq!(
        (
            c.source_content_hash,
            c.expected_content_hash,
            c.receipt_sha256
        ),
        ([0; 32], [0; 32], [0; 32])
    );
    o
}
