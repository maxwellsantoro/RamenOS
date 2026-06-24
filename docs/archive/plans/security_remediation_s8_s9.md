# Security Remediation Plan: S8/S9 Vulnerabilities

**Last Updated:** 2026-02-18

**Executive Summary**

Two independent security audits identified critical vulnerabilities in the S8 shared memory implementation and related services. This plan provides methodical remediation for all findings, organized by severity and architectural layer.

**Audit Sources:**
- Principal Systems Architect + Security Auditor review (Grade D)
- Second auditor review (Grade B-)

**Overall Risk Assessment:** CRITICAL - Multiple memory corruption and privilege escalation vectors

---

## Phase 1: Critical Memory Safety Fixes

### Issue 1.1: SHMEM Refcount Overflow → Use-After-Free (CRITICAL)

**Location:** `kernel/src/shmem.rs:293`
**Finding:** Unchecked `u32` refcount increment allows wraparound to zero, causing premature frame deallocation while mappings remain active.

**Evidence:**
```rust
// Increment refcount
slot.refcount += 1;  // Line 293 - unchecked!
```

**Exploit Scenario:**
1. Attacker repeatedly maps region 2³² times (~4 billion)
2. Refcount wraps from `u32::MAX` to 0
3. Attacker calls `close_region` - frees physical frames
4. Attacker still has valid mappings to freed frames
5. Kernel reallocates frames for page tables or other domains
6. Attacker corrupts newly allocated memory → privilege escalation

**Fix Required:**

1. Change `refcount: u32` to `refcount: u64` in `ShmemRegion` struct
2. Use checked arithmetic with explicit overflow cap
3. Add per-region maximum mapping limit

**Implementation:**

```rust
// In ShmemRegion struct (line ~74):
pub struct ShmemRegion {
    // ... existing fields ...
    pub refcount: u64,  // Changed from u32
    // ... rest of struct
}

// In map_region function (line ~293):
const MAX_REFCOUNT: u64 = u32::MAX as u64; // Prevent overflow

slot.refcount = slot.refcount.checked_add(1)
    .ok_or(STATUS_OVERFLOW)?;

// Add bounds check:
if slot.refcount > MAX_REFCOUNT {
    return Err(STATUS_INVALID_RIGHTS);
}
```

**Testing Requirements:**
- Unit test: verify overflow rejection at `u32::MAX + 1`
- Integration test: map/unmap 2³² times, verify refcount
- Gate test: `foundry-s8-s9-shmem-refcount-overflow`

---

### Issue 1.2: Ring Buffer Capacity TOCTOU → OOB Write (CRITICAL)

**Location:** `kernel_api/src/ring_buffer.rs:192, 206, 271, 312`
**Finding:** `capacity` field is read from shared memory on every operation, allowing untrusted producer/consumer to modify it and cause out-of-bounds kernel access.

**Evidence:**
```rust
// Line 192: trusts shared header
if data.len() > header.capacity as usize {
    return Err(WriteError::InvalidSize);
}

// Line 206: creates slice based on attacker-controlled capacity
let data_slice = unsafe { core::slice::from_raw_parts_mut(self.data, capacity) };
```

**Exploit Scenario:**
1. Attacker maps shared memory region containing ring buffer
2. Attacker overwrites `capacity` field at offset 16 with `u64::MAX`
3. Kernel calls `try_write` - passes size check
4. Kernel constructs slice covering entire address space starting at `self.data`
5. Kernel writes data, corrupting arbitrary kernel memory

**Fix Required:**

Cache `capacity` in kernel-private `RingBuffer` struct at initialization time. Never read from shared memory again.

**Implementation:**

```rust
// Modify RingBuffer struct (line ~117):
pub struct RingBuffer {
    header: *mut RingBufferHeader,
    data: *mut u8,
    capacity: usize,  // NEW: cached, immutable capacity
}

// Modify from_raw_parts (line ~147):
pub unsafe fn from_raw_parts(header: *mut RingBufferHeader, data: *mut u8) -> Self {
    assert!(!header.is_null(), "ring buffer header pointer must not be null");
    assert!(!data.is_null(), "ring buffer data pointer must not be null");

    // NEW: Read capacity ONCE from shared header
    let capacity = unsafe { (*header).capacity } as usize;
    assert!(capacity > 0, "ring buffer capacity must be greater than zero");
    assert!(capacity <= isize::MAX as u64, "ring buffer capacity exceeds maximum supported size");

    Self { header, data, capacity }  // Store cached capacity
}

// Modify try_write (line ~187):
pub fn try_write(&mut self, data: &[u8]) -> Result<(), WriteError> {
    // CHANGED: Use cached capacity, not header.capacity
    if data.len() > self.capacity {
        return Err(WriteError::InvalidSize);
    }

    let header = unsafe { &*self.header };
    let producer_head = header.producer_head.load(Ordering::Acquire);
    let consumer_head = header.consumer_head.load(Ordering::Acquire);
    let used = producer_head.saturating_sub(consumer_head) as usize;

    if data.len() > (self.capacity - used) {  // Use cached capacity
        return Err(WriteError::NoSpace);
    }

    // CHANGED: Use cached capacity
    let capacity = self.capacity;
    let capacity_u64 = capacity as u64;
    // ... rest of function unchanged
}

// Similar changes in try_read, available_read, available_write
```

