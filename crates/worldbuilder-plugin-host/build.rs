use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/timeline-service.wat");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let wasm = wat::parse_file("assets/timeline-service.wat")
        .expect("the bundled Timeline service WAT must compile");
    fs::write(out_dir.join("timeline-service.wasm"), wasm)
        .expect("write bundled Timeline service WASM");
}
