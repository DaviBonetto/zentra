use std::{env, path::PathBuf};

fn main() {
    tauri_build::build();
    println!("cargo:rustc-check-cfg=cfg(mobile)");
    println!("cargo:rerun-if-env-changed=VOSK_LIB_DIR");
    println!("cargo:rerun-if-env-changed=VOSK_LIB_PATH");

    if let Ok(lib_path) = env::var("VOSK_LIB_PATH") {
        let path = PathBuf::from(lib_path);
        if let Some(dir) = path.parent() {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    }

    if let Ok(lib_dir) = env::var("VOSK_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let local_dir = manifest_dir.join("libs").join("vosk");
    if local_dir.exists() {
        println!("cargo:rustc-link-search=native={}", local_dir.display());
    }
}