**Testing Requirements:**
- Unit test: verify capacity is cached correctly
- Integration test: malicious domain modifies capacity, verify kernel rejects
- Gate test: `foundry-s9-ring-buffer-capacity-cache`

---

## Phase 2: Capability Validation Fixes

### Issue 2.1: SHMEM Capability Bypass - Missing Kind Check (HIGH)

**Location:** `kernel/src/shmem.rs:243-246`
**Finding:** `map_region` validates `(index, generation)` but doesn't call `validate_cap`, allowing capability type confusion.

**Evidence:**
```rust
// Line 243-246: ad-hoc validation without kind check
if shm_cap.index != index as u32 || shm_cap.generation != generation {
    return Err(STATUS_INVALID_CAPABILITY);
}

// Line 389-400: proper validator exists but is unused!
pub fn validate_cap(&self, shm_cap: Handle) -> bool {
    if shm_cap.kind != kernel_api::cap::HandleKind::Shmem {
        return false;
    }
    // ...
}
```

**Exploit Scenario:**
1. Attacker has IPC capability (HandleKind::Ipc) with index=1, generation=1
2. Attacker crafts SHMEM request with `shm_cap.index=1, shm_cap.generation=1`
3. `map_region` validation passes (index and generation match)
4. Attacker gains access to shared memory region they were never granted
5. Cross-domain memory access/corruption

**Fix Required:**

Replace ad-hoc validation with call to `validate_cap` and enforce `owner_domain_id` binding.

**Implementation:**

```rust
// In map_region function (line ~222):
pub fn map_region(
    &mut self,
    region_id: u64,
    shm_cap: Handle,
    target_domain_id: u64,
    rights: u32,
    cache_mode: u32,
) -> Result<u64, u32> {
    // ... existing parameter validation ...

    // CHANGED: Use proper validator
    if !self.validate_cap(shm_cap) {
        return Err(STATUS_INVALID_CAPABILITY);
    }

    // NEW: Enforce caller authority
    let slot = &mut self.regions[index - 1];
    if slot.owner_domain_id != current_caller_domain_id() {
        return Err(STATUS_INVALID_CAPABILITY);
    }

    // Check caller is authorized for target_domain
    if !authorized_for_domain(target_domain_id) {
        return Err(STATUS_INVALID_RIGHTS);
    }

    // ... rest of function
}

// Need to add caller identity tracking to IPC envelope:
// In kernel/src/ipc_v0.rs or envelope handling:
pub fn current_caller_domain_id() -> u64 {
    // Extract from IPC envelope or task context
    // For now: return domain ID from task structure
    // TODO: S8.5 - implement proper IPC envelope with caller_id
    unimplemented!()
}
```

**Testing Requirements:**
- Unit test: verify IPC caps are rejected by `validate_cap`
- Integration test: verify cross-domain mapping is rejected
- Gate test: `foundry-s8-s9-shmem-capability-kind-check`

---

### Issue 2.2: Virtual Address Collision (HIGH)

**Location:** `kernel/src/shmem.rs:267`
**Finding:** Virtual addresses derived from `region_id & 0xFFFF` cause collisions; no per-domain VA allocator.

**Evidence:**
```rust
// Line 267: VA derived from low 16 bits of region_id
let vaddr = unsafe {
    crate::arch::VirtAddr::new(0x8000_0000 + (region_id & 0xFFFF) * 0x1000)
};
```

**Exploit Scenario:**
1. Domain creates region with ID where `(region_id & 0xFFFF) == 0
2. Domain maps at 0x8000_0000
3. Domain creates another region with same low 16 bits
4. Second mapping overwrites first mapping's page table entries
5. Unmapping first region destroys second region's mappings
6. Use-after-free / corruption

**Fix Required:**

Implement per-domain virtual address allocator with explicit mapping objects.

**Implementation:**

```rust
// New module: kernel/src/va_allocator.rs
//! Per-domain virtual address allocator for shared memory mappings.
//!
//! Design:
//! - Fixed range per domain: 0x8000_0000 - 0x8FFF_F000
//! - Free list allocation
//! - Collision detection
//! - Alignment to page size (4KiB)

