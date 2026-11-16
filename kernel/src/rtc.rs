//! CMOS real-time clock (RTC).
//!
//! Every PC has a small battery-powered clock chip that keeps the date and time,
//! even when the computer is off. We talk to it through two I/O ports:
//!   0x70 = select which clock register we want
//!   0x71 = read that register's value
//! QEMU gives its virtual clock the time of your Windows PC.

use crate::port::{read_port, write_port};

const SELECT_PORT: u16 = 0x70;
const DATA_PORT: u16 = 0x71;

/// A date and time read from the clock chip.
#[derive(Clone, Copy)]
pub struct Time {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

fn read_register(register: u8) -> u8 {
    // SAFETY: 0x70/0x71 are the standard CMOS clock ports. Selecting a register
    // below 0x80 and reading it does not change any memory.
    unsafe {
        write_port(SELECT_PORT, register);
        read_port(DATA_PORT)
    }
}

/// Register 0x0A, bit 7: the chip is in the middle of updating its time.
fn update_in_progress() -> bool {
    read_register(0x0A) & 0x80 != 0
}

/// Read seconds, minutes, hours, day, month, year (still in the chip's format).
fn read_raw() -> [u8; 6] {
    while update_in_progress() {}
    [
        read_register(0x00),
        read_register(0x02),
        read_register(0x04),
        read_register(0x07),
        read_register(0x08),
        read_register(0x09),
    ]
}

/// Read the current date and time.
pub fn now() -> Time {
    // Read twice until both reads agree, so we never get a half-updated time.
    let mut raw = read_raw();
    loop {
        let again = read_raw();
        if again == raw {
            break;
        }
        raw = again;
    }

    // Register 0x0B tells us the chip's format.
    let status_b = read_register(0x0B);
    let is_binary = status_b & 0x04 != 0; // otherwise BCD: 0x59 means 59
    let is_24_hour = status_b & 0x02 != 0;
    let decode = |value: u8| if is_binary { value } else { (value & 0x0F) + (value >> 4) * 10 };

    let is_pm = raw[2] & 0x80 != 0;
    let mut hour = decode(raw[2] & 0x7F);
    if !is_24_hour {
        hour %= 12;
        if is_pm {
            hour += 12;
        }
    }

    Time {
        second: decode(raw[0]),
        minute: decode(raw[1]),
        hour,
        day: decode(raw[3]),
        month: decode(raw[4]),
        year: 2000 + decode(raw[5]) as u16,
    }
}
