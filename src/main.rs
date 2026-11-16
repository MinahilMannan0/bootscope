use std::process::Command;

fn main() {
    let bios_path = env!("BIOS_PATH");
    println!("Booting BootScope from: {bios_path}");
    println!("Known-good kernel hash (computed on Windows): {}", env!("KERNEL_HASH"));
    println!("Kernel file size: {} bytes", env!("KERNEL_SIZE"));

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive")
        .arg(format!("format=raw,file={bios_path}"))
        // Give the virtual clock chip your local Windows time instead of UTC.
        .arg("-rtc")
        .arg("base=localtime");

    // `cargo run -- --fullscreen` starts QEMU in full screen (for presenting).
    // Press Ctrl+Alt+F to leave full screen.
    if std::env::args().any(|arg| arg == "--fullscreen") {
        qemu.arg("-full-screen");
    }

    qemu.status().expect("could not start QEMU");
}
