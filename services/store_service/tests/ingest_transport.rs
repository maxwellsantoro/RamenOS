//! Real host transport regression. Access/signature policy is explicitly dev-only;
//! capability enforcement is covered by the signed handler boundary tests.
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use store_service::capability::{STORE_RIGHT_WRITE, StoreCapability};

struct Service(Child);
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_service(
    temp: &tempfile::TempDir,
    socket: &std::path::Path,
    root: &std::path::Path,
) -> Service {
    let mut service = Service(
        Command::new(env!("CARGO_BIN_EXE_store_service"))
            .env("RAMEN_STORE_SOCKET", socket)
            .env("RAMEN_STORE_ROOT", root)
            .env("RAMEN_STORE_AUDIT_LOG", temp.path().join("audit.jsonl"))
            .env("RAMEN_STORE_DEV_MODE", "1")
            .env("RAMEN_STORE_INGEST_MAX_BYTES", "1024")
            .env("RAMEN_STORE_REQUEST_TIMEOUT_MS", "500")
            .env("RAMEN_STORE_MAX_CONNECTIONS", "4")
            .env("RAMEN_STORE_ACCESS_POLICY", "AllowAll")
            .env_remove("RAMEN_STORE_TRUSTED_KEYS")
            .env_remove("RAMEN_STORE_PROJECTION_INDEX")
            .env_remove("RAMEN_STORE_CAP_TRUSTED_KEYS")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(
            service.0.try_wait().unwrap().is_none(),
            "Store exited before binding"
        );
        assert!(Instant::now() < until, "Store did not bind");
        std::thread::sleep(Duration::from_millis(10));
    }
    service
}

#[test]
fn review_ingest_transport_preserves_client_api_and_denies_legacy_paths() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("store.sock");
    let root = temp.path().join("cas");
    let _service = start_service(&temp, &socket, &root);
    let source = temp.path().join("client-source");
    let bytes = b"caller file through the actual client";
    std::fs::write(&source, bytes).unwrap();
    let cap = StoreCapability::new(7, STORE_RIGHT_WRITE, 99);
    let mut client =
        store_service::StoreClient::connect_with_capability(&socket, 7, Some(cap.clone())).unwrap();
    let reply = client.ingest_artifact("config", "test", &source).unwrap();
    assert_eq!(reply.content_id, artifact_store_core::hash_bytes(bytes));
    let id = artifact_store_schema::ContentId::parse(&reply.content_id).unwrap();
    assert_eq!(
        std::fs::read(root.join(format!("{}.blob", id.hash_hex()))).unwrap(),
        bytes
    );

    let private = temp.path().join("private-sentinel");
    std::fs::write(&private, b"unscoped secret").unwrap();
    let request = store_service::client::IngestArtifactRequest {
        request_id: 2,
        kind: "config".into(),
        channel: "test".into(),
        src_path: private.display().to_string(),
        capability_bytes: bincode::serialize(&cap).unwrap(),
    };
    let mut legacy = UnixStream::connect(&socket).unwrap();
    legacy
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut frame = vec![4];
    frame.extend_from_slice(&bincode::serialize(&request).unwrap());
    store_service::frame::write_message(&mut legacy, &frame).unwrap();
    assert_eq!(
        legacy.read(&mut [0; 1]).unwrap(),
        0,
        "legacy path request must close without dispatch"
    );
    let secret_id = artifact_store_schema::ContentId::parse(&artifact_store_core::hash_bytes(
        b"unscoped secret",
    ))
    .unwrap();
    assert!(!root.join(format!("{}.blob", secret_id.hash_hex())).exists());

    // New-message path labels without an attached fd also cannot authorize a read.
    let mut missing = UnixStream::connect(&socket).unwrap();
    missing
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    frame[0] = 7;
    store_service::frame::write_message(&mut missing, &frame).unwrap();
    missing.write_all(&[0xfd]).unwrap();
    assert_eq!(missing.read(&mut [0; 1]).unwrap(), 0);
    assert!(!root.join(format!("{}.blob", secret_id.hash_hex())).exists());
}

