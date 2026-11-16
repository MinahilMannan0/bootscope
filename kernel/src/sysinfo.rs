//! System information collected at boot.
//!
//! Everything here comes from the bootloader's BootInfo or from hardware we
//! actually read. Nothing is invented: unknown values are reported as unknown.

use crate::rtc::Time;
use bootloader_api::info::{MemoryRegion, MemoryRegionKind};

pub struct SystemInfo {
    pub bootloader_api: (u16, u16, u16),
    pub screen_width: usize,
    pub screen_height: usize,
    pub pixel_format: &'static str,
    pub bytes_per_pixel: usize,
    pub kernel_addr: u64,
    pub kernel_len: u64,
    pub physical_memory_offset: Option<u64>,
    pub rsdp_addr: Option<u64>,
    pub memory_regions: &'static [MemoryRegion],
    pub boot_time: Time,
    pub kernel_hash: Option<u64>,
}

/// Memory totals in bytes, grouped by who owns the memory.
pub struct MemoryTotals {
    pub usable: u64,
    pub bootloader: u64, // used by the bootloader: kernel, page tables, boot info
    pub other: u64,      // reserved by firmware/hardware
}

impl SystemInfo {
    pub fn memory_totals(&self) -> MemoryTotals {
        let mut totals = MemoryTotals { usable: 0, bootloader: 0, other: 0 };
        for region in self.memory_regions {
            let size = region.end - region.start;
            match region.kind {
                MemoryRegionKind::Usable => totals.usable += size,
                MemoryRegionKind::Bootloader => totals.bootloader += size,
                _ => totals.other += size,
            }
        }
        totals
    }
}

pub fn region_kind_name(kind: MemoryRegionKind) -> &'static str {
    match kind {
        MemoryRegionKind::Usable => "Usable",
        MemoryRegionKind::Bootloader => "Bootloader/kernel",
        _ => "Reserved (firmware)",
    }
}
