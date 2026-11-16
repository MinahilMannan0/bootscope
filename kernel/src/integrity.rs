//! EDUCATIONAL integrity-check demonstration.
//!
//! This is NOT Secure Boot and NOT a cryptographic signature check.
//! It shows the idea behind "measured boot": compute a fingerprint (hash) of
//! the kernel image, then compare it later or against a known-good value.
//!
//! We use FNV-1a, a simple 64-bit hash. It detects accidental changes well,
//! but an attacker could forge it, which is why real systems use SHA-256 plus
//! digital signatures checked by firmware *before* the kernel runs.

use crate::sysinfo::SystemInfo;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a hash of a block of bytes.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// The same hash, but as if ONE bit in the middle of the data were flipped.
/// Used to *simulate* tampering without changing real memory.
pub fn fnv1a_with_one_bit_flipped(bytes: &[u8]) -> u64 {
    let middle = bytes.len() / 2;
    let mut hash = FNV_OFFSET;
    for (i, &byte) in bytes.iter().enumerate() {
        let byte = if i == middle { byte ^ 0x01 } else { byte };
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// The kernel's ELF file as the bootloader loaded it into memory.
/// Returns None if physical memory is not mapped.
pub fn kernel_image(info: &SystemInfo) -> Option<&'static [u8]> {
    let offset = info.physical_memory_offset?;
    let start = offset.checked_add(info.kernel_addr)?;
    // SAFETY: we asked the bootloader (BOOTLOADER_CONFIG in main.rs) to map all
    // physical memory starting at `offset`, and it reports that the kernel file
    // occupies kernel_addr .. kernel_addr + kernel_len. We only read it.
    Some(unsafe { core::slice::from_raw_parts(start as *const u8, info.kernel_len as usize) })
}
