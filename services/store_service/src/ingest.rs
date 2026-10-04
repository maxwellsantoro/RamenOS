//! Stage a single source read, binding the published CAS name to the copied bytes.
use artifact_store_schema::ContentId;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Reservation persists through uncertain worker termination, bounding both
/// concurrent preparation and aggregate staging bytes (slots * byte ceiling).
pub struct Admission {
    active: Arc<AtomicUsize>,
    limit: usize,
}
pub struct Permit(Arc<AtomicUsize>);
impl Admission {
    pub fn new(limit: usize) -> Self {
        Self {
            active: Arc::new(AtomicUsize::new(0)),
            limit,
        }
    }
    pub fn acquire(&self) -> Option<Permit> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.limit).then_some(n + 1)
            })
            .ok()?;
        Some(Permit(self.active.clone()))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CopyReceipt {
    content_id: String,
    size_bytes: u64,
}

fn copy_bytes(input: &mut impl Read, output: &mut File, max_bytes: u64) -> io::Result<CopyReceipt> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut size_bytes = 0u64;
    loop {
        // Read one excess byte to distinguish exact-limit EOF from growth.
        let capacity = max_bytes
            .saturating_sub(size_bytes)
            .saturating_add(1)
            .min(buffer.len() as u64) as usize;
        let len = match input.read(&mut buffer[..capacity]) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if len == 0 {
            break;
        }
        size_bytes = size_bytes
            .checked_add(len as u64)
            .filter(|size| *size <= max_bytes)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "artifact byte limit exceeded")
            })?;
        output.write_all(&buffer[..len])?;
        hash.update(&buffer[..len]);
    }
    output.sync_all()?;
    Ok(CopyReceipt {
        content_id: format!("sha256:{}", hex::encode(hash.finalize())),
        size_bytes,
    })
}

/// Internal same-binary child mode. The source and private output arrive as
/// inherited descriptors; no source pathname is opened by the service.
pub fn run_worker(max_bytes: u64) -> io::Result<()> {
    use std::os::fd::AsFd;
    let mut input = File::from(std::io::stdin().as_fd().try_clone_to_owned()?);
    if !input.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ingest requires a regular file",
        ));
    }
    let mut output = File::from(std::io::stdout().as_fd().try_clone_to_owned()?);
    let receipt = copy_bytes(&mut input, &mut output, max_bytes)?;
    serde_json::to_writer(std::io::stderr(), &receipt)?;
    Ok(())
}

/// Peek without consuming a future frame. EOF/transport failure cancels
/// preparation; a lost reply after publication still has ordinary IO uncertainty.
pub fn client_closed(stream: &std::os::unix::net::UnixStream) -> io::Result<bool> {
    use std::os::fd::AsRawFd;
    let mut byte = 0u8;
    loop {
        let mut descriptor = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        #[cfg(target_os = "linux")]
        {
            descriptor.events |= libc::POLLRDHUP;
        }
        // HUP detects a disconnected peer even with unread pipelined bytes.
        // SAFETY: one live pollfd, no blocking wait, and the socket is borrowed.
        if unsafe { libc::poll(&mut descriptor, 1, 0) } < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        let terminal = libc::POLLHUP | libc::POLLERR | libc::POLLNVAL;
        #[cfg(target_os = "linux")]
        let terminal = terminal | libc::POLLRDHUP;
        if descriptor.revents & terminal != 0 {
            return Ok(true);
        }
        // SAFETY: one live byte is writable and the borrowed socket stays open.
        let count = unsafe {
            libc::recv(
                stream.as_raw_fd(),
                (&mut byte as *mut u8).cast(),
                1,
                libc::MSG_PEEK | libc::MSG_DONTWAIT,
            )
        };
        if count >= 0 {
            return Ok(count == 0);
        }
        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::Interrupted => continue,
            io::ErrorKind::WouldBlock => return Ok(false),
            _ => return Err(error),
        }
    }
}