use kernel_api::cap::DomainId;

/// Virtual address space per domain
const SHMEM_VA_START: u64 = 0x8000_0000;
const SHMEM_VA_END: u64 = 0x8FFF_F000;
const SHMEM_VA_RANGE_PAGES: u64 = (SHMEM_VA_END - SHMEM_VA_START) / 4096;

/// Mapping object - tracks individual mappings
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Clone, Copy)]
pub struct ShmemMapping {
    /// Mapping ID (unique per domain)
    mapping_id: u64,
    /// Region this mapping references
    region_index: u32,
    /// Virtual address base
    vaddr: crate::arch::VirtAddr,
    /// Size in pages
    num_pages: u32,
    /// Domain that owns this mapping
    domain_id: DomainId,
    /// Next mapping ID in free list
    next_free: Option<u64>,
}

/// Per-domain VA allocator
pub struct VaAllocator {
    domain_id: DomainId,
    /// Free list of mapping IDs
    free_list_head: Option<u64>,
    /// Bitmap of allocated VA pages
    allocated: [bool; SHMEM_VA_RANGE_PAGES as usize],
}

impl VaAllocator {
    pub const fn new(domain_id: DomainId) -> Self {
        Self {
            domain_id,
            free_list_head: None,
            allocated: [false; SHMEM_VA_RANGE_PAGES as usize],
        }
    }

    /// Allocate virtual address range for shared memory mapping
    pub fn allocate(&mut self, num_pages: u32, region_index: u32)
        -> Result<crate::arch::VirtAddr, u32>
    {
        // Check if space available
        let free_count = self.allocated.iter().filter(|&&!*x).count();
        if free_count < num_pages as usize {
            return Err(STATUS_NO_MEMORY);
        }

        // Find contiguous range
        let mut start_page = None;
        let mut consecutive = 0;
        for (i, allocated) in self.allocated.iter().enumerate() {
            if !*allocated {
                consecutive += 1;
                if consecutive >= num_pages {
                    start_page = Some(i as u32 - consecutive + 1);
                    break;
                }
            } else {
                consecutive = 0;
            }
        }

        let start_page = match start_page {
            Some(page) => page,
            None => return Err(STATUS_NO_MEMORY),
        };

        // Mark as allocated
        for i in start_page..(start_page + num_pages as u32) {
            self.allocated[i as usize] = true;
        }

        let vaddr = crate::arch::VirtAddr::new(
            SHMEM_VA_START + (start_page as u64) * 4096
        );

        Ok((vaddr, start_page))
    }

    /// Free virtual address range
    pub fn deallocate(&mut self, start_page: u32, num_pages: u32) {
        for i in start_page..(start_page + num_pages) {
            self.allocated[i as usize] = false;
        }
    }
}
```

**Modify `map_region`:**
```rust
// Replace line 267 with:
let (vaddr, va_page) = va_allocator.allocate(slot.num_frames as u32, index as u32)?;

// Store mapping for later unmap
// Need per-domain mapping table (see Issue 2.3 below)
```

**Testing Requirements:**
- Unit test: verify VA collision detection
- Integration test: map multiple regions, verify distinct VAs
- Gate test: `foundry-s8-s9-shmem-va-allocator`

---

### Issue 2.3: Wire Format UB from Untrusted Bytes (HIGH)

**Location:** `kernel_api/src/wire.rs:19, 44`
**Finding:** Generic `T: Copy` bound allows constructing arbitrary types from untrusted IPC payload, violating validity invariants (padding, enum discriminants, bool).

**Evidence:**
```rust
// Line 19-20: Transmute any T to bytes without validation
let src = unsafe {
    core::slice::from_raw_parts((value as *const T).cast::<u8>(), len)
};

// Line 44: Assume any T is valid from bytes
unsafe {
    core::ptr::copy_nonoverlapping(src.as_ptr(), out.as_mut_ptr().cast::<u8>(), len);
    Ok(out.assume_init())  // UB if T has invalid bit patterns!
}
```

**Exploit Scenario:**
1. Attacker sends IPC payload with invalid enum tag or `bool` value
2. `read_payload<T>` constructs invalid Rust enum/bool
3. Undefined behavior in kernel when pattern-matching invalid value
4. Potential memory corruption, control flow diversion

**Fix Required:**

Replace `T: Copy` bound with `WirePod` trait, restrict to integer types only, generate validators.

**Implementation:**

```rust
// New trait: kernel_api/src/wire_pod.rs
//! Plain Old Data types for wire format serialization.
//!
//! # Safety
//!
//! Only types satisfying WirePod can be serialized from untrusted bytes.
//! This prevents constructing invalid enums, bools, or structs with padding.

