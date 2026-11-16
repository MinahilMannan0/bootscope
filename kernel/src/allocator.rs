//! Kernel heap: dynamic memory allocation.
//!
//! Until now every buffer had a fixed size chosen in advance. A heap lets the
//! kernel ask for memory while running ("give me 200 bytes for this file") and
//! give it back later. This enables Rust's `Vec`, `String` and `Box`.
//!
//! How it works:
//! 1. From the bootloader's memory map we pick the largest "Usable" RAM region.
//! 2. We give (up to) 16 MiB of it to `linked_list_allocator`, which keeps a
//!    linked list of free blocks: allocating takes a block out of the list,
//!    freeing puts it back.

use crate::sysinfo::SystemInfo;
use bootloader_api::info::MemoryRegionKind;
use linked_list_allocator::LockedHeap;

const MAX_HEAP_SIZE: u64 = 16 * 1024 * 1024;

/// `#[global_allocator]` tells Rust: use this for every Vec/String/Box.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Set up the heap. Returns (physical start address, size in bytes).
pub fn init(info: &SystemInfo) -> Option<(u64, u64)> {
    let offset = info.physical_memory_offset?;

    // Largest usable region, starting at a 4 KiB boundary.
    let region = info
        .memory_regions
        .iter()
        .filter(|r| matches!(r.kind, MemoryRegionKind::Usable))
        .max_by_key(|r| r.end - r.start)?;
    let start = (region.start + 0xFFF) & !0xFFF;
    if start >= region.end {
        return None;
    }
    let size = (region.end - start).min(MAX_HEAP_SIZE);

    // SAFETY: the bootloader reported this RAM as unused ("Usable"), and it maps
    // all physical memory at `offset`, so offset + start is a valid address we
    // now own. We call init exactly once, during boot.
    unsafe {
        ALLOCATOR.lock().init((offset + start) as *mut u8, size as usize);
    }
    Some((start, size))
}

/// (used bytes, free bytes, total bytes) - real numbers from the allocator.
pub fn stats() -> (usize, usize, usize) {
    let heap = ALLOCATOR.lock();
    (heap.used(), heap.free(), heap.size())
}
