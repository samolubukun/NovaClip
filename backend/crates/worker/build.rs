use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let backend_dir = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("failed to locate backend directory")
        .to_path_buf();

    let lib_dir = env::var("VOSK_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| backend_dir.join("models").join("vosk-win64-0.3.45"));

    if cfg!(target_os = "windows") {
        let src = lib_dir.join("libvosk.lib");
        let dst = lib_dir.join("vosk.lib");
        if src.exists() && !dst.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rerun-if-env-changed=VOSK_LIB_DIR");
    println!(
        "cargo:rerun-if-changed={}",
        lib_dir.join("libvosk.lib").display()
    );
}
