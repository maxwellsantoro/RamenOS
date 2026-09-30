//! Trusted host bridge over actual generated protocol-14 IPC. Never registered by default.
use crate::protocol::*;
use artifact_store_schema::agent_task::{
    ExecutionBudgetV0, TaskContractV0, TaskSnapshotV1, ValidationOutcomeV0,
};
use kernel_api::{
    agent_task_protocol::*,
    generated::agent_task_v1::*,
    ipc::Envelope,
    wire::{read_payload, write_payload},
};
use std::{
    fs::OpenOptions,
    io::{self, Read},
    os::unix::{fs::OpenOptionsExt, net::UnixStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use store_service::agent_task::{
    TaskFixture, TaskService, content_hash, exchange, serve_connection,
};
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "task adapter protocol failure")
}
fn env<T: Copy>(ty: u32, req: T) -> io::Result<Envelope> {
    let mut e = Envelope::empty(14, ty);
    write_payload(&mut e, &req).map_err(|_| invalid())?;
    Ok(e)
}
fn hash(b: &[u8]) -> String {
    format!("sha256:{}", hex::encode(content_hash(b)))
}
pub fn development_fixture() -> TaskFixture {
    let input = include_bytes!("../../agent_task/fixtures/config.json").to_vec();
    let schema = include_bytes!("../../agent_task/fixtures/schema.json").to_vec();
    let policy = include_bytes!("../../agent_task/fixtures/policy.json").to_vec();
    let notes = include_bytes!("../../agent_task/fixtures/notes.txt").to_vec();
    let validator = wat::parse_str(include_str!(
        "../../agent_task/fixtures/equality_validator.wat"
    ))
    .expect("development WAT");
    fixture(input, schema, policy, notes, validator)
}
fn fixture(
    input: Vec<u8>,
    schema: Vec<u8>,
    policy: Vec<u8>,
    notes: Vec<u8>,
    validator: Vec<u8>,
) -> TaskFixture {
    TaskFixture {
        contract: TaskContractV0 {
            schema_version: 1,
            task_id: 17,
            domain_id: 7,
            resource: "workspace:a/config".into(),
            schema_id: hash(&schema),
            policy_id: hash(&policy),
            validator_id: hash(&validator),
            execution_budget: ExecutionBudgetV0 {
                guest_ms: 1500,
                wall_ms: 2500,
                host_call_ms: 1000,
                max_diagnostics_bytes: 4096,
            },
        },
        resource_id: 1,
        input,
        schema,
        policy,
        notes,
        validator,
    }
}
pub fn fixture_from_directory(path: &Path) -> io::Result<TaskFixture> {
    fn read(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
        let mut f = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?;
        if !f.metadata()?.is_file() {
            return Err(invalid());
        }
        let mut b = Vec::new();
        (&mut f).take(limit + 1).read_to_end(&mut b)?;
        if b.is_empty() || b.len() as u64 > limit {
            return Err(invalid());
        }
        Ok(b)
    }
    Ok(fixture(
        read(&path.join("config.json"), 65536)?,
        read(&path.join("schema.json"), 65536)?,
        read(&path.join("policy.json"), 65536)?,
        read(&path.join("notes.txt"), 65536)?,
        read(&path.join("validator.wasm"), 1048576)?,
    ))
}
pub struct RtAdapter {
    service: Arc<Mutex<TaskService>>,
    stream: UnixStream,
    thread: Option<JoinHandle<()>>,
    domain: u64,
    bootstrap: Bootstrap,
}
impl RtAdapter {
    pub fn launch(
        root: &Path,
        fixture: TaskFixture,
        worker: PathBuf,
        domain: u64,
    ) -> io::Result<Self> {
        // Caller context comes from the trusted launcher, never model bytes.
        let task_id = fixture.contract.task_id;
        let resource = fixture.resource_id;
        let logical = fixture.contract.resource.clone();
        let service = Arc::new(Mutex::new(TaskService::open(root, fixture, worker)?));
        let bootstrap = Bootstrap {
            schema_version: 1,
            task_id: Decimal(task_id),
            policy_cap: Cap(service.lock().map_err(|_| invalid())?.policy_cap()),
            resources: [
                ResourceBinding {
                    resource: Resource(resource),
                    logical_resource: logical,
                },
                ResourceBinding {
                    resource: Resource(100),
                    logical_resource: "task:schema".into(),
                },
                ResourceBinding {
                    resource: Resource(101),
                    logical_resource: "task:notes".into(),
                },
            ],
        };
        let (stream, peer) = UnixStream::pair()?;
        let backend = service.clone();
        let thread = std::thread::spawn(move || {
            let _ = serve_connection(backend, domain, peer);
        });
        Ok(Self {
            service,
            stream,
            thread: Some(thread),
            domain,
            bootstrap,
        })
    }
    pub fn bootstrap(&self) -> Bootstrap {
        self.bootstrap.clone()
    }
    /// Trusted evaluator only; no model-facing operation can call this.
    pub fn evidence(&self) -> io::Result<Vec<u8>> {
        self.service.lock().map_err(|_| invalid())?.evidence()
    }
    pub fn mapping_count(&self) -> usize {
        self.service.lock().expect("service mutex").mapping_count()
    }
    fn mapping(&self, cap: u64, len: u32) -> Result<Vec<u8>, u32> {
        if cap == 0 {
            return if len == 0 {
                Ok(Vec::new())
            } else {
                Err(STATUS_IO)
            };
        }
        let mut s = self.service.lock().map_err(|_| STATUS_IO)?;
        let bytes = s.read_mapping(self.domain, cap);
        let released = s.release_mapping(self.domain, cap);
        // clean expired/revoked mappings even when they can no longer be released.
        s.mapping_count();
        let bytes = bytes?;
        released?;
        if bytes.len() != len as usize {
            return Err(STATUS_IO);
        }
        Ok(bytes)
    }
    fn snapshot(&self, cap: u64, len: u32) -> Result<ModelState, u32> {
        let b = self.mapping(cap, len)?;
        let s: TaskSnapshotV1 = serde_json::from_slice(&b).map_err(|_| STATUS_IO)?;
        if s.schema_version != 1
            || s.task_id != self.bootstrap.task_id.0
            || s.resource_id != self.bootstrap.resources[0].resource.0
            || s.input_resources != [s.resource_id, 100, 101]
        {
            return Err(STATUS_IO);
        }
        let content = |s: String| Hash::new(s).map_err(|_| STATUS_IO);
        let validation = s
            .validation
            .map(|v| {
                Ok::<_, u32>(ModelValidation {
                    candidate_id: content(v.candidate_id)?,
                    schema_id: content(v.schema_id)?,
                    policy_id: content(v.policy_id)?,
                    validator_id: content(v.validator_id)?,
                    generation: Decimal(v.grant_generation),
                    valid_until_ms: Decimal(v.valid_until_ms),
                    wall_elapsed_ms: Decimal(v.wall_elapsed_ms),
                    guest_elapsed_ms: Decimal(v.guest_elapsed_ms),
                    diagnostics_bytes: Decimal(v.diagnostics_bytes),
                    diagnostics_truncated: v.diagnostics_truncated,
                    outcome: match v.outcome {
                        ValidationOutcomeV0::Valid => Outcome::Valid,
                        ValidationOutcomeV0::Invalid => Outcome::Invalid,
                        ValidationOutcomeV0::Timeout => Outcome::Timeout,
                        ValidationOutcomeV0::HostFailure => Outcome::HostFailure,
                    },
                })
            })
            .transpose()?;
        Ok(ModelState {
            task_id: Decimal(s.task_id),
            resource: Resource(s.resource_id),
            revision: Decimal(s.revision),
            content_id: content(s.content_id)?,
            generation: Decimal(s.grant_generation),
            rights: rights(s.granted_rights).map_err(|_| STATUS_IO)?,
            expires_at_ms: Decimal(s.grant_expires_at_ms),
            now_ms: Decimal(s.now_ms),
            schema_id: content(s.schema_id)?,
            policy_id: content(s.policy_id)?,
            validator_id: content(s.validator_id)?,
            input_resources: s.input_resources.map(Resource),
            validation,
            validation_current: s.validation_current,
        })
    }
    pub fn execute_json(&mut self, bytes: &[u8]) -> io::Result<Vec<u8>> {
        let response = match decode_request(bytes) {
            Ok(r) => self.execute(r)?,
            Err(_) => Response::error(None, Status::Invalid),
        };
        encode_response(&response).map_err(|_| invalid())
    }
    fn execute(&mut self, req: Request) -> io::Result<Response> {
        let id = req.request_id.0;
        let mut source = None;
        let request = match req.call {
            Call::ReadInput { task_cap, resource } => env(
                1,
                ReadInput {
                    task_cap: task_cap.0,
                    request_id: id,
                    resource_id: resource.0,
                },
            )?,
            Call::RequestGrant {
                policy_cap,
                task_id,
                resource,
                rights,
                lifetime_ms,
            } => env(
                3,
                RequestGrant {
                    policy_cap: policy_cap.0,
                    request_id: id,
                    task_id: task_id.0,
                    resource_id: resource.0,
                    lifetime_ms,
                    rights: rights.into_iter().fold(0, |b, r| b | r.bit()),
                },
            )?,
            Call::StageCandidate {
                task_cap,
                bytes_base64,
            } => {
                let cap = match self
                    .service
                    .lock()
                    .map_err(|_| invalid())?
                    .install_source(self.domain, bytes_base64.as_slice())
                {
                    Ok(cap) => cap,
                    Err(code) => {
                        return Ok(Response::error(
                            Some(req.request_id),
                            Status::from_native(code).ok_or_else(invalid)?,
                        ));
                    }
                };
                source = Some(cap);
                env(
                    5,
                    StageCandidate {
                        task_cap: task_cap.0,
                        request_id: id,
                        source_shm_cap: cap,
                        source_len: bytes_base64.as_slice().len() as u32,
                        reserved: 0,
                    },
                )?
            }
            Call::ValidateCandidate {
                task_cap,
                candidate_cap,
                validator_id,
            } => env(
                7,
                ValidateCandidate {
                    task_cap: task_cap.0,
                    request_id: id,
                    candidate_cap: candidate_cap.0,
                    validator_content_id_hash: validator_id.native(),
                },
            )?,
            Call::CommitCandidate {
                task_cap,
                candidate_cap,
                expected_revision,
                expected_content_id,
            } => env(
                9,
                CommitCandidate {
                    task_cap: task_cap.0,
                    request_id: id,
                    candidate_cap: candidate_cap.0,
                    expected_revision: expected_revision.0,
                    expected_content_id_hash: expected_content_id.native(),
                },
            )?,
            Call::GetReceipt {
                task_cap,
                commit_request_id,
            } => env(
                11,
                GetReceipt {
                    task_cap: task_cap.0,
                    request_id: id,
                    commit_request_id: commit_request_id.0,
                },
            )?,
            Call::GetTaskState { task_cap } => env(
                13,
                GetTaskState {
                    task_cap: task_cap.0,
                    request_id: id,
                },
            )?,
            Call::RevokeGrant {
                policy_cap,
                task_cap,
            } => env(
                15,
                RevokeGrant {
                    policy_cap: policy_cap.0,
                    request_id: id,
                    task_cap: task_cap.0,
                },
            )?,
        };
        let reply = exchange(&mut self.stream, &request);
        if let Some(cap) = source {
            let mut s = self.service.lock().map_err(|_| invalid())?;
            let _ = s.release_mapping(self.domain, cap);
            s.mapping_count();
        }
        let reply = reply?;
        let decode = || -> Result<(u32, Reply), u32> {
            macro_rules! payload {
                ($t:ty) => {
                    read_payload::<$t>(&reply).map_err(|_| STATUS_IO)?
                };
            }
            macro_rules! status {
                ($r:expr) => {
                    if $r.status != STATUS_OK {
                        return Err($r.status);
                    }
                };
            }
            Ok(match request.msg_type {
                1 => {
                    let r = payload!(ReadInputReply);
                    status!(r);
                    let b = self.mapping(r.input_shm_cap, r.input_len)?;
                    if content_hash(&b) != r.content_id_hash {
                        return Err(STATUS_IO);
                    }
                    (
                        r.status,
                        Reply::ReadInput {
                            content_id: Hash::from_bytes(r.content_id_hash),
                            bytes_base64: Bytes::new(b).map_err(|_| STATUS_IO)?,
                        },
                    )
                }
                3 => {
                    let r = payload!(RequestGrantReply);
                    status!(r);
                    (
                        r.status,
                        Reply::RequestGrant {
                            task_cap: Cap(r.task_cap),
                            generation: Decimal(r.generation),
                            expires_at_ms: Decimal(r.expires_at_ms),
                            rights: rights(r.rights).map_err(|_| STATUS_IO)?,
                        },
                    )
                }
                5 => {
                    let r = payload!(StageCandidateReply);
                    status!(r);
                    (
                        r.status,
                        Reply::StageCandidate {
                            candidate_cap: Cap(r.candidate_cap),
                            content_id: Hash::from_bytes(r.content_id_hash),
                        },
                    )
                }
                7 => {
                    let r = payload!(ValidateCandidateReply);
                    if r.status == STATUS_DENIED
                        || r.status == STATUS_EXPIRED
                        || r.outcome == OUTCOME_NOT_RUN
                    {
                        return Err(r.status);
                    }
                    (
                        r.status,
                        Reply::ValidateCandidate {
                            outcome: Outcome::from_native(r.outcome).ok_or(STATUS_IO)?,
                            valid_until_ms: Decimal(r.valid_until_ms),
                            diagnostics_base64: Bytes::new(
                                self.mapping(r.diagnostics_shm_cap, r.diagnostics_len)?,
                            )
                            .map_err(|_| STATUS_IO)?,
                            truncated: r.diagnostics_flags & DIAGNOSTICS_TRUNCATED != 0,
                        },
                    )
                }
                9 => {
                    let r = payload!(CommitCandidateReply);
                    status!(r);
                    (
                        r.status,
                        Reply::CommitCandidate {
                            receipt_cap: Cap(r.receipt_cap),
                            revision: Decimal(r.revision),
                            content_id: Hash::from_bytes(r.content_id_hash),
                        },
                    )
                }
                11 => {
                    let r = payload!(GetReceiptReply);
                    status!(r);
                    (
                        r.status,
                        Reply::GetReceipt {
                            commit_request_id: Decimal(r.commit_request_id),
                            revision: Decimal(r.revision),
                            content_id: Hash::from_bytes(r.content_id_hash),
                        },
                    )
                }
                13 => {
                    let r = payload!(GetTaskStateReply);
                    status!(r);
                    (
                        r.status,
                        Reply::GetTaskState {
                            state: Box::new(self.snapshot(r.state_shm_cap, r.state_len)?),
                        },
                    )
                }
                15 => {
                    let r = payload!(RevokeGrantReply);
                    status!(r);
                    (
                        r.status,
                        Reply::RevokeGrant {
                            generation: Decimal(r.generation),
                            revoked_count: r.revoked_count,
                        },
                    )
                }
                _ => return Err(STATUS_IO),
            })
        };
        Ok(match decode() {
            Ok((status, result)) => Response {
                schema_version: 1,
                request_id: Some(req.request_id),
                status: Status::from_native(status).ok_or_else(invalid)?,
                result: Some(result),
            },
            Err(status) => Response::error(
                Some(req.request_id),
                Status::from_native(status).ok_or_else(invalid)?,
            ),
        })
    }
}
impl Drop for RtAdapter {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