struct Worker {
    child: Option<Child>,
    staged: Option<StagedBlob>,
    permit: Option<Permit>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        // Only the acknowledged private process group is terminated.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let until = Instant::now() + Duration::from_millis(200);
        loop {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            if Instant::now() >= until {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        // A kernel-stalled process must not block the requesting thread, nor
        // release its reservation and allow unbounded replacement workers.
        let staged = self.staged.take();
        let permit = self.permit.take();
        if let Some(blob) = &staged {
            let _ = fs::remove_file(&blob.path);
        }
        eprintln!(
            "store_service: ingest worker {} termination pending; reservation retained",
            child.id()
        );
        std::thread::spawn(move || {
            while child.wait().is_err() {
                // Preserve uncertainty and its reservation until reaping succeeds.
                std::thread::sleep(Duration::from_secs(1));
            }
            drop(staged);
            drop(permit);
        });
    }
}

pub struct StagedBlob {
    path: PathBuf,
    pub content_id: ContentId,
    pub size_bytes: u64,
    _permit: Option<Permit>,
}

impl StagedBlob {
    #[cfg(test)]
    pub fn read(source: &Path, store_root: &Path) -> io::Result<Self> {
        // O_NONBLOCK prevents a replacement FIFO from blocking the open. Validate
        // the opened descriptor, not a prior path lookup; never follow symlinks.
        let input = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(source)?;
        Self::read_file(input, store_root)
    }

    #[cfg(test)]
    pub fn read_file(input: File, store_root: &Path) -> io::Result<Self> {
        Self::read_file_bounded(input, store_root, 1024 * 1024 * 1024)
    }

    #[cfg(test)]
    pub fn read_file_bounded(
        mut input: File,
        store_root: &Path,
        max_bytes: u64,
    ) -> io::Result<Self> {
        if !input.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ingest requires a regular file",
            ));
        }
        let (mut staged, mut output) = Self::create(store_root)?;
        let receipt = copy_bytes(&mut input, &mut output, max_bytes)?;
        staged.content_id = ContentId::parse(&receipt.content_id)
            .map_err(|error| io::Error::other(error.to_string()))?;
        staged.size_bytes = receipt.size_bytes;
        Ok(staged)
    }

    fn create(store_root: &Path) -> io::Result<(Self, File)> {
        fs::create_dir_all(store_root)?;
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let (path, output) = loop {
            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = store_root.join(format!(".ingest-{}-{serial}.tmp", std::process::id()));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => break (path, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        // Establish cleanup before any fallible copying, hashing or synchronization.
        let staged = Self {
            path,
            content_id: ContentId::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .expect("valid placeholder"),
            size_bytes: 0,
            _permit: None,
        };
        Ok((staged, output))
    }

    pub fn prepare(
        input: File,
        store_root: &Path,
        max_bytes: u64,
        timeout: Duration,
        executable: &Path,
        control: Option<(&std::os::unix::net::UnixStream, Permit)>,
    ) -> io::Result<Self> {
        let (client, permit) = match control {
            Some((client, permit)) => (Some(client), Some(permit)),
            None => (None, None),
        };
        let deadline = Instant::now() + timeout;
        let (staged, output) = Self::create(store_root)?;
        let child = Command::new(executable)
            .args(["--internal-ingest-worker", &max_bytes.to_string()])
            .process_group(0)
            .stdin(Stdio::from(input))
            .stdout(Stdio::from(output))
            .stderr(Stdio::piped())
            .spawn()?;
        let mut worker = Worker {
            child: Some(child),
            staged: Some(staged),
            permit,
        };
        loop {
            if client.is_some_and(|stream| client_closed(stream).unwrap_or(true)) {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "client cancelled preparation",
                ));
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "ingest preparation deadline exceeded",
                ));
            }
            if let Some(status) = worker.child.as_mut().unwrap().try_wait()? {
                let mut child = worker.child.take().unwrap();
                child.wait()?; // Returns the status cached by try_wait; already reaped.
                if !status.success() {
                    return Err(io::Error::other("ingest worker failed"));
                }
                // The trusted child emits one small receipt and no descendants.
                let mut bytes = Vec::new();
                child
                    .stderr
                    .take()
                    .unwrap()
                    .take(512)
                    .read_to_end(&mut bytes)?;
                let receipt: CopyReceipt = serde_json::from_slice(&bytes)?;
                if receipt.size_bytes > max_bytes {
                    return Err(io::Error::other("invalid ingest worker receipt"));
                }
                let mut staged = worker.staged.take().unwrap();
                staged.content_id = ContentId::parse(&receipt.content_id)
                    .map_err(|error| io::Error::other(error.to_string()))?;
                staged.size_bytes = receipt.size_bytes;
                staged._permit = worker.permit.take();
                return Ok(staged);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn publish(self, destination: &Path) -> io::Result<()> {
        fs::rename(&self.path, destination)?;
        if let Some(parent) = destination.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(())
    }
}

