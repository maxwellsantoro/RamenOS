//! Host StoreClient lifecycle regressions; fake peers make dispatch and lost
//! replies observable without granting publication authority.
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use store_service::client::{GetBlobReply, GetBlobRequest};
use store_service::{StoreClient, frame};

fn read_request(stream: &mut UnixStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    frame::read_message(stream).unwrap()
}

fn reply_to_read(stream: &mut UnixStream, request: &[u8]) {
    assert_eq!(
        request[0], 2,
        "an uncertain ingestion must never be replayed"
    );
    let request: GetBlobRequest = bincode::deserialize(&request[1..]).unwrap();
    let reply = GetBlobReply {
        request_id: request.request_id,
        status: 0,
        blob_path: "recovered".into(),
    };
    frame::write_message(stream, &bincode::serialize(&reply).unwrap()).unwrap();
}

#[test]
fn initial_and_reconnected_response_waits_are_bounded_for_reads_and_ingestion() {
    for ingest in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let socket = temp.path().join("store.sock");
        let source = temp.path().join("source");
        std::fs::write(&source, b"candidate").unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        let (release, released) = mpsc::channel();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                assert_eq!(request[0], if ingest { 7 } else { 2 });
                if ingest {
                    let _source = store_service::source_fd::receive(&stream).unwrap();
                }
                // A watchdog bounds the regression even with a broken client.
                let _ = released.recv_timeout(Duration::from_secs(1));
            }
        });
        let mut client =
            StoreClient::connect_with_timeout(&socket, Duration::from_millis(100)).unwrap();
        for _ in 0..2 {
            let began = Instant::now();
            let result = if ingest {
                client
                    .ingest_artifact("config", "test", &source)
                    .map(|_| ())
            } else {
                client.get_blob("sha256:unused").map(|_| ())
            };
            let elapsed = began.elapsed();
            release.send(()).unwrap();
            assert!(result.is_err());
            assert!(
                elapsed < Duration::from_millis(400),
                "response wait took {elapsed:?}"
            );
        }
        drop(client);
        server.join().unwrap();
    }
}

#[test]
fn failed_transport_is_discarded_and_uncertain_ingestion_is_not_replayed() {
    for (ingest, broken_reply) in [false, true]
        .into_iter()
        .flat_map(|ingest| (0..3).map(move |reply| (ingest, reply)))
    {
        let temp = tempfile::tempdir().unwrap();
        let socket = temp.path().join("store.sock");
        let source = temp.path().join("source");
        std::fs::write(&source, b"candidate").unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        let (dispatched, dispatch) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert_eq!(request[0], if ingest { 7 } else { 2 });
            if ingest {
                let _source = store_service::source_fd::receive(&stream).unwrap();
            }
            dispatched.send(()).unwrap();
            // A reply is lost after the complete request (including its source)
            // was received. This cannot authorize automatic mutation retry.
            match broken_reply {
                0 => {} // Entire reply lost.
                1 => stream.write_all(&u32::MAX.to_le_bytes()).unwrap(),
                2 => stream.write_all(&[8, 0, 0, 0, 1]).unwrap(),
                _ => unreachable!(),
            }
            drop(stream);
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            reply_to_read(&mut stream, &request);
        });
        let mut client =
            StoreClient::connect_with_timeout(&socket, Duration::from_millis(200)).unwrap();
        let result = if ingest {
            client
                .ingest_artifact("config", "test", &source)
                .map(|_| ())
        } else {
            client.get_blob("sha256:unused").map(|_| ())
        };
        assert!(result.is_err());
        dispatch.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(
            client.get_blob("sha256:unused").unwrap().blob_path,
            "recovered"
        );
        server.join().unwrap();
    }
}

#[test]
fn response_trickles_cannot_reset_the_client_deadline() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("store.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_request(&mut stream);
        for byte in [8, 0, 0, 0, 1, 2, 3, 4] {
            if stream.write_all(&[byte]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(70));
        }
    });
    let mut client =
        StoreClient::connect_with_timeout(&socket, Duration::from_millis(150)).unwrap();
    let began = Instant::now();
    assert!(client.get_blob("sha256:unused").is_err());
    assert!(began.elapsed() < Duration::from_millis(400));
    drop(client);
    server.join().unwrap();
}