/// Marker trait for wire-safe types.
///
/// # Safety
///
/// Only implement on types where:
/// - All bit patterns are valid (no enums, bools, references)
/// - No padding bytes (or padding is explicitly zeroed)
/// - Representation is C-compatible
pub unsafe trait WirePod: Copy {
    /// Validate bytes represent a valid instance of this type.
    ///
    /// Called after deserializing from untrusted IPC payload.
    /// Returns true if bytes are valid for this type.
    fn validate(bytes: &[u8]) -> bool;

    /// Get required alignment for this type.
    fn alignment() -> usize {
        core::mem::align_of::<Self>()
    }
}

// Implement WirePod for primitive integers only
macro_rules! impl_wire_pod_int {
    ($ty:ty) => {
        unsafe impl WirePod for $ty {
            fn validate(_bytes: &[u8]) -> bool {
                true  // All bit patterns valid for integers
            }
        }
    }
    }
}

impl_wire_pod_int!(u8);
impl_wire_pod_int!(u16);
impl_wire_pod_int!(u32);
impl_wire_pod_int!(u64);
impl_wire_pod_int!(i8);
impl_wire_pod_int!(i16);
impl_wire_pod_int!(i32);
impl_wire_pod_int!(i64);

// Deliberately NO impl for bool, enums, references!
```

**Modify wire functions:**
```rust
// In kernel_api/src/wire.rs:

pub fn write_payload<T: WirePod>(
    env: &mut Envelope,
    value: &T
) -> Result<(), WireError> {
    let len = core::mem::size_of::<T>();
    if len > env.payload.len() {
        return Err(WireError::PayloadTooLarge);
    }

    // NEW: Validate value can be safely transmuted
    let value_bytes = unsafe {
        core::slice::from_raw_parts((value as *const T).cast::<u8>(), len)
    };

    if !T::validate(value_bytes) {
        return Err(WireError::InvalidPayload);  // NEW error variant
    }

    env.payload[..len].copy_from_slice(value_bytes);
    // ... zero padding...

    Ok(())
}

pub fn read_payload<T: WirePod>(env: &Envelope) -> Result<T, WireError> {
    let len = core::mem::size_of::<T>();
    let payload_len = env.payload_len as usize;

    if payload_len > env.payload.len() {
        return Err(WireError::PayloadLenInvalid);
    }
    if payload_len < len {
        return Err(WireError::PayloadTooSmall);
    }

    let bytes = &env.payload[..len];

    // NEW: Validate before constructing T
    if !T::validate(bytes) {
        return Err(WireError::InvalidPayload);  // NEW error variant
    }

    let mut out = core::mem::MaybeUninit::<T>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), out.as_mut_ptr().cast::<u8>(), len);
        Ok(out.assume_init())
    }
}
```

**Modify IDL codegen:**
```rust
// In idl_codegen/src/main.rs, restrict generated types:
// Only allow integer types in kernel-facing messages.
// Reject enums, bools, structs with padding.
```

**Testing Requirements:**
- Unit test: verify invalid payload rejected
- Fuzz test: malformed IPC payloads to kernel handlers
- Gate test: `foundry-s9-wire-pod-validation`

---

## Phase 3: Store Service Hardening

### Issue 3.1: Default Test Key in Production (HIGH)

**Location:** `services/store_service/src/capability.rs:93-106`
**Finding:** Missing `RAMEN_STORE_TRUSTED_KEYS` falls back to RFC 8032 test vector, allowing attackers with knowledge of corresponding private key.

**Evidence:**
```rust
// Line 93-106: DANGEROUS default key
Err(_) => {
    // Development mode: use a default test key
    let default_key_bytes = [
        0xd7, 0x5a, 0x98, ...  // RFC 8032 test vector!
    ];
    // ...
}
```

**Fix Required:**

Remove default key fallback; require explicit configuration or fail-closed in production.

**Implementation:**

```rust
// In load_trusted_public_keys function:

