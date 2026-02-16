// Build script to ensure all FFI exports are included in the DLL on Windows
use std::env;

fn main() {
    // Only configure exports on Windows
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Check if we're using GNU or MSVC toolchain
        let target = env::var("TARGET").unwrap_or_default();

        if target.contains("gnu") {
            // GNU linker: Export all symbols marked with #[no_mangle]
            println!("cargo:rustc-cdylib-link-arg=-Wl,--export-all-symbols");
        } else if target.contains("msvc") {
            // MSVC linker: Use .def file
            println!("cargo:rustc-cdylib-link-arg=/DEF:tower_core.def");
        }
    }

    // Rerun if the .def file changes
    println!("cargo:rerun-if-changed=tower_core.def");
}