impl Drop for StagedBlob {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_ingest_commits_the_hashed_snapshot_after_source_changes() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::write(&source, b"original bytes").unwrap();
        let staged = StagedBlob::read(&source, root.path()).unwrap();
        let id = staged.content_id.clone();
        fs::write(&source, b"changed bytes").unwrap();
        let destination = root.path().join(format!("{}.blob", id.hash_hex()));
        staged.publish(&destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"original bytes");
        assert_eq!(
            artifact_store_core::hash_blob(&destination).unwrap(),
            id.as_str()
        );
    }

    #[test]
    fn review_ingest_rejects_fifo_and_symlink_without_blocking() {
        let root = tempfile::tempdir().unwrap();
        let fifo = root.path().join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(StagedBlob::read(&fifo, root.path()).is_err());
        let source = root.path().join("source");
        fs::write(&source, b"bytes").unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        assert!(StagedBlob::read(&link, root.path()).is_err());
    }

    #[test]
    fn review_ingest_staging_is_private_unique_and_cleaned_on_abort() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::write(&source, b"bytes").unwrap();
        let first = StagedBlob::read(&source, root.path()).unwrap();
        let second = StagedBlob::read(&source, root.path()).unwrap();
        assert_ne!(first.path, second.path);
        assert_eq!(
            fs::metadata(&first.path).unwrap().permissions().mode() & 0o077,
            0
        );
        let path = first.path.clone();
        drop(first);
        assert!(!path.exists());
        let second_path = second.path.clone();
        assert!(second.publish(&root.path().join("absent/blob")).is_err());
        assert!(
            !second_path.exists(),
            "failed publication cleans the staging file"
        );
    }
}

#[cfg(test)]
mod bound_tests {
    use super::*;
    #[test]
    fn growing_source_is_bounded_at_the_copy_and_failed_staging_is_removed() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::write(&source, b"12345").unwrap();
        assert!(
            StagedBlob::read_file_bounded(File::open(&source).unwrap(), root.path(), 4).is_err()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        let staged =
            StagedBlob::read_file_bounded(File::open(&source).unwrap(), root.path(), 5).unwrap();
        assert_eq!(staged.size_bytes, 5);
        assert_eq!(
            staged.content_id.as_str(),
            artifact_store_core::hash_bytes(b"12345")
        );
    }
    #[test]
    fn blocked_copy_worker_is_killed_reaped_and_staging_removed() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let worker = root.path().join("blocked-worker");
        // exec keeps the deliberate blocked work in the acknowledged process.
        fs::write(&worker, "#!/bin/sh\necho $$ > \"$0.pid\"\nexec sleep 30\n").unwrap();
        fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
        let input = tempfile::tempfile().unwrap();
        let start = std::time::Instant::now();
        let error = StagedBlob::prepare(
            input,
            root.path(),
            1024,
            std::time::Duration::from_millis(500),
            &worker,
            None,
        )
        .err()
        .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut, "{error}");
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        let pid: i32 = fs::read_to_string(worker.with_extension("pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert!(
            !fs::read_dir(root.path())
                .unwrap()
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().starts_with(".ingest-"))
        );
    }
}
