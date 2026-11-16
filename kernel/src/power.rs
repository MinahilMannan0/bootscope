//! Reboot and shutdown.

use crate::port::{read_port, write_port, write_port_u16};

/// Restart the computer by asking the PS/2 keyboard controller to pulse the
/// CPU reset line. This is an old PC mechanism that QEMU and most PCs support.
pub fn reboot() -> ! {
    // SAFETY: 0x64 is the PS/2 controller's command port; 0xFE means "reset CPU".
    unsafe {
        // Wait until the controller is ready to accept a command (bit 1 = busy).
        while read_port(0x64) & 0x02 != 0 {}
        write_port(0x64, 0xFE);
    }
    // Only reached if the reset did not happen.
    crate::halt_forever()
}

/// Power off the QEMU virtual machine.
/// QEMU's default PC exposes an ACPI power-management port at 0x604; writing
/// 0x2000 there means "power off". This is QEMU-specific: real hardware would
/// need a full ACPI driver to find the correct port and value.
pub fn shutdown() -> ! {
    // SAFETY: on QEMU's default machine, port 0x604 is the ACPI power control port.
    unsafe { write_port_u16(0x604, 0x2000) };
    // If power-off did not work, at least stop the CPU.
    crate::halt_forever()
}
