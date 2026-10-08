//! Assertion-only fixtures. Every effect crosses the frozen host API; no service stub.
use desktop_service::dev::Status;
use desktop_service::editor_dev::*;
use kernel_api::cap::Handle;
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as editor,
    desktop_focus_v1 as focus, desktop_surface_v1 as surface, input_v1 as input,
};
use kernel_api::ipc::Envelope;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub const APP: [u8; 32] = [0x41; 32];
pub const MANIFEST: [u8; 32] = [0x4d; 32];
pub const FONT_HASH: &str = "50ab6522c495f06a69590ccb6958458be15e02a2ca4c6b1024b052e826d21c64";
pub const OLD_HASH: &str = "69679689bb8a4cc1ce90a4f761525f8820f73056401d116736443a6585fee3c1";
pub const NEW_HASH: &str = "cfa160298051a6ece5616dc73eb963da3ae6fe55324c59b6370bc6fe1281cf7f";
pub const FRAME_BYTES: usize = 640 * 480 * 4;
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
pub fn config(preview: u64, instance: u64) -> HostConfig {
    let data = include_bytes!("../fixtures/ascii8x16_v0.bin");
    assert_eq!(hash(data), FONT_HASH);
    assert_eq!(
        hash(include_bytes!("../fixtures/ascii8x16_v0.golden.json")),
        "90141218185439b97a6d22a959f1c69bbbf17b6c2408d8eabb2069fb76c1499c"
    );
    HostConfig {
        preview_ttl_ms: preview,
        instance_ttl_ms: instance,
        font_rows: std::array::from_fn(|i| data[i * 16..(i + 1) * 16].try_into().unwrap()),
    }
}
pub fn call<Q: EditorMessage, R: EditorMessage>(
    h: &HostDesktop,
    e: &Endpoint,
    q: Q,
    now: u64,
) -> R {
    let frame = q.encode(e.handle).expect("generated initialized message");
    let wire = encode_envelope_wire(&frame).expect("canonical request");
    let decoded = decode_envelope_wire(&wire).expect("strict bridge request");
    let reply = h.dispatch(&e.peer, &decoded, now);
    let wire = encode_envelope_wire(&reply).expect("canonical service reply");
    R::decode(&decode_envelope_wire(&wire).unwrap()).expect("generated typed reply")
}
pub fn desc(raw: u64, kind: ObjectKind, len: u32) -> ObjectDescriptor {
    let handle = Handle::unpack(raw);
    assert_eq!(handle.pack(), raw, "canonical descriptor handle");
    ObjectDescriptor {
        handle,
        kind,
        object_generation: handle.generation,
        byte_len: len,
    }
}
pub fn read(h: &HostDesktop, e: &Endpoint, d: &ObjectDescriptor, now: u64) -> Vec<u8> {
    let lease = h
        .read_lease(&e.peer, d, now)
        .expect("owned bounded read lease");
    let mut bytes = vec![0; d.byte_len as usize];
    lease
        .copy_into(0, &mut bytes, now)
        .expect("checked data-plane read");
    bytes
}
pub fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}
pub fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
pub fn zero_reply(r: &Envelope, status_offset: usize, status: Status) {
    assert_eq!(u32_at(&r.payload, status_offset), status as u32);
    for (i, b) in r.payload.iter().enumerate() {
        if !(0..8).contains(&i) && !(status_offset..status_offset + 4).contains(&i) {
            assert_eq!(*b, 0, "denied reply exposes byte {i}");
        }
    }
}
pub fn deny<Q: EditorMessage>(
    h: &HostDesktop,
    e: &Endpoint,
    q: Q,
    offset: usize,
    now: u64,
    status: Status,
) {
    let req = q.encode(e.handle).unwrap();
    let r = h.dispatch(&e.peer, &req, now);
    let correlation = if req.protocol == 802 && req.msg_type == 3 {
        24
    } else {
        0
    };
    assert_eq!(
        &r.payload[..8],
        &req.payload[correlation..correlation + 8],
        "correlation"
    );
    zero_reply(&r, offset, status);
}
pub struct Fixture {
    pub h: HostDesktop,
    pub c: FixtureController,
    pub d: Driver,
}
pub struct Driver {
    pub s: SessionFixture,
    pub queue: u64,
    pub next: u64,
    pub mods: u32,
}
pub struct Live {
    pub client: EditorClient,
    pub bindings: EditorBindings,
    pub boot: editor::InstanceBootstrap,
    pub surface: surface::CreateReply,
}
pub fn clone_bindings(b: &EditorBindings) -> EditorBindings {
    EditorBindings {
        bootstrap: b.bootstrap,
        self_status: b.self_status.clone(),
        focus_read: b.focus_read.clone(),
        surface: b.surface.clone(),
        artifact: b.artifact.clone(),
    }
}
impl Fixture {
    pub fn new(bytes: &[u8]) -> Self {
        Self::ttl(bytes, 30_000, 600_000)
    }
    pub fn ttl(bytes: &[u8], preview: u64, instance: u64) -> Self {
        let (h, c) = HostDesktop::new(config(preview, instance)).expect("default-off host fixture");
        let d = Driver::register(&h, &c, 1, bytes, 0);
        Self { h, c, d }
    }
    pub fn launch(&mut self, now: u64) -> Live {
        self.d.launch(&self.h, &self.c, now)
    }
}
impl Driver {
    pub fn register(
        h: &HostDesktop,
        c: &FixtureController,
        owner: u64,
        bytes: &[u8],
        now: u64,
    ) -> Self {
        let s = c
            .register_session(owner, APP, MANIFEST, bytes, now)
            .expect("owned selected fixture");
        let mut d = Self {
            s,
            queue: 0,
            next: 1,
            mods: 0,
        };
        d.attach(h, now);
        d
    }
    pub fn attach(&mut self, h: &HostDesktop, now: u64) {
        let r: input::AttachReply = call(
            h,
            &self.s.input,
            input::Attach {
                request_id: 1,
                session_id: self.s.session_id,
                session_generation: self.s.session_generation,
                device_generation: self.s.device_generation,
            },
            now,
        );
        assert_eq!(r.status, 0);
        assert_ne!(r.queue_generation, 0);
        self.queue = r.queue_generation;
    }
    pub fn request(&self, usage: u32, phase: u32, mods: u32) -> input::ProduceKey {
        input::ProduceKey {
            session_id: self.s.session_id,
            session_generation: self.s.session_generation,
            queue_generation: self.queue,
            sequence: self.next,
            usage,
            phase,
            modifiers: mods,
            reserved: 0,
        }
    }
    pub fn send(&mut self, h: &HostDesktop, usage: u32, phase: u32, now: u64) {
        let mods = if phase == 3 {
            0
        } else if usage == 224 || usage == 225 {
            let bit = if usage == 224 { 1 } else { 2 };
            if phase == 1 {
                self.mods | bit
            } else {
                self.mods & !bit
            }
        } else {
            self.mods
        };
        let q = self.request(usage, phase, mods);
        let r: input::ProduceKeyReply = call(h, &self.s.input, q, now);
        assert_eq!(
            r.status, 0,
            "accepted input sequence {} usage {usage} phase {phase}",
            self.next
        );
        assert_eq!(r.sequence, self.next);
        self.next = self.next.checked_add(1).unwrap();
        self.mods = mods;
    }
    pub fn stroke(&mut self, h: &HostDesktop, usage: u32, now: u64) {
        self.send(h, usage, 1, now);
        self.send(h, usage, 2, now);
    }
    pub fn launcher(&mut self, h: &HostDesktop, now: u64) {
        self.send(h, 224, 1, now);
        self.stroke(h, 41, now);
        self.send(h, 224, 2, now);
    }
    pub fn preview(
        &mut self,
        h: &HostDesktop,
        c: &FixtureController,
        now: u64,
    ) -> editor::PrepareLaunchReply {
        self.launcher(h, now);
        self.stroke(h, 40, now);
        latest::<editor::PrepareLaunchReply>(&c.evidence().unwrap())
    }
    pub fn launch(&mut self, h: &HostDesktop, c: &FixtureController, now: u64) -> Live {
        let p = self.preview(h, c, now);
        assert_eq!(p.status, 0);
        assert_eq!(p.granted_rights, 63);
        self.stroke(h, 40, now);
        let r = latest::<editor::ConfirmLaunchReply>(&c.evidence().unwrap());
        assert_eq!(r.status, 0);
        assert_ne!(r.instance_id, 0);
        let bindings = c
            .editor_bindings(r.instance_id)
            .expect("actual issued client bindings");
        let boot = editor::InstanceBootstrap::decode(&bindings.bootstrap).unwrap();
        assert_eq!(boot.instance_id, r.instance_id);
        assert_eq!(boot.grants_shm, r.grants_shm);
        let mut client = EditorClient::new(h.clone(), clone_bindings(&bindings), now).unwrap();
        client
            .render(now)
            .expect("real Rust consumer produces first frame");
        let surface = latest::<surface::CreateReply>(&c.evidence().unwrap());
        assert_eq!(
            (surface.status, surface.stride, surface.format),
            (0, 2560, 1)
        );
        assert_ne!(surface.buffer0_shm, 0);
        assert_ne!(surface.buffer1_shm, 0);
        let composed = h
            .compose_next(&self.s.compositor, now)
            .expect("actual frozen frame consumer");
        assert_eq!(composed.instance_id, boot.instance_id);
        let assigned: focus::AssignReply = call(
            h,
            &self.s.chrome,
            focus::Assign {
                request_id: 2,
                session_id: self.s.session_id,
                session_generation: self.s.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
                surface_id: surface.surface_id,
                surface_generation: surface.surface_generation,
            },
            now,
        );
        assert_eq!(assigned.status, 0);
        assert_ne!(assigned.focus_epoch, 0);
        Live {
            client,
            bindings,
            boot,
            surface,
        }
    }
    pub fn event(&mut self, h: &HostDesktop, l: &mut Live, usage: u32, phase: u32, now: u64) {
        self.send(h, usage, phase, now);
        match l.client.step(now) {
            Ok(_) => {}
            Err(Status::NotReady) => {}
            Err(e) => panic!("step: {e:?}"),
        }
    }
    pub fn key(&mut self, h: &HostDesktop, l: &mut Live, usage: u32, now: u64) {
        self.event(h, l, usage, 1, now);
        self.event(h, l, usage, 2, now);
    }
    pub fn ctrl(&mut self, h: &HostDesktop, l: &mut Live, usage: u32, now: u64) {
        self.event(h, l, 224, 1, now);
        self.key(h, l, usage, now);
        self.event(h, l, 224, 2, now);
    }
    pub fn text(&mut self, h: &HostDesktop, l: &mut Live, text: &[u8], now: u64) {
        for &b in text {
            let (usage, shift) = translate(b);
            if shift {
                self.event(h, l, 225, 1, now);
            }
            self.key(h, l, usage, now);
            if shift {
                self.event(h, l, 225, 2, now);
            }
        }
    }
}
pub fn translate(b: u8) -> (u32, bool) {
    match b {
        b'a'..=b'z' => (4 + (b - b'a') as u32, false),
        b'A'..=b'Z' => (4 + (b - b'A') as u32, true),
        b'1'..=b'9' => (30 + (b - b'1') as u32, false),
        b'0' => (39, false),
        b'\n' => (40, false),
        b'\t' => (43, false),
        b' ' => (44, false),
        _ => {
            let plain = b"-=[]\\;'`,./";
            let shifted = b"_+{}|:\"~<>?";
            let usages = [45, 46, 47, 48, 49, 51, 52, 53, 54, 55, 56];
            if let Some(i) = plain.iter().position(|x| *x == b) {
                return (usages[i], false);
            }
            if let Some(i) = shifted.iter().position(|x| *x == b) {
                return (usages[i], true);
            }
            let i = b"!@#$%^&*()"
                .iter()
                .position(|x| *x == b)
                .expect("frozen printable ASCII");
            (if i == 9 { 39 } else { 30 + i as u32 }, true)
        }
    }
}
pub fn latest<T: EditorMessage>(e: &Evidence) -> T {
    e.exchanges
        .iter()
        .rev()
        .find_map(|(_, r)| T::decode(r).ok())
        .expect("actual routed typed exchange")
}
pub fn status_request(l: &Live, s: &SessionFixture) -> editor::GetStatus {
    editor::GetStatus {
        request_id: 10,
        session_id: s.session_id,
        session_generation: s.session_generation,
        instance_id: l.boot.instance_id,
        instance_generation: l.boot.instance_generation,
    }
}
pub fn observe(h: &HostDesktop, l: &Live, now: u64) -> editor::ObserveFocusReply {
    call(
        h,
        &l.bindings.self_status,
        editor::ObserveFocus {
            request_id: 11,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        now,
    )
}
pub fn poll_request(l: &Live, epoch: u64) -> focus::PollKeys {
    focus::PollKeys {
        request_id: 12,
        session_id: l.boot.session_id,
        session_generation: l.boot.session_generation,
        instance_id: l.boot.instance_id,
        instance_generation: l.boot.instance_generation,
        focus_epoch: epoch,
    }
}
pub fn selected(h: &HostDesktop, l: &Live, now: u64) -> (u64, Vec<u8>) {
    let r: artifact::ReadSelectedReply = call(
        h,
        &l.bindings.artifact,
        artifact::ReadSelected {
            request_id: 13,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        now,
    );
    assert_eq!(r.status, 0);
    let d = desc(r.data_shm, ObjectKind::SelectedText, r.byte_len);
    assert_eq!(d.object_generation, r.object_generation);
    let bytes = read(h, &l.bindings.artifact, &d, now);
    assert!(bytes.len() >= 64);
    assert_eq!(u32_at(&bytes, 0), 1);
    assert_eq!(u32_at(&bytes, 4) as usize, bytes.len());
    assert_eq!(u64_at(&bytes, 16), r.revision);
    assert_eq!(u32_at(&bytes, 56) as usize, bytes.len() - 64);
    assert_eq!(u32_at(&bytes, 60), 0);
    assert_eq!(&bytes[24..56], &Sha256::digest(&bytes[64..])[..]);
    (r.revision, bytes[64..].to_vec())
}
pub fn commit_request(
    h: &HostDesktop,
    l: &Live,
    bytes: &[u8],
    revision: u64,
    now: u64,
) -> artifact::Commit {
    let op: artifact::AllocateSaveIdReply = call(
        h,
        &l.bindings.artifact,
        artifact::AllocateSaveId {
            request_id: 14,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            expected_revision: revision,
        },
        now,
    );
    assert_eq!(op.status, 0);
    assert_ne!(op.operation_id, 0);
    let source = h
        .draft_source(&l.bindings.artifact.peer, revision, bytes, now)
        .unwrap();
    artifact::Commit {
        request_id: 15,
        session_id: l.boot.session_id,
        session_generation: l.boot.session_generation,
        expected_revision: revision,
        operation_id: op.operation_id,
        source_shm: source.handle.pack(),
        byte_len: source.byte_len,
        reserved: 0,
    }
}
pub fn receipt(
    h: &HostDesktop,
    e: &Endpoint,
    s: &SessionFixture,
    operation: u64,
    now: u64,
) -> Vec<u8> {
    let r: artifact::SaveStatusReply = call(
        h,
        e,
        artifact::SaveStatus {
            request_id: 16,
            session_id: s.session_id,
            session_generation: s.session_generation,
            operation_id: operation,
        },
        now,
    );
    assert_eq!(r.status, 0);
    assert_eq!(r.operation_id, operation);
    assert_eq!(r.receipt_len, 176);
    let bytes = read(
        h,
        e,
        &desc(r.receipt_shm, ObjectKind::Receipt, r.receipt_len),
        now,
    );
    assert_eq!(u32_at(&bytes, 0), 1);
    assert_eq!(u32_at(&bytes, 4), 176);
    assert_eq!(u32_at(&bytes, 12), 0);
    assert_eq!(u64_at(&bytes, 72), operation);
    assert_eq!(u64_at(&bytes, 24), s.session_id);
    bytes
}
pub fn crop(f: &ComposedFrame) -> Vec<u8> {
    assert_eq!(f.bgra.len(), 640 * 568 * 4);
    f.bgra[48 * 640 * 4..528 * 640 * 4].to_vec()
}
pub fn assert_golden(f: &ComposedFrame, new: bool) {
    let b = crop(f);
    assert_eq!(hash(&b), if new { NEW_HASH } else { OLD_HASH });
    for (x, y, v) in [
        (0, 0, 255),
        (0, 4, 0),
        (4, 4, 0),
        (5, 4, 255),
        (44, 4, 0),
        (45, 4, 255),
        (0, 16, 0),
        (1, 31, 0),
        (2, 16, 255),
        (639, 479, 255),
    ] {
        let i = (y * 640 + x) * 4;
        assert_eq!(&b[i..i + 4], &[v, v, v, 255]);
    }
    let i = (4 * 640 + 40) * 4;
    let v = if new { 0 } else { 255 };
    assert_eq!(&b[i..i + 4], &[v, v, v, 255]);
}
pub fn assert_draft_equal(a: &DraftSnapshot, b: &DraftSnapshot) {
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(a.cursor, b.cursor);
    assert_eq!(a.selection, b.selection);
    assert_eq!(a.first_visible_line, b.first_visible_line);
    assert_eq!(a.confirmed_revision, b.confirmed_revision);
}
pub fn assert_ledger_equal(a: &ResourceLedger, b: &ResourceLedger) {
    assert_eq!(
        (
            a.sessions,
            a.selected_objects,
            a.instances,
            a.endpoints,
            a.shared_objects,
            a.queued_keys
        ),
        (
            b.sessions,
            b.selected_objects,
            b.instances,
            b.endpoints,
            b.shared_objects,
            b.queued_keys
        )
    );
    assert_eq!(a.operations_per_object, b.operations_per_object);
}
pub fn assert_caps(e: &Evidence) {
    assert!(e.volatile_backend);
    assert!(e.exchanges.len() <= 256);
    assert!(e.ledger.sessions <= 2 && e.ledger.selected_objects <= 2 && e.ledger.instances <= 16);
    assert!(
        e.ledger.endpoints <= 64 && e.ledger.shared_objects <= 32 && e.ledger.queued_keys <= 128
    );
    assert!(e.ledger.operations_per_object.len() <= 2);
    assert!(e.ledger.operations_per_object.iter().all(|(_, n)| *n <= 16));
}
pub fn revoke(h: &HostDesktop, s: &SessionFixture, b: &editor::InstanceBootstrap, now: u64) {
    let r: editor::RevokeInstanceReply = call(
        h,
        &s.chrome,
        editor::RevokeInstance {
            request_id: 18,
            session_id: s.session_id,
            session_generation: s.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
        },
        now,
    );
    assert_eq!(r.status, 0);
}
pub fn close(h: &HostDesktop, s: &SessionFixture, b: &editor::InstanceBootstrap, now: u64) {
    let r: editor::CloseInstanceReply = call(
        h,
        &s.chrome,
        editor::CloseInstance {
            request_id: 19,
            session_id: s.session_id,
            session_generation: s.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
        },
        now,
    );
    assert_eq!(r.status, 0);
}
pub fn render(h: &HostDesktop, d: &Driver, l: &mut Live, now: u64) -> ComposedFrame {
    let seq = l.client.render(now).unwrap();
    let f = h.compose_next(&d.s.compositor, now).unwrap();
    assert_eq!(f.sequence, seq);
    f
}
pub fn barrier_note(record: &mut Record, name: &str, b: &BarrierSnapshot) {
    record.note(name,&format!("{{\"request_id\":{},\"operation_id\":{},\"entered\":{},\"settled\":{},\"permit_issued\":{}}}",
        b.request_id,b.operation_id,b.entered,b.settled,b.permit_issued));
}
pub fn assert_volatile_label(f: &ComposedFrame) {
    assert_label(f, b"VOLATILE / IN-PROCESS");
}
pub fn assert_label(f: &ComposedFrame, label: &[u8]) {
    let font = include_bytes!("../fixtures/ascii8x16_v0.bin");
    // Independently match the pinned glyph raster in trusted chrome/status only.
    let found = [0usize, 8, 16, 24, 32, 528, 536, 544, 552]
        .iter()
        .any(|&y| {
            (0..=640 - label.len() * 8).step_by(8).any(|x| {
                label.iter().enumerate().all(|(i, &c)| {
                    (0..16).all(|dy| {
                        (0..8).all(|dx| {
                            let bit = font[(c as usize - 32) * 16 + dy] & (128 >> dx) != 0;
                            let o = ((y + dy) * 640 + x + i * 8 + dx) * 4;
                            let want = if bit {
                                [0, 0, 0, 255]
                            } else {
                                [224, 224, 224, 255]
                            };
                            f.bgra[o..o + 4] == want
                        })
                    })
                })
            })
        });
    assert!(
        found,
        "trusted output lacks pinned fixture label {:?}",
        label
    );
}
pub struct Record {
    case: &'static str,
    observations: Vec<String>,
    frames: Vec<ComposedFrame>,
}
impl Record {
    pub fn new(case: &'static str) -> Self {
        Self {
            case,
            observations: vec![],
            frames: vec![],
        }
    }
    pub fn note(&mut self, name: &str, value: &str) {
        assert!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
        self.observations
            .push(format!("{{\"name\":\"{name}\",\"value\":{value}}}"));
    }
    pub fn frame(&mut self, f: ComposedFrame) {
        assert!(self.frames.len() < 16);
        self.frames.push(f)
    }
    pub fn finish(&self, c: &FixtureController, index: u32) {
        let e = c.evidence().unwrap();
        self.finish_snapshot(&e, index);
    }
    pub fn finish_trace_negative(
        &self,
        c: &FixtureController,
        index: u32,
        prior: &Evidence,
        request: &Envelope,
        reply: &Envelope,
    ) {
        assert_eq!(index, 10);
        assert_eq!(
            self.case,
            "ui1_1_clock_capacity_and_counter_limits_fail_closed"
        );
        assert_eq!(prior.exchanges.len(), 256);
        assert_eq!(request.protocol, 352);
        assert_eq!(request.msg_type, 7);
        assert_eq!((reply.protocol, reply.msg_type), (352, 8));
        assert_eq!(&reply.payload[..8], &request.payload[..8]);
        zero_reply(reply, 44, Status::Exhausted);
        let status = match c.evidence() {
            Err(Status::Exhausted) => Status::Exhausted,
            Err(e) => panic!("unexpected trace terminal status {e:?}"),
            Ok(_) => panic!("overflow silently returned a truncated PASS snapshot"),
        };
        self.finish_snapshot(prior, index);
        if let Some(root) = std::env::var_os("RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE") {
            let dir = PathBuf::from(root).join(self.case);
            let prior_json = std::fs::read(dir.join("registry-10.json")).unwrap();
            let q = encode_envelope_wire(request).unwrap();
            let r = encode_envelope_wire(reply).unwrap();
            let json = format!(
                "{{\"schema_version\":1,\"case\":\"{}\",\"registry_index\":10,\"prior_record_sha256\":\"{}\",\"prior_exchange_count\":256,\"terminal_request_hex\":\"{}\",\"terminal_reply_hex\":\"{}\",\"post_overflow_evidence_status\":{},\"intentional_expected_failure\":true}}",
                self.case,
                hash(&prior_json),
                hex(&q),
                hex(&r),
                status as u32
            );
            std::fs::write(dir.join("trace-terminal-10.json"), json).unwrap();
        }
    }
    fn finish_snapshot(&self, e: &Evidence, index: u32) {
        assert_caps(e);
        if let Some(root) = std::env::var_os("RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE") {
            let dir = PathBuf::from(root).join(self.case);
            std::fs::create_dir_all(&dir).unwrap();
            let exchanges = e
                .exchanges
                .iter()
                .map(|(q, r)| {
                    let qwire = raw_wire(q);
                    let rwire = encode_envelope_wire(r).expect("strict actual reply");
                    format!(
                        "{{\"request_hex\":\"{}\",\"reply_hex\":\"{}\",\"request_canonical\":{}}}",
                        hex(&qwire),
                        hex(&rwire),
                        encode_envelope_wire(q).is_ok()
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let mut frames = Vec::new();
            for (i, f) in self.frames.iter().enumerate() {
                let file = format!("registry-{index}-frame-{i}.bgra");
                std::fs::write(dir.join(&file), &f.bgra).unwrap();
                let kind = if f.focus_epoch == 0 && f.sequence == 0 {
                    "recovery"
                } else {
                    "app"
                };
                frames.push(format!("{{\"file\":\"{file}\",\"frame_kind\":\"{kind}\",\"session_id\":{},\"instance_id\":{},\"focus_epoch\":{},\"sequence\":{},\"sha256\":\"{}\",\"crop_sha256\":\"{}\"}}",
                    f.session_id,f.instance_id,f.focus_epoch,f.sequence,hash(&f.bgra),hash(&crop(f))));
            }
            let ops = e
                .ledger
                .operations_per_object
                .iter()
                .map(|(id, n)| format!("[{id},{n}]"))
                .collect::<Vec<_>>()
                .join(",");
            let json = format!(
                "{{\"schema_version\":1,\"case\":\"{}\",\"registry_index\":{index},\"backend\":\"volatile\",\"execution\":\"in_process\",\"font_sha256\":\"{FONT_HASH}\",\"api_sha256\":\"{}\",\"test_sha256\":\"{}\",\"support_sha256\":\"{}\",\"exchanges\":[{exchanges}],\"frames\":[{}],\"observations\":[{}],\"mutation_dispatch_count\":{},\"permit_count\":{},\"transition_count\":{},\"ledger\":{{\"sessions\":{},\"selected_objects\":{},\"instances\":{},\"endpoints\":{},\"shared_objects\":{},\"queued_keys\":{},\"operations_per_object\":[{ops}]}}}}",
                self.case,
                hash(include_bytes!(
                    "../../../../docs/DESKTOP_EDITOR_HOST_API_V0.md"
                )),
                hash(include_bytes!("../editor_host.rs")),
                hash(include_bytes!("editor_host_assertions.rs")),
                frames.join(","),
                self.observations.join(","),
                e.mutation_dispatch_count,
                e.permit_count,
                e.transition_count,
                e.ledger.sessions,
                e.ledger.selected_objects,
                e.ledger.instances,
                e.ledger.endpoints,
                e.ledger.shared_objects,
                e.ledger.queued_keys
            );
            std::fs::write(dir.join(format!("registry-{index}.json")), json).unwrap();
        }
    }
}
fn raw_wire(e: &Envelope) -> [u8; 88] {
    // Preserve actual malformed Envelope probes without normalizing payload data.
    let mut b = [0; 88];
    b[..4].copy_from_slice(&e.protocol.to_le_bytes());
    b[4..8].copy_from_slice(&e.msg_type.to_le_bytes());
    b[8..16].copy_from_slice(&e.handle.pack().to_le_bytes());
    b[16..20].copy_from_slice(&e.payload_len.to_le_bytes());
    b[20..84].copy_from_slice(&e.payload);
    b
}
