use std::{env, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let native = root.join("work/hotr-build/native");
    println!(
        "cargo:rerun-if-changed={}",
        native.join("libcrypto.lib").display()
    );
    assert!(
        native.join("libcrypto.lib").is_file(),
        "Run .cargo/prepare-native.ps1 first"
    );
    assert_eq!(
        env::var("TARGET").unwrap(),
        "x86_64-pc-windows-msvc",
        "This native gate supports Windows x64; other platforms have later gates"
    );
    println!("cargo:rustc-link-search=native={}", native.display());
    println!("cargo:rustc-link-lib=static=libcrypto");
    for lib in ["user32", "crypt32", "advapi32", "ws2_32"] {
        println!("cargo:rustc-link-lib={lib}");
    }
    if env::var("PROFILE").as_deref() == Ok("release") {
        // The vendored OpenSSL build does not distribute its intermediate PDB.
        // Release executables intentionally contain no debug information.
        println!("cargo:rustc-link-arg=/DEBUG:NONE");
    }
}
