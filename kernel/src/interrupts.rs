//! Interrupts and CPU exceptions.
//!
//! An INTERRUPT is a signal that makes the CPU pause what it is doing, run a
//! small "handler" function, and then continue. Two kinds matter here:
//!
//! * CPU exceptions - the CPU itself reports a problem (e.g. a page fault when
//!   code touches memory that is not mapped).
//! * Hardware interrupts (IRQs) - a device asks for attention: the timer chip
//!   ticks, or the keyboard has a new key.
//!
//! The pieces:
//! * IDT (Interrupt Descriptor Table): a table of 256 entries that tells the CPU
//!   which handler function to run for each interrupt number ("vector").
//! * PIC (8259 Programmable Interrupt Controller): collects device IRQs and
//!   forwards them to the CPU. We remap IRQ 0-15 to vectors 32-47, because
//!   vectors 0-31 are reserved for CPU exceptions.
//! * PIT (8253/8254 Programmable Interval Timer): we program it to tick 100
//!   times per second (IRQ 0).

use crate::port::{read_port, write_port};
use core::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use pic8259::ChainedPics;
use spin::{Lazy, Mutex};
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32; // IRQ 0-7  -> vectors 32-39
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8; // IRQ 8-15 -> vectors 40-47
const TIMER_VECTOR: u8 = PIC_1_OFFSET; // IRQ 0
const KEYBOARD_VECTOR: u8 = PIC_1_OFFSET + 1; // IRQ 1

pub const TIMER_HZ: u64 = 100;

static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// Counters, updated by the handlers (atomics are safe to share with handlers).
static TICKS: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_IRQS: AtomicU64 = AtomicU64::new(0);
static BREAKPOINTS: AtomicU64 = AtomicU64::new(0);

/// The table of handlers. `Lazy` builds it the first time it is used.
static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    // CPU exceptions
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.general_protection_fault.set_handler_fn(general_protection_handler);
    idt.double_fault.set_handler_fn(double_fault_handler);
    // Hardware interrupts
    idt[TIMER_VECTOR].set_handler_fn(timer_handler);
    idt[KEYBOARD_VECTOR].set_handler_fn(keyboard_handler);
    idt
});

/// Load the IDT, set up the PIC and timer, then switch interrupts on.
pub fn init() {
    IDT.load();
    // SAFETY: standard PIC/PIT ports, programmed once at boot while
    // interrupts are still disabled.
    unsafe {
        PICS.lock().initialize();
        // Masks: a 0 bit = IRQ allowed. Allow only IRQ 0 (timer) and 1 (keyboard).
        write_port(0x21, 0b1111_1100);
        write_port(0xA1, 0b1111_1111);
        // PIT channel 0, mode 3 (square wave), divisor for TIMER_HZ ticks per second.
        let divisor = (1_193_182 / TIMER_HZ) as u16;
        write_port(0x43, 0x36);
        write_port(0x40, (divisor & 0xFF) as u8);
        write_port(0x40, (divisor >> 8) as u8);
    }
    x86_64::instructions::interrupts::enable(); // the `sti` instruction
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn keyboard_irqs() -> u64 {
    KEYBOARD_IRQS.load(Ordering::Relaxed)
}

pub fn breakpoints() -> u64 {
    BREAKPOINTS.load(Ordering::Relaxed)
}

/// Sleep until the next interrupt arrives (saves CPU time).
/// Interrupts are disabled while we check for waiting keys, then `sti; hlt`
/// re-enables them and halts in one step, so no key press can be missed.
pub fn wait_for_interrupt() {
    x86_64::instructions::interrupts::disable();
    if has_scancode() {
        x86_64::instructions::interrupts::enable();
    } else {
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

/// Wait for a number of timer ticks (1 tick = 10 ms).
pub fn sleep_ticks(count: u64) {
    let end = ticks() + count;
    while ticks() < end {
        x86_64::instructions::hlt(); // woken by the next timer tick
    }
}

// ---------------------------------------------------------------------------
// Keyboard queue: the keyboard handler puts scancodes in, the main loop takes
// them out. A "ring buffer" with one writer and one reader needs no lock.
// ---------------------------------------------------------------------------

const QUEUE_SIZE: usize = 64;
static QUEUE: [AtomicU8; QUEUE_SIZE] = [const { AtomicU8::new(0) }; QUEUE_SIZE];
static HEAD: AtomicUsize = AtomicUsize::new(0); // next slot to write (handler)
static TAIL: AtomicUsize = AtomicUsize::new(0); // next slot to read (main loop)

fn push_scancode(scancode: u8) {
    let head = HEAD.load(Ordering::Relaxed);
    let next = (head + 1) % QUEUE_SIZE;
    if next == TAIL.load(Ordering::Acquire) {
        return; // queue full: drop the key rather than overwrite
    }
    QUEUE[head].store(scancode, Ordering::Relaxed);
    HEAD.store(next, Ordering::Release);
}

fn has_scancode() -> bool {
    TAIL.load(Ordering::Relaxed) != HEAD.load(Ordering::Acquire)
}

/// Take the oldest scancode out of the queue, if any.
pub fn pop_scancode() -> Option<u8> {
    let tail = TAIL.load(Ordering::Relaxed);
    if tail == HEAD.load(Ordering::Acquire) {
        return None;
    }
    let scancode = QUEUE[tail].load(Ordering::Relaxed);
    TAIL.store((tail + 1) % QUEUE_SIZE, Ordering::Release);
    Some(scancode)
}

// ---------------------------------------------------------------------------
// Handlers. `extern "x86-interrupt"` makes Rust save and restore all CPU
// registers, so the interrupted code continues as if nothing happened.
// ---------------------------------------------------------------------------

extern "x86-interrupt" fn timer_handler(_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    // SAFETY: tell the PIC we finished, so it can send the next interrupt.
    unsafe { PICS.lock().notify_end_of_interrupt(TIMER_VECTOR) };
}

extern "x86-interrupt" fn keyboard_handler(_frame: InterruptStackFrame) {
    // SAFETY: port 0x60 holds the scancode that caused this interrupt.
    let scancode = unsafe { read_port(0x60) };
    push_scancode(scancode);
    KEYBOARD_IRQS.fetch_add(1, Ordering::Relaxed);
    unsafe { PICS.lock().notify_end_of_interrupt(KEYBOARD_VECTOR) };
}

/// Breakpoint (`int3`): harmless; we count it and continue.
extern "x86-interrupt" fn breakpoint_handler(_frame: InterruptStackFrame) {
    BREAKPOINTS.fetch_add(1, Ordering::Relaxed);
}

/// Page fault: code touched memory that is not mapped. Fatal for us.
extern "x86-interrupt" fn page_fault_handler(frame: InterruptStackFrame, error: PageFaultErrorCode) {
    panic!("CPU EXCEPTION: PAGE FAULT\naccessed address: {:?}\nerror code: {:?}\n{:#?}",
        Cr2::read(), error, frame);
}

extern "x86-interrupt" fn general_protection_handler(frame: InterruptStackFrame, error: u64) {
    panic!("CPU EXCEPTION: GENERAL PROTECTION FAULT (error code {error})\n{frame:#?}");
}

/// Double fault: an exception happened while handling another exception.
extern "x86-interrupt" fn double_fault_handler(frame: InterruptStackFrame, _error: u64) -> ! {
    panic!("CPU EXCEPTION: DOUBLE FAULT\n{frame:#?}");
}
