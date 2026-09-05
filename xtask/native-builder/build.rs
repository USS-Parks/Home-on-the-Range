use std::{env, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.parent().unwrap().parent().unwrap();
    let native = root.join("work/hotr-build/native");
    let source = native.join("sqlite3.c");
    println!("cargo:rerun-if-changed={}", source.display());
    assert!(
        source.is_file(),
        "Run .cargo/prepare-native.ps1 before Cargo"
    );
    let text = std::fs::read_to_string(&source).expect("SQLCipher source must be readable");
    assert!(
        text.contains("#define CIPHER_VERSION_NUMBER 4.18.0"),
        "Unapproved SQLCipher source"
    );
    assert_eq!(env::var("TARGET").unwrap(), "x86_64-pc-windows-msvc");
    let crypto = openssl_src::Build::new().build();
    cc::Build::new()
        .file(&source)
        .include(crypto.include_dir())
        .out_dir(&native)
        .define("SQLITE_HAS_CODEC", None)
        .define("SQLCIPHER_CRYPTO_OPENSSL", None)
        .define("SQLITE_TEMP_STORE", "3")
        .define("SQLITE_THREADSAFE", "1")
        .define("SQLITE_EXTRA_INIT", "sqlcipher_extra_init")
        .define("SQLITE_EXTRA_SHUTDOWN", "sqlcipher_extra_shutdown")
        .define("SQLITE_ENABLE_FTS5", None)
        .define("SQLITE_ENABLE_API_ARMOR", None)
        .define("SQLITE_ENABLE_COLUMN_METADATA", None)
        .define("SQLITE_OMIT_LOAD_EXTENSION", None)
        .define("SQLITE_DQS", "0")
        .warnings(false)
        .compile("sqlcipher");
    std::fs::copy(
        crypto.lib_dir().join("libcrypto.lib"),
        native.join("libcrypto.lib"),
    )
    .unwrap();
    crypto.print_cargo_metadata();
}