pub fn load_trusted_public_keys() -> Vec<EdVerifyingKey> {
    let env_var = env::var("RAMEN_STORE_TRUSTED_KEYS");

    match env_var {
        Ok(keys_str) => {
            // Parse keys as before...
            keys
        }
        Err(_) => {
            // CHANGED: No default key fallback!

            // Option 1: Fail-closed (production)
            eprintln!("ERROR: RAMEN_STORE_TRUSTED_KEYS not set");
            eprintln!("Capability verification will FAIL-CLOSED");
            eprintln!("Set environment variable with comma-separated base64 Ed25519 public keys");
            return Vec::new();  // Empty = no trusted keys = all verifications fail

            // Option 2: Generate ephemeral per-boot key (dev-only)
            // #[cfg(debug_assertions)]
            // {
            //     let (signing_key, verifying_key) = generate_keypair();
            //     eprintln!("WARNING: Generated ephemeral dev key:");
            //     eprintln!("{}", hex::encode(verifying_key.as_bytes()));
            //     vec![verifying_key]
            // }
        }
    }
}

// Add build-time feature gate:
#[cfg(feature = "store_service_prod")]
const REQUIRE_TRUSTED_KEYS: bool = true;

#[cfg(not(feature = "store_service_prod"))]
const REQUIRE_TRUSTED_KEYS: bool = false;

// Add startup check:
pub fn verify_trusted_keys_configured() -> Result<(), String> {
    if REQUIRE_TRUSTED_KEYS && load_trusted_public_keys().is_empty() {
        return Err("RAMEN_STORE_TRUSTED_KEYS must be set in production mode".to_string());
    }
    Ok(())
}
```

**Service startup:**
```rust
// In services/store_service/src/main.rs, add at startup:
if let Err(e) = capability::verify_trusted_keys_configured() {
    eprintln!("FATAL: {}", e);
    std::process::exit(1);
}
```

**Testing Requirements:**
- Unit test: verify missing env var causes empty key list
- Integration test: verify signature verification fails with no keys
- Production build flag: `--features store_service_prod`

---

### Issue 3.2: Path Traversal in Artifact Store (MEDIUM)

**Location:** `artifact_store_core/src/lib.rs:127-138`
**Finding:** `manifest_path` and `blob_path` accept `&str` without validation, allowing `../` escape if `content_id` is attacker-controlled.

**Evidence:**
```rust
// Line 127-131: No validation of content_id format
pub fn manifest_path(root: &Path, content_id: &str) -> PathBuf {
    let name = content_id
        .strip_prefix(CONTENT_ID_PREFIX)  // unwrap_or returns input!
        .unwrap_or(content_id);
    root.join(format!("{}.manifest.json", name))
}
```

**Fix Required:**

Remove `&str` variants; require `ContentId` type with embedded validation.

**Implementation:**

```rust
// In artifact_store_core/src/lib.rs:

// DEPRECATED: Use manifest_path_for() with ContentId instead
#[deprecated(note = "Use manifest_path_for() with ContentId type for validation")]
pub fn manifest_path(root: &Path, content_id: &str) -> PathBuf {
    // Existing code - mark deprecated
    let name = content_id
        .strip_prefix(CONTENT_ID_PREFIX)
        .unwrap_or(content_id);
    root.join(format!("{}.manifest.json", name))
}

// DEPRECATED: Use blob_path_for() with ContentId instead
#[deprecated(note = "Use blob_path_for() with ContentId type for validation")]
pub fn blob_path(root: &Path, content_id: &str) -> PathBuf {
    let name = content_id
        .strip_prefix(CONTENT_ID_PREFIX)
        .unwrap_or(content_id);
    root.join(format!("{}.blob", name))
}

// NEW functions using ContentId with embedded validation:
pub fn manifest_path_for(root: &Path, content_id: &ContentId) -> PathBuf {
    // ContentId already validated during construction
    root.join(format!("{}.manifest.json", content_id.hash_hex()))
}

pub fn blob_path_for(root: &Path, content_id: &ContentId) -> PathBuf {
    root.join(format!("{}.blob", content_id.hash_hex()))
}

// Add validation to ContentId in artifact_store_schema:
```

**ContentId validation:**
```rust
// In artifact_store_schema/src/lib.rs:

