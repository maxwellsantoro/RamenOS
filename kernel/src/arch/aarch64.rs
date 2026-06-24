use core::arch::asm;

use crate::mm::address::PhysAddr;

/// Kernel virtual address range for aarch64.
/// The kernel is mapped in the higher half of the address space.
pub const KERNEL_START: usize = 0xFFFF000000000000;
pub const KERNEL_END: usize = 0xFFFFFFFFFFFFFFFF;

/// Physical address range for aarch64.
/// UEFI loads init images at physical addresses (e.g., 0x10000000).
pub const PHYS_MEMORY_START: usize = 0;
pub const PHYS_MEMORY_END: usize = 4 * 1024 * 1024 * 1024; // 4 GiB

/// MMIO region range (3-4 GiB) for device memory.
/// This region is excluded from valid physical memory for init images.
pub const PHYS_MMIO_REGION_START: usize = 3 * 1024 * 1024 * 1024; // 3 GiB

pub mod mmu;

/// Get the current page table root from TTBR0_EL1 register.
///
/// Reads the TTBR0_EL1 register which contains the physical address of the
/// top-level page table for the lower address space. The lower 12 bits are
/// flags and are masked out.
///
/// # Returns
///
/// The physical address of the current page table root.
pub fn get_current_page_table_root() -> PhysAddr {
    let ttbr0: u64;
    // SAFETY: Reading TTBR0_EL1 is safe because it only returns the current
    // page table base address. It does not modify any state. The MRS instruction
    // is a read-only system register access.
    unsafe {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) ttbr0, options(nomem, nostack));
    }
    // SAFETY: TTBR0 contains a valid page table root address that was set
    // during early boot. The mask removes flags from the lower 12 bits to
    // get the physical address, which is guaranteed to be page-aligned.
    unsafe { PhysAddr::new(ttbr0 & !0xFFF) } // Clear lower 12 bits (flags)
}

pub mod serial {
    const UART_BASE: usize = 0x0900_0000;
    const UART_DR: usize = UART_BASE;
    const UART_FR: usize = UART_BASE + 0x18;
    const UART_IBRD: usize = UART_BASE + 0x24;
    const UART_FBRD: usize = UART_BASE + 0x28;
    const UART_LCRH: usize = UART_BASE + 0x2C;
    const UART_CR: usize = UART_BASE + 0x30;

    #[inline]
    unsafe fn mmio_write(addr: usize, val: u32) {
        // SAFETY: MMIO write to UART registers is safe in bare-metal kernel context.
        // The UART_BASE (0x0900_0000) is a well-defined PL011 UART in QEMU's virt machine.
        // Register offsets (DR, FR, IBRD, FBRD, LCRH, CR) follow the PL011 specification.
        // No other kernel code writes to these addresses during initialization.
        core::arch::asm!(
            "str {val:w}, [{addr}]",
            addr = in(reg) addr,
            val = in(reg) val,
            options(nostack, preserves_flags)
        );
    }

    #[inline]
    unsafe fn mmio_read(addr: usize) -> u32 {
        // SAFETY: MMIO read from UART registers is safe in bare-metal kernel context.
        // The UART_BASE (0x0900_0000) is a well-defined PL011 UART in QEMU's virt machine.
        // Reading from UART flag register (FR) is safe and well-defined.
        let val: u32;
        core::arch::asm!(
            "ldr {val:w}, [{addr}]",
            addr = in(reg) addr,
            val = out(reg) val,
            options(nostack, preserves_flags)
        );
        val
    }

    pub fn init() {
        unsafe {
            // SAFETY: Serial port initialization writes to well-defined PL011 UART registers.
            // The initialization sequence follows the ARM PL011 UART datasheet:
            // - Disable UART (UART_CR = 0)
            // - Set integer baud rate divisor (UART_IBRD = 13) for 115200 baud from 24MHz clock
            // - Set fractional baud rate divisor (UART_FBRD = 2)
            // - Set 8-bit word length, enable FIFOs (UART_LCRH = 0x70)
            // - Enable UART, TX, and RX (UART_CR = 0x301)
            mmio_write(UART_CR, 0);
            mmio_write(UART_IBRD, 13);
            mmio_write(UART_FBRD, 2);
            mmio_write(UART_LCRH, (1 << 4) | (1 << 5) | (1 << 6));
            mmio_write(UART_CR, (1 << 0) | (1 << 8) | (1 << 9));
        }
    }

    pub fn write_byte(byte: u8) {
        unsafe {
            // SAFETY: Wait for transmit FIFO full flag (bit 5) in FR register.
            // Polling the UART flag register is safe; the bit is cleared by hardware when space available.
            // Writing to DR when TX FIFO not full is well-defined and safe.
            while (mmio_read(UART_FR) & (1 << 5)) != 0 {}
            mmio_write(UART_DR, byte as u32);
        }
    }
}

pub fn halt() {
    unsafe {
        // SAFETY: WFE (Wait For Event) instruction puts the CPU in low-power state.
        // In bare-metal kernel context, this is a valid idle state.
        // The CPU will resume execution on the next event (e.g., interrupt, SEV).
        asm!("wfe", options(nomem, nostack));
    }
}
