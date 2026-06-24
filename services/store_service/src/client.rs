// V-007 Phase 2: Store Service Client
//
// Client library for IPC communication with store service.
// Provides type-safe methods for all store operations.
//
// V-007 Phase 5: Enhanced with capability presentation

use crate::capability::{STORE_RIGHT_READ, STORE_RIGHT_WRITE, StoreCapability};
use crate::frame;
use crate::status::*;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

// Message types
const MSG_GET_MANIFEST: u8 = 1;
const MSG_GET_BLOB: u8 = 2;
const MSG_VERIFY_ARTIFACT: u8 = 3;
const MSG_INGEST_ARTIFACT: u8 = 4;
const MSG_QUERY_PROJECTION_BY_PATH: u8 = 5;
const MSG_QUERY_PROJECTION_BY_TAG: u8 = 6;

// Default timeouts
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

// Request/response types
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct GetManifestRequest {
    pub request_id: u64,
    pub content_id: String,
    /// Capability bytes (serialized StoreCapability)
    pub capability_bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct GetManifestReply {
    pub request_id: u64,
    pub status: u32,
    pub schema_version: u32,
    pub content_id: String,
    pub size_bytes: u64,
    pub kind: String,
    pub channels: String,
    pub signatures: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct GetBlobRequest {
    pub request_id: u64,
    pub content_id: String,
    /// Capability bytes (serialized StoreCapability)
    pub capability_bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct GetBlobReply {
    pub request_id: u64,
    pub status: u32,
    pub blob_path: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct VerifyArtifactRequest {
    pub request_id: u64,
    pub content_id: String,
    /// Capability bytes (serialized StoreCapability)
    pub capability_bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct VerifyArtifactReply {
    pub request_id: u64,
    pub status: u32,
    pub valid: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct IngestArtifactRequest {
    pub request_id: u64,
    pub kind: String,
    pub channel: String,
    pub src_path: String,
    /// Capability bytes (serialized StoreCapability)
    pub capability_bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct IngestArtifactReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id: String,
    pub size_bytes: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct QueryProjectionByPathRequest {
    pub request_id: u64,
    pub path: String,
    pub capability_bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct QueryProjectionByPathReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct QueryProjectionByTagRequest {
    pub request_id: u64,
    pub tag: String,
    pub capability_bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct QueryProjectionByTagReply {
    pub request_id: u64,
    pub status: u32,
    pub content_ids: String,
}

/// Store service client error types
#[derive(Debug, thiserror::Error)]
pub enum StoreClientError {
    #[error("connection failed: {0}")]
    ConnectionFailed(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    SerializationFailed(#[from] bincode::Error),

    #[error("framing failed: {0}")]
    FramingFailed(String),

    #[error("store service error: status={status}, message={message}")]
    ServiceError { status: u32, message: String },

    #[error("timeout after {0:?}")]
    Timeout(Duration),

    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

/// Store service client for IPC communication
///
/// # Example
///
/// ```no_run
/// use store_service::StoreClient;
///
/// let mut client = StoreClient::connect("/path/to/socket.sock").unwrap();
/// let reply = client.get_manifest("sha256:...").unwrap();
/// ```
pub struct StoreClient {
    socket_path: PathBuf,
    stream: Option<UnixStream>,
    timeout: Duration,
    next_request_id: u64,

    // V-007 Phase 5: Capability presentation
    /// Domain ID this client is representing
    pub domain_id: u64,

    /// Capability to present with requests (optional for now)
    pub capability: Option<StoreCapability>,
}

impl StoreClient {
    /// Connect to store service at the given socket path
    ///
    /// Uses default timeout of 30 seconds for read operations.
    /// Defaults to domain 0 (kernel) with no capability.
    pub fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self, StoreClientError> {
        Self::connect_with_timeout_and_capability(socket_path, DEFAULT_READ_TIMEOUT, 0, None)
    }

    /// Connect with custom timeout
    pub fn connect_with_timeout<P: AsRef<Path>>(
        socket_path: P,
        timeout: Duration,
    ) -> Result<Self, StoreClientError> {
        Self::connect_with_timeout_and_capability(socket_path, timeout, 0, None)
    }

    /// Connect with domain ID (V-007 Phase 5)
    ///
    /// # Arguments
    /// * `socket_path` - Path to Unix domain socket
    /// * `domain_id` - Domain ID this client represents
    pub fn connect_with_domain<P: AsRef<Path>>(
        socket_path: P,
        domain_id: u64,
    ) -> Result<Self, StoreClientError> {
        Self::connect_with_timeout_and_capability(
            socket_path,
            DEFAULT_READ_TIMEOUT,
            domain_id,
            None,
        )
    }

    /// Connect with capability (V-007 Phase 5)
    ///
    /// # Arguments
    /// * `socket_path` - Path to Unix domain socket
    /// * `domain_id` - Domain ID this client represents
    /// * `capability` - Optional capability to present
    pub fn connect_with_capability<P: AsRef<Path>>(
        socket_path: P,
        domain_id: u64,
        capability: Option<StoreCapability>,
    ) -> Result<Self, StoreClientError> {
        Self::connect_with_timeout_and_capability(
            socket_path,
            DEFAULT_READ_TIMEOUT,
            domain_id,
            capability,
        )
    }

    /// Connect with timeout, domain ID, and capability (V-007 Phase 5)
    ///
    /// # Arguments
    /// * `socket_path` - Path to Unix domain socket
    /// * `timeout` - Read timeout
    /// * `domain_id` - Domain ID this client represents
    /// * `capability` - Optional capability to present
    pub fn connect_with_timeout_and_capability<P: AsRef<Path>>(
        socket_path: P,
        timeout: Duration,
        domain_id: u64,
        capability: Option<StoreCapability>,
    ) -> Result<Self, StoreClientError> {
        let socket_path = socket_path.as_ref().to_path_buf();

        // Set socket timeout
        let stream = UnixStream::connect(&socket_path)?;

        Ok(Self {
            socket_path,
            stream: Some(stream),
            timeout,
            next_request_id: 1,
            domain_id,
            capability,
        })
    }

    /// Get manifest for a content ID
    ///
    /// V-007 Phase 5: Requires STORE_RIGHT_READ capability
    pub fn get_manifest(&mut self, content_id: &str) -> Result<GetManifestReply, StoreClientError> {
        // V-007 Phase 5: Check capability before sending request
        if let Some(ref cap) = self.capability {
            if !cap.has_right(STORE_RIGHT_READ) {
                return Err(StoreClientError::InvalidResponse(format!(
                    "Capability does not grant READ right for domain {}",
                    self.domain_id
                )));
            }
        }

        let request_id = self.next_request_id();
        // V-007 Phase 5: Serialize capability for request
        let capability_bytes = if let Some(ref cap) = self.capability {
            bincode::serialize(cap).unwrap_or_default()
        } else {
            vec![]
        };
        let request = GetManifestRequest {
            request_id,
            content_id: content_id.to_string(),
            capability_bytes,
        };

        let reply_bytes = self.send_request(MSG_GET_MANIFEST, &request)?;
        let reply: GetManifestReply = bincode::deserialize(&reply_bytes)?;
        self.validate_request_id(reply.request_id, request_id)?;
        self.ensure_status_ok("get_manifest", reply.status)?;

        Ok(reply)
    }

    /// Get blob path for a content ID
    ///
    /// V-007 Phase 5: Requires STORE_RIGHT_READ capability
    pub fn get_blob(&mut self, content_id: &str) -> Result<GetBlobReply, StoreClientError> {
        // V-007 Phase 5: Check capability before sending request
        if let Some(ref cap) = self.capability {
            if !cap.has_right(STORE_RIGHT_READ) {
                return Err(StoreClientError::InvalidResponse(format!(
                    "Capability does not grant READ right for domain {}",
                    self.domain_id
                )));
            }
        }

        let request_id = self.next_request_id();
        // V-007 Phase 5: Serialize capability for request
        let capability_bytes = if let Some(ref cap) = self.capability {
            bincode::serialize(cap).unwrap_or_default()
        } else {
            vec![]
        };
        let request = GetBlobRequest {
            request_id,
            content_id: content_id.to_string(),
            capability_bytes,
        };

        let reply_bytes = self.send_request(MSG_GET_BLOB, &request)?;
        let reply: GetBlobReply = bincode::deserialize(&reply_bytes)?;
        self.validate_request_id(reply.request_id, request_id)?;
        self.ensure_status_ok("get_blob", reply.status)?;

        Ok(reply)
    }

    /// Verify artifact integrity
    ///
    /// V-007 Phase 5: Requires STORE_RIGHT_READ capability
    pub fn verify_artifact(
        &mut self,
        content_id: &str,
    ) -> Result<VerifyArtifactReply, StoreClientError> {
        // V-007 Phase 5: Check capability before sending request
        if let Some(ref cap) = self.capability {
            if !cap.has_right(STORE_RIGHT_READ) {
                return Err(StoreClientError::InvalidResponse(format!(
                    "Capability does not grant READ right for domain {}",
                    self.domain_id
                )));
            }
        }

        let request_id = self.next_request_id();
        // V-007 Phase 5: Serialize capability for request
        let capability_bytes = if let Some(ref cap) = self.capability {
            bincode::serialize(cap).unwrap_or_default()
        } else {
            vec![]
        };
        let request = VerifyArtifactRequest {
            request_id,
            content_id: content_id.to_string(),
            capability_bytes,
        };

        let reply_bytes = self.send_request(MSG_VERIFY_ARTIFACT, &request)?;
        let reply: VerifyArtifactReply = bincode::deserialize(&reply_bytes)?;
        self.validate_request_id(reply.request_id, request_id)?;
        self.ensure_status_ok("verify_artifact", reply.status)?;

        Ok(reply)
    }

    /// Ingest a new artifact
    ///
    /// V-007 Phase 5: Requires STORE_RIGHT_WRITE capability
    pub fn ingest_artifact(
        &mut self,
        kind: &str,
        channel: &str,
        src_path: &Path,
    ) -> Result<IngestArtifactReply, StoreClientError> {
        // V-007 Phase 5: Check capability before sending request
        if let Some(ref cap) = self.capability {
            if !cap.has_right(STORE_RIGHT_WRITE) {
                return Err(StoreClientError::InvalidResponse(format!(
                    "Capability does not grant WRITE right for domain {}",
                    self.domain_id
                )));
            }
        }

        let request_id = self.next_request_id();
        // V-007 Phase 5: Serialize capability for request
        let capability_bytes = if let Some(ref cap) = self.capability {
            bincode::serialize(cap).unwrap_or_default()
        } else {
            vec![]
        };
        let request = IngestArtifactRequest {
            request_id,
            kind: kind.to_string(),
            channel: channel.to_string(),
            src_path: src_path.to_string_lossy().to_string(),
            capability_bytes,
        };

        let reply_bytes = self.send_request(MSG_INGEST_ARTIFACT, &request)?;
        let reply: IngestArtifactReply = bincode::deserialize(&reply_bytes)?;
        self.validate_request_id(reply.request_id, request_id)?;
        self.ensure_status_ok("ingest_artifact", reply.status)?;

        Ok(reply)
    }

    /// Query projection index by virtual path (S10.3).
    pub fn query_projection_by_path(
        &mut self,
        path: &str,
    ) -> Result<QueryProjectionByPathReply, StoreClientError> {
        let request_id = self.next_request_id();
        let capability_bytes = if let Some(cap) = &self.capability {
            bincode::serialize(cap).unwrap_or_default()
        } else {
            vec![]
        };
        let request = QueryProjectionByPathRequest {
            request_id,
            path: path.to_string(),
            capability_bytes,
        };

        let reply_bytes = self.send_request(MSG_QUERY_PROJECTION_BY_PATH, &request)?;
        let reply: QueryProjectionByPathReply = bincode::deserialize(&reply_bytes)?;
        self.validate_request_id(reply.request_id, request_id)?;
        self.ensure_status_ok("query_projection_by_path", reply.status)?;
        Ok(reply)
    }

    /// Query projection index by tag (S10.3).
    pub fn query_projection_by_tag(
        &mut self,
        tag: &str,
    ) -> Result<QueryProjectionByTagReply, StoreClientError> {
        let request_id = self.next_request_id();
        let capability_bytes = if let Some(cap) = &self.capability {
            bincode::serialize(cap).unwrap_or_default()
        } else {
            vec![]
        };
        let request = QueryProjectionByTagRequest {
            request_id,
            tag: tag.to_string(),
            capability_bytes,
        };

        let reply_bytes = self.send_request(MSG_QUERY_PROJECTION_BY_TAG, &request)?;
        let reply: QueryProjectionByTagReply = bincode::deserialize(&reply_bytes)?;
        self.validate_request_id(reply.request_id, request_id)?;
        self.ensure_status_ok("query_projection_by_tag", reply.status)?;
        Ok(reply)
    }

    /// Close the connection
    pub fn close(&mut self) -> Result<(), StoreClientError> {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        Ok(())
    }

    // Internal methods

    fn next_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = id.wrapping_add(1);
        id
    }

    fn ensure_connected(&mut self) -> Result<(), StoreClientError> {
        if self.stream.is_none() {
            let stream = UnixStream::connect(&self.socket_path)?;
            stream.set_read_timeout(Some(self.timeout))?;
            self.stream = Some(stream);
        }
        Ok(())
    }

    fn send_request<T: serde::Serialize>(
        &mut self,
        msg_type: u8,
        request: &T,
    ) -> Result<Vec<u8>, StoreClientError> {
        self.ensure_connected()?;

        // Serialize request
        let request_payload = bincode::serialize(request)?;

        // Format message: [msg_type: 1 byte] [request_payload: N bytes]
        let mut msg = vec![msg_type];
        msg.extend_from_slice(&request_payload);

        // Write to socket
        frame::write_message(self.stream.as_mut().unwrap(), &msg)
            .map_err(|e| StoreClientError::FramingFailed(e.to_string()))?;

        // Read reply
        let reply_payload = frame::read_message(self.stream.as_mut().unwrap())
            .map_err(|e| StoreClientError::FramingFailed(e.to_string()))?;

        Ok(reply_payload)
    }

    fn validate_request_id(&self, reply_id: u64, expected_id: u64) -> Result<(), StoreClientError> {
        if reply_id != expected_id {
            return Err(StoreClientError::InvalidResponse(format!(
                "request_id mismatch: expected {}, got {}",
                expected_id, reply_id
            )));
        }
        Ok(())
    }

    /// Validate that service reply status indicates success.
    /// Returns error with operation context if status is not OK.
    fn ensure_status_ok(&self, op: &str, status: u32) -> Result<(), StoreClientError> {
        match status {
            STATUS_OK => Ok(()),
            STATUS_NOT_FOUND => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: not found", op),
            }),
            STATUS_IO_ERROR => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: I/O error", op),
            }),
            STATUS_VALIDATION_FAILED => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: validation failed", op),
            }),
            STATUS_PERMISSION_DENIED => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: permission denied", op),
            }),
            _ => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: unknown error (status={})", op, status),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::StoreCapability;

    #[test]
    fn client_can_be_created() {
        // Note: This will fail to connect if socket doesn't exist,
        // but tests the struct can be created
        let client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 0,
            capability: None,
        };
        assert_eq!(client.next_request_id, 1);
        assert_eq!(client.domain_id, 0);
        assert!(client.capability.is_none());
    }

    #[test]
    fn request_ids_are_monotonic() {
        let mut client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 0,
            capability: None,
        };

        assert_eq!(client.next_request_id(), 1);
        assert_eq!(client.next_request_id(), 2);
        assert_eq!(client.next_request_id(), 3);
    }

    #[test]
    fn request_id_wraps_correctly() {
        let mut client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: u64::MAX,
            domain_id: 0,
            capability: None,
        };

        assert_eq!(client.next_request_id(), u64::MAX);
        assert_eq!(client.next_request_id(), 0); // Wraps to 0
    }

    // V-007 Phase 5: Capability presentation tests

    #[test]
    fn client_tracks_domain_id() {
        let client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 42,
            capability: None,
        };

        assert_eq!(client.domain_id, 42);
    }

    #[test]
    fn client_tracks_capability() {
        let cap = StoreCapability::new(5, STORE_RIGHT_READ | STORE_RIGHT_WRITE, 100);

        let client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 5,
            capability: Some(cap),
        };

        assert_eq!(client.domain_id, 5);
        assert!(client.capability.is_some());

        let cap_ref = client.capability.as_ref().unwrap();
        assert_eq!(cap_ref.domain_id, 5);
        assert!(cap_ref.has_right(STORE_RIGHT_READ));
        assert!(cap_ref.has_right(STORE_RIGHT_WRITE));
    }

    #[test]
    fn client_accepts_none_capability() {
        let client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 0,
            capability: None,
        };

        assert!(client.capability.is_none());
        // For now, None capability is allowed (will be enforced by service)
    }

    #[test]
    fn client_capability_matches_domain_id() {
        let cap = StoreCapability::new(10, STORE_RIGHT_READ, 200);

        let client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 10,
            capability: Some(cap),
        };

        assert_eq!(client.domain_id, 10);
        let cap_ref = client.capability.as_ref().unwrap();
        assert_eq!(cap_ref.domain_id, 10);
        assert!(cap_ref.is_for_domain(10));
        assert!(!cap_ref.is_for_domain(5));
    }

    #[test]
    fn client_capability_rights_check_works() {
        let read_only_cap = StoreCapability::new(7, STORE_RIGHT_READ, 300);

        let client = StoreClient {
            socket_path: PathBuf::from("/tmp/test.sock"),
            stream: None,
            timeout: Duration::from_secs(30),
            next_request_id: 1,
            domain_id: 7,
            capability: Some(read_only_cap),
        };

        let cap_ref = client.capability.as_ref().unwrap();
        assert!(cap_ref.has_right(STORE_RIGHT_READ));
        assert!(!cap_ref.has_right(STORE_RIGHT_WRITE));
    }
}
