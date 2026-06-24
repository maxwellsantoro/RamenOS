#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use core::arch::global_asm;
#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use kernel::boot::{BootMemoryMap, RegionKind};

#[cfg(not(test))]
extern "C" {
    static __stack_end: u8;
}

#[cfg(not(test))]
const INIT_IMAGE_ADDR: usize = 0x4400_0000;
#[cfg(not(test))]
const INIT_IMAGE_MAX_LEN: usize = 4096;

#[cfg(not(test))]
global_asm!(
    r#"
.section .text._start
.global _start
_start:
    ldr x0, =__stack_end
    mov sp, x0
    bl rust_start
1:
    wfe
    b 1b
"#
);

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn rust_start() -> ! {
    enable_fp();

    // Set init image
    kernel::boot::set_init_image(kernel::boot::InitImage {
        ptr: INIT_IMAGE_ADDR as *const u8,
        len: INIT_IMAGE_MAX_LEN,
    });

    // S8 Phase 3: Hardcode memory map for QEMU virt
    // RAM starts at 0x4000_0000.
    // Kernel is at 0x4008_0000.
    // Init image is at 0x4400_0000.
    // We declare a safe usable region starting at 0x5000_0000 (256MB mark) for 256MB.
    // This avoids overlapping with kernel code, stack, or init image.
    let mut map = BootMemoryMap::new();
    map.add(0x5000_0000, 256 * 1024 * 1024, RegionKind::Usable);
    kernel::boot::set_memory_map(map);

    kernel::boot::boot_main();
}

#[cfg(not(test))]
fn enable_fp() {
    unsafe {
        let mut el: u64;
        core::arch::asm!("mrs {0}, CurrentEL", out(reg) el);
        let el = (el >> 2) & 0x3;

        let mut cpacr: u64;
        core::arch::asm!("mrs {0}, cpacr_el1", out(reg) cpacr);
        cpacr |= 0b11 << 20;
        core::arch::asm!("msr cpacr_el1, {0}", in(reg) cpacr);

        if el >= 2 {
            let mut cptr: u64;
            core::arch::asm!("mrs {0}, cptr_el2", out(reg) cptr);
            cptr &= !(0b11 << 20);
            core::arch::asm!("msr cptr_el2, {0}", in(reg) cptr);
        }

        core::arch::asm!("isb");
    }
}

#[panic_handler]
#[cfg(not(test))]
fn panic(info: &PanicInfo) -> ! {
    kernel::serial::init();
    kernel::kprintln!("PANIC: {}", info);
    loop {
        kernel::arch::halt();
    }
}

#[cfg(test)]
fn main() {}