#[test]
fn stalled_descriptor_and_oversize_ingest_do_not_monopolize_reads_or_publish() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("store.sock");
    let root = temp.path().join("cas");
    let _service = start_service(&temp, &socket, &root);
    let cap = StoreCapability::new(7, store_service::capability::STORE_RIGHT_ALL, 99);
    let source = temp.path().join("source");
    std::fs::write(&source, b"readable").unwrap();
    let mut client =
        store_service::StoreClient::connect_with_capability(&socket, 7, Some(cap.clone())).unwrap();
    let known = client.ingest_artifact("config", "test", &source).unwrap();
    let mut stalled = UnixStream::connect(&socket).unwrap();
    stalled
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let request = store_service::client::IngestArtifactRequest {
        request_id: 77,
        kind: "config".into(),
        channel: "test".into(),
        src_path: source.display().to_string(),
        capability_bytes: bincode::serialize(&cap).unwrap(),
    };
    let mut frame = vec![7];
    frame.extend_from_slice(&bincode::serialize(&request).unwrap());
    store_service::frame::write_message(&mut stalled, &frame).unwrap();
    // Client A omits SCM_RIGHTS. Client B must read before A's deadline.
    let began = Instant::now();
    assert_eq!(
        std::fs::read(client.get_blob(&known.content_id).unwrap().blob_path).unwrap(),
        b"readable"
    );
    assert!(began.elapsed() < Duration::from_millis(500));
    assert_eq!(stalled.read(&mut [0; 1]).unwrap(), 0);
    let mut client =
        store_service::StoreClient::connect_with_capability(&socket, 7, Some(cap)).unwrap();
    // An authorized descriptor with too many bytes must fail without CAS or staging debris.
    std::fs::write(&source, vec![1u8; 1025]).unwrap();
    assert!(client.ingest_artifact("config", "test", &source).is_err());
    assert_eq!(
        std::fs::read(client.get_blob(&known.content_id).unwrap().blob_path).unwrap(),
        b"readable"
    );
    assert_eq!(
        std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".ingest-"))
            .count(),
        0
    );
    let oversized =
        artifact_store_schema::ContentId::parse(&artifact_store_core::hash_bytes(&vec![1u8; 1025]))
            .unwrap();
    assert!(!root.join(format!("{}.blob", oversized.hash_hex())).exists());
    assert!(
        !root
            .join(format!("{}.manifest.json", oversized.hash_hex()))
            .exists()
    );
}

#[test]
fn retained_client_recovers_after_server_idle_expiry_before_reads_and_ingestion() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("store.sock");
    let root = temp.path().join("cas");
    let _service = start_service(&temp, &socket, &root);
    let source = temp.path().join("source");
    std::fs::write(&source, b"first artifact").unwrap();
    let cap = StoreCapability::new(7, store_service::capability::STORE_RIGHT_ALL, 99);
    let mut client =
        store_service::StoreClient::connect_with_capability(&socket, 7, Some(cap)).unwrap();
    let first = client.ingest_artifact("config", "test", &source).unwrap();
    // The real server's fixture idle budget is 500ms.
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(
        std::fs::read(client.get_blob(&first.content_id).unwrap().blob_path).unwrap(),
        b"first artifact"
    );
    std::thread::sleep(Duration::from_millis(800));
    std::fs::write(&source, b"second artifact").unwrap();
    let second = client.ingest_artifact("config", "test", &source).unwrap();
    assert_eq!(
        second.content_id,
        artifact_store_core::hash_bytes(b"second artifact")
    );
    assert_eq!(
        std::fs::read(client.get_blob(&second.content_id).unwrap().blob_path).unwrap(),
        b"second artifact"
    );
}
