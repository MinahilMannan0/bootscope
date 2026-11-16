use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let kernel = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel").unwrap());

    let bios_path = out_dir.join("bootscope-bios.img");
    bootloader::BiosBoot::new(&kernel)
        .create_disk_image(&bios_path)
        .expect("failed to create BIOS disk image");
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());

    // Known-good fingerprint of the kernel file, computed here on Windows with
    // the same FNV-1a hash the kernel uses (educational integrity demo).
    let bytes = std::fs::read(&kernel).expect("failed to read kernel file");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in &bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    println!("cargo:rustc-env=KERNEL_HASH={hash:#018x}");
    println!("cargo:rustc-env=KERNEL_SIZE={}", bytes.len());
}
