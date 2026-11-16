use std::process::Command;

fn main() {
    let bios_path = env!("BIOS_PATH");
    println!("Booting BootScope from: {bios_path}");

    Command::new("qemu-system-x86_64")
        .arg("-drive")
        .arg(format!("format=raw,file={bios_path}"))
        .status()
        .expect("could not start QEMU");
}