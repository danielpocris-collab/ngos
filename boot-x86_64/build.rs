use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../platform-x86_64/linker/kernel-x86_64.ld");
    println!("cargo:rerun-if-changed=src/entry.S");
    println!("cargo:rerun-if-changed=src/smp_trampoline.S");
    println!("cargo:rerun-if-changed=src/traps.S");

    let target = env::var("TARGET").unwrap_or_default();
    if !(target.contains("x86_64-ngos-kernel") || target == "x86_64-unknown-none") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let linker_script = manifest_dir
        .join("../platform-x86_64/linker/kernel-x86_64.ld")
        .canonicalize()
        .expect("linker script path");

    println!(
        "cargo:rustc-link-arg-bin=ngos-boot-x86_64=-T{}",
        linker_script.display()
    );
}
