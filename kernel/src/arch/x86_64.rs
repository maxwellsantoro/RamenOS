use core::arch::asm;

use crate::mm::address::PhysAddr;

/// Kernel virtual address range for x86_64.
/// The kernel is mapped in the higher half of the address space.
pub const KERNEL_START: usize = 0xFFFF800000000000;
pub const KERNEL_END: usize = 0xFFFFFFFFFFFFFFFF;

/// Physical address range for x86_64.
/// UEFI loads init images at physical addresses (e.g., 0x10000000).
pub const PHYS_MEMORY_START: usize = 0;
pub const PHYS_MEMORY_END: usize = 4 * 1024 * 1024 * 1024; // 4 GiB

/// MMIO region range (3-4 GiB) for device memory.
/// This region is excluded from valid physical memory for init images.
pub const PHYS_MMIO_REGION_START: usize = 3 * 1024 * 1024 * 1024; // 3 GiB

pub mod mmu;

// Re-export MMU types
pub use mmu::X86_64Mmu;

pub mod serial {
    use core::arch::asm;

    const COM1: u16 = 0x3F8;

    #[inline]
    unsafe fn outb(port: u16, val: u8) {
        // SAFETY: Port I/O to COM1 (0x3F8) is safe in bare-metal kernel context.
        // The COM1 base address is a well-defined UART port in QEMU's default serial setup.
        // No other kernel code writes to these ports during initialization.
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
    }

    #[inline]
    unsafe fn inb(port: u16) -> u8 {
        // SAFETY: Port I/O from COM1 (0x3F8) is safe in bare-metal kernel context.
        // The COM1 base address is a well-defined UART port in QEMU's default serial setup.
        // Reading from UART status registers (COM1 + 5) is safe and well-defined.
        let mut val: u8;
        asm!("in al, dx", in("dx") port, out("al") val, options(nomem, nostack, preserves_flags));
        val
    }

    pub fn init() {
        unsafe {
            // SAFETY: Serial port initialization writes to well-defined COM1 registers.
            // The initialization sequence follows the 16550 UART datasheet:
            // - Disable interrupts (COM1 + 1 = 0x00)
            // - Set DLAB to access divisor latch (COM1 + 3 = 0x80)
            // - Set divisor low byte (COM1 + 0 = 0x03) for 38400 baud
            // - Set divisor high byte (COM1 + 1 = 0x00)
            // - Clear DLAB and set 8N1 format (COM1 + 3 = 0x03)
            // - Enable FIFO with 14-byte threshold (COM1 + 2 = 0xC7)
            // - Enable DTR and RTS (COM1 + 4 = 0x0B)
            outb(COM1 + 1, 0x00);
            outb(COM1 + 3, 0x80);
            // 16550 THR is COM1+0; offset written explicitly for datasheet parity.
            #[allow(clippy::identity_op)]
            outb(COM1 + 0, 0x03);
            outb(COM1 + 1, 0x00);
            outb(COM1 + 3, 0x03);
            outb(COM1 + 2, 0xC7);
            outb(COM1 + 4, 0x0B);
        }
    }

    pub fn write_byte(byte: u8) {
        unsafe {
            // SAFETY: Wait for transmit hold register empty (THRE) bit (bit 5) in LSR (COM1 + 5).
            // Polling the UART status register is safe; the bit is set by hardware when ready.
            // Writing to THR (COM1) when THRE is set is well-defined and safe.
            while (inb(COM1 + 5) & 0x20) == 0 {}
            outb(COM1, byte);
        }
    }

    /// COM2 UART for host↔target IPC bridge (S10.5.2). QEMU maps this via `isa-serial` chardev.
    pub mod ipc {
        use super::{inb, outb};

        const COM2: u16 = 0x2F8;

        pub fn init() {
            unsafe {
                outb(COM2 + 1, 0x00);
                outb(COM2 + 3, 0x80);
                outb(COM2, 0x03);
                outb(COM2 + 1, 0x00);
                outb(COM2 + 3, 0x03);
                outb(COM2 + 2, 0xC7);
                outb(COM2 + 4, 0x0B);
            }
            flush_rx();
        }

        /// Discard any bytes already in the RX FIFO (firmware/QEMU noise before host frames).
        pub fn flush_rx() {
            unsafe {
                while (inb(COM2 + 5) & 0x01) != 0 {
                    let _ = inb(COM2);
                }
            }
        }

        pub fn write_byte(byte: u8) {
            unsafe {
                while (inb(COM2 + 5) & 0x20) == 0 {}
                outb(COM2, byte);
            }
        }

        pub fn read_byte_timeout(max_spins: u32) -> Option<u8> {
            for i in 0..max_spins {
                unsafe {
                    if (inb(COM2 + 5) & 0x01) != 0 {
                        return Some(inb(COM2));
                    }
                }
                if i % 4096 == 4095 {
                    crate::arch::halt();
                }
            }
            None
        }

        pub fn read_byte_blocking() -> u8 {
            loop {
                if let Some(byte) = read_byte_timeout(50_000_000) {
                    return byte;
                }
                crate::arch::halt();
            }
        }

        pub fn write_bytes(bytes: &[u8]) {
            for byte in bytes {
                write_byte(*byte);
            }
        }

        pub fn read_exact(buf: &mut [u8], max_spins_per_byte: u32) -> bool {
            for byte in buf.iter_mut() {
                *byte = match read_byte_timeout(max_spins_per_byte) {
                    Some(value) => value,
                    None => return false,
                };
            }
            true
        }
    }
}

pub fn halt() {
    unsafe {
        // SAFETY: HLT instruction halts the CPU until an interrupt occurs.
        // In bare-metal kernel context, this is a valid idle state.
        // The CPU will resume execution on the next interrupt (e.g., timer, I/O).
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Get the current page table root from CR3 register.
///
/// Reads the CR3 register which contains the physical address of the
/// top-level page table (PML4). The lower 12 bits are flags and are masked out.
///
/// # Returns
///
/// The physical address of the current page table root.
pub fn get_current_page_table_root() -> PhysAddr {
    let cr3: u64;
    // SAFETY: Reading CR3 returns the current page table base. It's a read-only
    // operation that doesn't affect memory safety. The MOV from CR3 instruction
    // simply reads the control register without any side effects.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
    }
    // SAFETY: CR3 contains a valid page table root address that was set during
    // early boot by UEFI or the bootloader. The mask removes flags from the
    // lower 12 bits to get the physical address, which is guaranteed to be
    // page-aligned by the hardware.
    unsafe { PhysAddr::new(cr3 & !0xFFF) } // Clear lower 12 bits (flags)
}