impl ContentId {
    pub fn new(hash_hex: &str) -> Result<Self, ContentIdError> {
        // Validate format: sha256: followed by 64 hex chars
        if !hash_hex.starts_with(CONTENT_ID_PREFIX) {
            return Err(ContentIdError::InvalidFormat);
        }

        let hash_part = &hash_hex[CONTENT_ID_PREFIX.len()..];

        // Validate hex encoding and length
        if hash_part.len() != 64 {
            return Err(ContentIdError::InvalidLength);
        }

        if !hash_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ContentIdError::InvalidHex);
        }

        // Additional security: validate no path traversal chars
        if hash_part.contains("..") || hash_part.contains("/") || hash_part.contains("\\") {
            return Err(ContentIdError::InvalidCharacters);  // NEW error variant
        }

        Ok(Self { hash: HashArray::from_hex(hash_part)? })
    }

    pub fn hash_hex(&self) -> &str {
        // Returns validated hash without prefix
        // Safe for path joining
        &self.hash.to_hex_string()[..64]  // Truncate to validated length
    }
}
```

**Update all call sites:**
```rust
// In services/store_service/src/main.rs and elsewhere:
// Replace:
let path = manifest_path(root, &content_id);
// With:
let path = manifest_path_for(root, &content_id);
```

**Testing Requirements:**
- Unit test: verify path traversal rejected
- Integration test: verify valid ContentId works
- Security test: fuzzer for content_id validation

---

## Phase 4: Sandbox Hardening

### Issue 4.1: POSIX Runner Sandbox Escape (HIGH)

**Location:** `runtime_supervisor/src/sandbox.rs:277-287`
**Finding:** Sandbox relies on external `seccomp` binary; silently proceeds without filtering if binary missing.

**Evidence:**
```rust
// Line 277-287: Ineffective seccomp detection
if std::path::Path::new("/usr/sbin/seccomp").exists()
    || std::path::Path::new("/usr/bin/seccomp").exists()
{
    // Use seccomp tool to apply filter
    // TODO: Integrate libseccomp-sys for proper seccomp filtering
}
```

**Exploit Scenario:**
1. Attacker compromises host system, removes `/usr/bin/seccomp`
2. POSIX runner executed with no seccomp filtering
3. Full syscall access → escape chroot namespace
4. Execute arbitrary commands, read/write host filesystem

**Fix Required:**

Use `libseccomp` crate for native BPF filtering; fail-closed if unavailable.

**Implementation:**

```rust
// In runtime_supervisor/Cargo.toml, add dependency:
[dependencies]
libseccomp = "0.3"  # Or similar crate

// In runtime_supervisor/src/sandbox.rs:

#[cfg(target_os = "linux")]
use libseccomp::{ScmpFilterContext, ScmpAction, ScmpCompare, ScmpSyscall};

/// Apply seccomp filter - native implementation
#[cfg(target_os = "linux")]
fn apply_seccomp_filter(cmd: &mut Command) -> io::Result<()> {
    // CHANGED: Build filter programmatically, never rely on external binary

    let mut ctx = ScmpFilterContext::new()?;

    // Allow whitelist only
    ctx.add_rule(
        ScmpSyscall::new(libc::SYS_read)?,
        ScmpCompare::NotEqual(0),  // Disallow read(fd=0) for stderr attack
        ScmpAction::Allow
    )?;

    // ... Add all whitelisted syscalls from SECCOMP_WHITELIST ...

    // Block dangerous syscalls explicitly
    ctx.add_rule(libc::SYS_execve?, ScmpAction::Errno(libc::EPERM))?;
    ctx.add_rule(libc::SYS_fork?, ScmpAction::Errno(libc::EPERM))?;
    ctx.add_rule(libc::SYS_clone?, ScmpAction::Errno(libc::EPERM))?;
    ctx.add_rule(libc::SYS_socket?, ScmpAction::Errno(libc::EPERM))?;

    // Load filter into current process
    cmd.arg0("--seccomp-filter");
    ctx.apply(cmd)?;

    Ok(())
}

// Remove the ineffective external binary check
// Delete lines 277-287 entirely
```

**Fail-safe behavior:**
```rust
// If libseccomp not available, fail:
#[cfg(not(feature = "dev_allow_weak_sandbox"))]
{
    compile_error!("seccomp-sys crate is required for sandboxing in production builds");
}
```

**Testing Requirements:**
- Unit test: verify filter is loaded
- Integration test: attempt blocked syscalls, verify SIGSYS
- Security test: run exploit payload without seccomp binary

---

## Phase 5: Structural Architectural Fixes

### Issue 5.1: Per-Domain Capability Tables

**Current Problem:** No kernel-side caller identity tracking; SHMEM caps not bound to specific domains.

**Implementation Required:**

1. Add `caller_domain_id` to IPC envelope
2. Add per-domain capability tables
3. Validate authorization in `map_region`

**Design:**

```rust
// In kernel_api/src/ipc.rs or envelope:

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Envelope {
    pub msg_type: u32,
    pub payload_len: u32,
    pub handle: Handle,
    pub caller_domain_id: u64,  // NEW: caller identity
    pub payload: [u8; PAYLOAD_MAX],
}

