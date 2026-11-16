//! x86 I/O port access.
//!
//! Devices like the keyboard controller are reached through numbered "I/O ports".
//! The CPU uses special instructions for this: `in` (read) and `out` (write).
//! These functions are unsafe because using the wrong port or value can confuse
//! or reset hardware - the caller must know which device the port belongs to.

use core::arch::asm;

/// Read one byte from an I/O port.
pub unsafe fn read_port(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

/// Write one byte to an I/O port.
pub unsafe fn write_port(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

/// Write two bytes (a 16-bit value) to an I/O port.
pub unsafe fn write_port_u16(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
}
