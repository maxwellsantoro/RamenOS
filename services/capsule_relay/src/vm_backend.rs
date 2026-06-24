//! VM Backend — launches QEMU with virtio-serial and communicates via Unix socket.

use std::error::Error;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use kernel_api::cap::Handle;
use kernel_api::ipc::Envelope;

use crate::CapsuleBackend;

/// Fixed wire size for capsule relay envelope encoding.
/// This is independent from the in-memory size of `kernel_api::ipc::Envelope`.
const ENVELOPE_SIZE: usize = 88;

/// VM backend that spawns QEMU with virtio-serial.
pub struct VmBackend {
    qemu_child: Child,
    socket: UnixStream,
    #[allow(dead_code)]
    socket_path: PathBuf,
}

impl VmBackend {
    /// Spawn QEMU and connect to the virtio-serial socket.
    ///
    /// - `kernel_path`: Path to Linux kernel (bzImage)
    /// - `initrd_path`: Path to initrd with capsule_agent as /init
    /// - `socket_path`: Path for the Unix socket (will be created by QEMU)
    /// - `timeout`: How long to wait for socket to appear
    pub fn spawn(
        kernel_path: &Path,
        initrd_path: &Path,
        socket_path: &Path,
        timeout: Duration,
    ) -> Result<Self, Box<dyn Error>> {
        // Remove stale socket file
        let _ = std::fs::remove_file(socket_path);

        // Spawn QEMU with virtio-serial
        let mut qemu_child = spawn_qemu(kernel_path, initrd_path, socket_path)?;

        // Wait for QEMU to create the socket and connect
        let socket = wait_for_socket(socket_path, &mut qemu_child, timeout)?;
        socket.set_read_timeout(Some(Duration::from_secs(30)))?;
        socket.set_write_timeout(Some(Duration::from_secs(10)))?;

        Ok(Self {
            qemu_child,
            socket,
            socket_path: socket_path.to_path_buf(),
        })
    }
}

impl CapsuleBackend for VmBackend {
    fn call(&mut self, request: &Envelope) -> Result<Envelope, String> {
        // Serialize envelope to bytes
        let request_bytes = envelope_to_bytes(request);

        // Write to socket
        self.socket
            .write_all(&request_bytes)
            .map_err(|e| format!("socket write: {e}"))?;

        // Read reply
        let mut reply_bytes = [0u8; ENVELOPE_SIZE];
        self.socket
            .read_exact(&mut reply_bytes)
            .map_err(|e| format!("socket read: {e}"))?;

        Ok(bytes_to_envelope(&reply_bytes))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        // Close socket
        let _ = self.socket.shutdown(std::net::Shutdown::Both);

        // Kill QEMU if still running (check first to avoid kill on already-exited)
        match self.qemu_child.try_wait() {
            Ok(Some(_)) => {
                // Already exited
            }
            _ => {
                let _ = self.qemu_child.kill();
                let _ = self.qemu_child.wait();
            }
        }

        // Clean up socket file
        let _ = std::fs::remove_file(&self.socket_path);

        Ok(())
    }
}

impl Drop for VmBackend {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn spawn_qemu(kernel: &Path, initrd: &Path, socket: &Path) -> Result<Child, Box<dyn Error>> {
    let socket_str = socket.to_str().ok_or("socket path not valid UTF-8")?;

    println!("vm_backend: spawning QEMU...");
    println!("vm_backend:   kernel = {}", kernel.display());
    println!("vm_backend:   initrd = {}", initrd.display());
    println!("vm_backend:   socket = {}", socket.display());

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-machine").arg("q35");
    cmd.arg("-m").arg("512M");
    cmd.arg("-smp").arg("1");
    cmd.arg("-nographic");
    cmd.arg("-no-reboot");
    cmd.arg("-no-shutdown");
    cmd.arg("-kernel").arg(kernel);
    cmd.arg("-initrd").arg(initrd);

    // virtio-serial bus
    cmd.arg("-device").arg("virtio-serial-pci,id=vs0");

    // chardev: Unix socket, QEMU is server
    cmd.arg("-chardev").arg(format!(
        "socket,id=agent-chan,path={},server=on,wait=off",
        socket_str
    ));

    // virtserialport: port nr=1 → /dev/vport0p1 in guest
    cmd.arg("-device")
        .arg("virtserialport,chardev=agent-chan,name=com.ramen.agent,nr=1");

    // Kernel command line
    cmd.arg("-append").arg("console=ttyS0");

    // Detach stdio
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let child = cmd.spawn()?;
    println!("vm_backend: QEMU pid = {}", child.id());

    Ok(child)
}

fn wait_for_socket(
    path: &Path,
    qemu: &mut Child,
    timeout: Duration,
) -> Result<UnixStream, Box<dyn Error>> {
    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        if start.elapsed() > timeout {
            return Err(format!("socket connect timeout after {:?}", timeout).into());
        }

        // Check if QEMU crashed before socket was created
        if let Ok(Some(status)) = qemu.try_wait() {
            return Err(format!("QEMU exited early with status: {:?}", status).into());
        }

        match UnixStream::connect(path) {
            Ok(stream) => {
                println!(
                    "vm_backend: connected to socket after {:?}",
                    start.elapsed()
                );
                return Ok(stream);
            }
            Err(_) => {
                std::thread::sleep(poll_interval);
            }
        }
    }
}

/// Serialize Envelope to bytes using explicit field writes (no transmute).
///
/// Wire format is **little-endian** for cross-arch determinism (traces as spec).
/// Layout: protocol(4) + msg_type(4) + handle(8 packed) + payload_len(4) + payload(64) + pad(4) = 88
fn envelope_to_bytes(env: &Envelope) -> [u8; ENVELOPE_SIZE] {
    let mut buf = [0u8; ENVELOPE_SIZE];
    buf[0..4].copy_from_slice(&env.protocol.to_le_bytes());
    buf[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    buf[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    buf[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    buf[20..84].copy_from_slice(&env.payload);
    // bytes 84..88 are padding (zeroed)
    buf
}

/// Deserialize Envelope from bytes using explicit field reads (no transmute).
///
/// Wire format is **little-endian** for cross-arch determinism.
fn bytes_to_envelope(bytes: &[u8; ENVELOPE_SIZE]) -> Envelope {
    let protocol = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let msg_type = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let handle_raw = u64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    let payload_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let mut payload = [0u8; 64];
    payload.copy_from_slice(&bytes[20..84]);
    Envelope {
        protocol,
        msg_type,
        handle: Handle::unpack(handle_raw),
        payload_len,
        payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_size_is_88() {
        assert_eq!(ENVELOPE_SIZE, 88);
    }

    #[test]
    fn envelope_roundtrip() {
        let env = Envelope::empty(0x200, 1);
        let bytes = envelope_to_bytes(&env);
        let env2 = bytes_to_envelope(&bytes);
        assert_eq!(env.protocol, env2.protocol);
        assert_eq!(env.msg_type, env2.msg_type);
    }
}