// In kernel/src/shmem.rs:

pub struct PerDomainShmemState {
    /// Per-domain capability table for SHMEM regions
    shm_caps: [Option<Handle>; MAX_SHMEM_CAPS_PER_DOMAIN],

    /// Per-domain VA allocator
    va_allocator: VaAllocator,

    /// Domain ID
    domain_id: DomainId,
}

impl PerDomainShmemState {
    pub const fn new(domain_id: DomainId) -> Self {
        Self {
            shm_caps: [None; MAX_SHMEM_CAPS_PER_DOMAIN],
            va_allocator: VaAllocator::new(domain_id),
            domain_id,
        }
    }

    /// Grant SHMEM capability to domain
    pub fn grant_shmem_cap(&mut self, cap: Handle) -> Result<(), u32> {
        // Find free slot
        let slot = self.shm_caps.iter_mut().find(|s| s.is_none());

        match slot {
            Some(s) => {
                *s = Some(cap);
                Ok(())
            }
            None => Err(STATUS_NO_SLOTS),
        }
    }

    /// Validate capability is granted to specific domain
    pub fn validate_has_shmem_cap(&self, cap: Handle) -> bool {
        self.shm_caps.iter().any(|c| c == Some(cap))
    }
}
```

---

### Issue 5.2: Locking for Multi-Core

**Current Problem:** `static mut FRAME_ALLOCATOR` documented as single-threaded only but no enforcement mechanism prevents concurrent access when SMP enabled.

**Implementation Required:**

Add `Spinlock<T>` wrapper; protect all global state.

**Implementation:**

```rust
// New module: kernel/src/sync.rs

//! Kernel synchronization primitives.
//!
//! # V-011: Spinlocks for SMP
//!
//! Simple spinlock implementation using atomic operations.
//! Used to protect global state before full scheduler is available.

use core::sync::atomic::{AtomicBool, Ordering};
use core::ops::Deref;

pub struct Spinlock<T> {
    locked: AtomicBool,
    data: T,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data,
        }
    }

    pub fn lock(&self) -> LockGuard<T> {
        // Spin until we acquire lock
        while self.locked.compare_exchange(false, true, Ordering::Acquire) {
            // Spin - hint to CPU to yield
            core::hint::spin_loop();
        }

        LockGuard { lock: &self.locked, data: &self.data }
    }
}

// Safety: never allow direct access to inner data
impl<T> Deref for Spinlock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        panic!("Attempted to access Spinlock<T> content without acquiring lock");
    }
}

pub struct LockGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a T,
}

impl<T> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl<T> Deref for LockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
```

**Protect global allocator:**
```rust
// In kernel/src/mm/mod.rs:

use crate::sync::Spinlock;

pub static FRAME_ALLOCATOR: Spinlock<Option<BitmapAllocator>> = Spinlock::new(None);
```

**Update smp_enabled check:**
```rust
// Instead of just panicking, actually prevent concurrent access:
pub fn smp_enabled() {
    if SMP_ENABLED.swap(true, Ordering::SeqCst) {
        // After this point, Spinlock protects globals
        // No panic needed
    }
}
```

**Testing Requirements:**
- Unit test: verify spinlock mutual exclusion
- Stress test: concurrent allocation/deallocation
- Integration test: run shmem operations with SMP enabled

---

## Phase 6: Service Boundary Cleanup

### Issue 6.1: CLI Depends on Service Internals (MEDIUM)

**Location:** `store_cli/Cargo.toml:1-13`
**Finding:** CLI depends directly on `store_service` crate, exposing service internals to untrusted CLI.

**Evidence:**
```toml
[dependencies]
store_service = { path = "../services/store_service" }
```

**Fix Required:**

Split `store_service` into `service` + `client` crates.

**Implementation:**

```
New crate structure:
- store_service_core/: Protocol types + serialization only (no IO)
- store_service/: Server implementation with IO
- store_service_client/: Client using only store_service_core
- store_cli/: Uses store_service_client (not store_service)
```

**Protocol types only:**
```rust
// In store_service_core/src/lib.rs:
pub mod protocol;
pub mod capability;

