# Native Windows storage build

HOTR-02 uses rusqlite 0.40.2 with its supported external SQLCipher link feature. The crates.io native bundle was inspected and still contained SQLCipher 4.14.0. HOTR instead generates and compiles upstream SQLCipher 4.18.0 and statically links its OpenSSL provider. There is no plaintext fallback.

Prerequisites: Windows x64, installed Rust stable with rustfmt/Clippy, MSVC x64 Build Tools, PowerShell, tar, and Python's standard SQLite module for the independent rejection test. The reference compiler is Rust 1.98.0 and MSVC 19.50.35730. This is a development build; clean-machine packaging and other operating systems have separate roster gates.

From the canonical repository in PowerShell:

```powershell
. ./.cargo/prepare-native.ps1
cargo build --release --locked
cargo fmt --check
cargo clippy --release --locked --all-targets -- -D warnings
cargo test --release --locked -- --test-threads=1
./work/hotr-build/target/release/hotr.exe native-info
```

The preparation script checks the project path and rejects reparse points before native build operations. It uses the installed MSVC toolchain and a checksum-verified portable Perl runtime within `work/hotr-tool-cache`. It does not install Perl, change PATH persistently, or alter existing application settings. Cargo home, build outputs, and temporary files are project-local. Native compiler tools manage their own generated files within this approved build boundary. Historical evidence and completed synthetic vaults are retained.

Native dependency pins:

| Input | Pin / provenance |
|---|---|
| SQLCipher | 4.18.0, upstream tag source commit `63697beb0fafcb61faa7a3e6fd267036548ab11b` |
| SQLCipher source archive SHA-256 | `1df02d1b346fa27feaf2da2cb2c0d8209e788248e461ec288718aa5d3e9643e5` |
| libsqlite3-sys bindings | 0.38.2; native library supplied by HOTR's prerequisite build |
| OpenSSL | 4.0.2 via openssl-src 400.0.1; exact registry checksum in native builder Cargo.lock |
| Portable Perl | Strawberry Perl 5.42.3.1; only the Perl subtree extracted |
| Perl archive SHA-256 | `6a081a811781c30aca51dbc036afd93092af91e3297901f02c17043795a10690` |

The SQLCipher source archive is about 19 MB; the portable Perl download is about 305 MB. One project cache is retained. OpenSSL is built from locked source, rather than reusing an unrelated application's crypto DLL. Without NASM, the native build uses OpenSSL's supported no-assembly configuration; performance claims await the roster's measured benchmarks.

Required compiler settings include SQLCipher codec and OpenSSL provider, SQLCipher extra initialization/shutdown, FTS5, thread safety, API armor, disabled extension loading, disabled double-quoted string literals, and `SQLITE_TEMP_STORE=3` to force SQLite temporary tables into memory. Runtime configuration requires WAL, synchronous FULL, foreign keys, memory security, trusted-schema off, and redacted errors. Passphrase bytes go directly to `sqlite3_key` and are never assembled into SQL.

The native smoke test creates a unique synthetic run directory, tests keyless/wrong-key/ordinary-SQLite rejection, checks FTS after reopening, checks integrity, scans live DB/WAL/SHM and closed storage for dynamically generated UTF-8/UTF-16 canaries and key bytes, and retains a sanitized result manifest. It refuses to silently create a missing database during an open operation. This proves synthetic application-managed file storage, not protection of an unlocked process, OS pagefile/hibernation, or client applications' own copies.

Current upstream review: SQLCipher 4.15 fixed an export defensive-mode bypass; 4.17 incorporated SQLite FTS5 CVE fixes; 4.18 incorporates SQLite 3.53.4 and a Windows logging/memory-security fix. See [Zetetic release notes](https://www.zetetic.net/blog/) and the [official SQLCipher build instructions](https://github.com/sqlcipher/sqlcipher/tree/v4.18.0). This dependency review is a foundation check, not a completed HOTR-31 security audit.

The [OpenSSL 4.0 vulnerability record](https://openssl-library.org/news/vulnerabilities-4.0/) was checked on 2026-09-05. Its August 25 fixes are included in 4.0.2. An older cached 3.x source was not selected as a shortcut around the native prerequisite build. Native compilation and actual runtime tests remain necessary to establish compatibility with SQLCipher's provider interface.
