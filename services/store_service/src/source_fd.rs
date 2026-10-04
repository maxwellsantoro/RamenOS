//! Host-only source transport. A pathname is a display label, never read authority.
//! Native ingestion remains the IDL src_shm_cap/src_len contract.
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;

const MARKER: u8 = 0xfd;

pub fn send(stream: &mut UnixStream, file: &File) -> io::Result<()> {
    let mut marker = MARKER;
    let mut iov = libc::iovec {
        iov_base: (&mut marker as *mut u8).cast(),
        iov_len: 1,
    };
    // usize storage gives cmsghdr its required alignment; only CMSG_SPACE is used.
    let mut control = [0usize; 16];
    // SAFETY: zeroed msghdr has valid null pointers for unused fields.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    // SAFETY: a single RawFd fits the allocated, aligned buffer.
    msg.msg_controllen = unsafe { libc::CMSG_SPACE(std::mem::size_of::<libc::c_int>() as _) } as _;
    // SAFETY: msg/control/iov stay live through sendmsg; SCM_RIGHTS copies the fd.
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&msg);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<libc::c_int>() as _) as _;
        std::ptr::write_unaligned(
            libc::CMSG_DATA(header).cast::<libc::c_int>(),
            file.as_raw_fd(),
        );
    }
    loop {
        #[cfg(target_os = "linux")]
        let flags = libc::MSG_NOSIGNAL;
        #[cfg(not(target_os = "linux"))]
        let flags = 0;
        // SAFETY: all pointers reference initialized live buffers as above.
        let count = unsafe { libc::sendmsg(stream.as_raw_fd(), &msg, flags) };
        if count == 1 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if count < 0 && error.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(error);
    }
}

pub fn receive(stream: &UnixStream) -> io::Result<File> {
    let mut marker = 0u8;
    let mut iov = libc::iovec {
        iov_base: (&mut marker as *mut u8).cast(),
        iov_len: 1,
    };
    let mut control = [0usize; 32];
    // SAFETY: zeroed msghdr has valid null pointers for unused fields.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = std::mem::size_of_val(&control) as _;
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_CMSG_CLOEXEC;
    #[cfg(not(target_os = "linux"))]
    let flags = 0;
    let count = loop {
        // SAFETY: all pointers refer to writable live buffers, with exact lengths.
        let count = unsafe { libc::recvmsg(stream.as_raw_fd(), &mut msg, flags) };
        if count >= 0 {
            break count;
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    };
    let mut descriptors = Vec::new();
    let mut unexpected = false;
    // SAFETY: CMSG traversal is bounded by the kernel-returned control length.
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&msg);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let length =
                    ((*header).cmsg_len as usize).saturating_sub(libc::CMSG_LEN(0) as usize);
                for index in 0..length / std::mem::size_of::<libc::c_int>() {
                    let raw = std::ptr::read_unaligned(
                        libc::CMSG_DATA(header).cast::<libc::c_int>().add(index),
                    );
                    descriptors.push(OwnedFd::from_raw_fd(raw));
                }
            } else {
                unexpected = true;
            }
            header = libc::CMSG_NXTHDR(&msg, header);
        }
    }
    if count != 1
        || marker != MARKER
        || unexpected
        || msg.msg_flags & libc::MSG_CTRUNC != 0
        || descriptors.len() != 1
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "exactly one source descriptor required",
        ));
    }
    let fd = descriptors.pop().unwrap();
    // SAFETY: the received descriptor is owned here; fcntl sets close-on-exec.
    if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(File::from(fd))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn review_source_descriptor_survives_path_replacement() {
        use std::io::Read;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source");
        std::fs::write(&path, b"caller opened bytes").unwrap();
        let file = File::open(&path).unwrap();
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        send(&mut sender, &file).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"service private bytes").unwrap();
        let mut bytes = Vec::new();
        receive(&receiver).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"caller opened bytes");
    }
    #[test]
    fn review_source_descriptor_is_required() {
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        sender.write_all(&[MARKER]).unwrap();
        assert!(receive(&receiver).is_err());
    }

    #[test]
    fn review_source_transport_rejects_extra_descriptors() {
        let source = tempfile::tempfile().unwrap();
        let (sender, receiver) = UnixStream::pair().unwrap();
        let mut marker = MARKER;
        let mut iov = libc::iovec {
            iov_base: (&mut marker as *mut u8).cast(),
            iov_len: 1,
        };
        let mut control = [0usize; 16];
        // SAFETY: initialized aligned buffers hold exactly two descriptor slots.
        unsafe {
            let mut msg: libc::msghdr = std::mem::zeroed();
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = control.as_mut_ptr().cast();
            msg.msg_controllen =
                libc::CMSG_SPACE((2 * std::mem::size_of::<libc::c_int>()) as _) as _;
            let header = libc::CMSG_FIRSTHDR(&msg);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN((2 * std::mem::size_of::<libc::c_int>()) as _) as _;
            let data = libc::CMSG_DATA(header).cast::<libc::c_int>();
            std::ptr::write_unaligned(data, source.as_raw_fd());
            std::ptr::write_unaligned(data.add(1), source.as_raw_fd());
            assert_eq!(libc::sendmsg(sender.as_raw_fd(), &msg, 0), 1);
        }
        assert!(receive(&receiver).is_err());
    }
}