// NO IO functions exported
// Only:
// - Request/Response structs
// - Encoding/decoding helpers
// - Error types
```

**Testing Requirements:**
- Verify store_cli builds without store_service dependency
- Integration test: CLI commands work via client protocol

---

## Execution Order

**Priority Matrix (Critical → Medium):**

1. **[P0-CRITICAL]** Ring buffer capacity caching (1.2) - 2 days
2. **[P0-CRITICAL]** SHMEM refcount overflow (1.1) - 2 days
3. **[P1-HIGH]** SHMEM capability kind check (2.1) - 3 days
4. **[P1-HIGH]** VA allocator implementation (2.2) - 5 days
5. **[P1-HIGH]** WirePod trait for wire format (2.3) - 5 days
6. **[P2-HIGH]** Default test key removal (3.1) - 2 days
7. **[P2-HIGH]** Per-domain capability tables (5.1) - 5 days
8. **[P2-HIGH]** Native seccomp filtering (4.1) - 3 days
9. **[P3-MEDIUM]** Spinlock for SMP (5.2) - 3 days
10. **[P3-MEDIUM]** Path traversal fix (3.2) - 1 day
11. **[P4-MEDIUM]** Service/client split (6.1) - 3 days

**Total Effort: ~31 days**

**Dependencies:**
- Issues 1.2 and 5.2 can proceed independently
- Issues 1.1, 2.1, 2.2, 2.3, 3.1 require foundational changes
- Issues 3.1, 4.1, 5.1, 6.1 are localized fixes

---

## Gates Required

**S9.0 Hardening Gates:**
- `foundry-s9-ring-buffer-capacity-cache` - verify 1.2
- `foundry-s9-shmem-refcount-overflow` - verify 1.1
- `foundry-s9-shmem-capability-kind-check` - verify 2.1
- `foundry-s9-shmem-va-allocator` - verify 2.2
- `foundry-s9-wire-pod-validation` - verify 2.3
- `foundry-s9-store-trusted-keys-required` - verify 3.1
- `foundry-s9-native-seccomp` - verify 4.1
- `foundry-s9-spinlock-smp` - verify 5.2
- `foundry-s9-path-traversal-validation` - verify 3.2

**S9.1 Boundary Gates:**
- `foundry-s9-per-domain-cap-tables` - verify 5.1
- `foundry-s9-service-client-split` - verify 6.1

**S9.2 Integration Gates:**
- `foundry-s9-shmem-full-stack` - all SHMEM fixes integrated
- `foundry-s9-store-hardening` - all store fixes integrated

---

## Decision Log Entries

All changes documented in this plan MUST be recorded in `DECISIONS.md` with:

- Rationale for each change
- Alternatives considered
- Impact on invariants

**Example:**
```markdown
## 2025-01-15: Fix SHMEM Refcount Overflow (VULN-1.1)

**Issue:** Unchecked u32 refcount increment allows wraparound → UAF
**Change:** Use u64 refcount with checked_add()
**Files:** kernel/src/shmem.rs
**Invariant:** Preserves "kernel-side capability validation" - no change
**Rationale:** u32 overflow at 4B mappings is realistic; use-after-free is catastrophic
**Alternatives Considered:**
- Per-mapping refcount table (rejected: too complex for S8)
- Explicit mapping limit instead (chosen: both)
**Impact:** Fixes critical memory corruption; enables safe multi-domain SHMEM
```

---

## Success Criteria

Each fix is complete when:

1. **Code Change:** Fix implemented and committed
2. **Unit Tests:** New tests cover the vulnerability
3. **Integration Tests:** Gate test demonstrates fix prevents exploit
4. **Documentation:** Code comments explain security invariant
5. **Decision Log:** Entry in DECISIONS.md
6. **Review:** Security review of fix by second auditor

**Definition of Done:**
- All P0 (Critical) issues resolved
- All P1 (High) issues resolved
- Gates passing for new hardened code
- No regressions in existing functionality
- Documentation updated

---

## Appendix: Threat Model Update

After remediation, system enforces:

**Kernel TCB Protections:**
- Typed messages with `WirePod` validation
- Capability kind and provenance checks
- Per-domain authorization enforcement
- Kernel-private cached data (ring buffer capacity)

**Service TCB Protections:**
- Fail-closed key loading
- Native seccomp BPF filters
- Path traversal validation
- Service/client boundary separation

**Attack Surface Reduction:**
- Removed external binary dependencies
- No default test keys in production
- All untrusted inputs validated
- Kernel globals protected by spinlocks

**Remaining Trust Boundaries:**
- Kernel assumes services are correctly implemented (defense-in-depth)
- Services assume kernel IPC is correct (not bypassed)
- Store assumes capabilities are valid (signed by trusted key)

---

**Next Review Required:**
After all P0-P4 fixes complete, schedule second security audit focusing on:
- S10: Device driver isolation
- S11: Scheduler and timer security
- S12: Network stack hardening
