pub mod limits;

#[cfg(test)]
extern crate self as hotr_xtask;
#[cfg(test)]
#[path = "../tests/contracts.rs"]
mod contracts;

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    os::windows::fs::MetadataExt,
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const LOG_LIMIT: usize = 8 * 1024 * 1024;
pub const FIXTURE_SECRET: &str = "hotr-synthetic-secret-64e9ec26";

#[derive(Clone)]
pub struct Guard {
    root: PathBuf,
}

impl Guard {
    pub fn new(root: &Path) -> io::Result<Self> {
        for path in root.ancestors() {
            if fs::symlink_metadata(path)?.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other("root reparse point refused"));
            }
        }
        let root = root.canonicalize()?;
        let guard = Self { root };
        guard.checked(Path::new("work"))?;
        Ok(guard)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn checked(&self, relative: &Path) -> io::Result<PathBuf> {
        if !relative
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
            || relative.components().next() != Some(Component::Normal("work".as_ref()))
        {
            return Err(io::Error::other(
                "path outside owned work directory refused",
            ));
        }
        let mut path = self.root.clone();
        for component in relative.components() {
            path.push(component);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                    return Err(io::Error::other("reparse point refused"));
                }
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => (),
            }
        }
        Ok(path)
    }
    pub fn new_run(&self, prompt: &str) -> io::Result<PathBuf> {
        if !matches!(
            prompt,
            "HOTR-02" | "HOTR-03" | "HOTR-04" | "HOTR-05" | "HOTR-03-fault"
        ) {
            return Err(io::Error::other("unregistered prompt refused"));
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let parent = self.checked(Path::new("work/hotr-evidence"))?;
        fs::create_dir_all(&parent)?;
        let path = parent.join(format!("{prompt}-{}-{stamp}", std::process::id()));
        fs::create_dir(&path)?;
        write_new(
            &path.join("SYNTHETIC-ONLY"),
            b"HOTR verification run; no real user data\n",
        )?;
        Ok(path)
    }
    pub fn budget(&self) -> io::Result<(u64, u64)> {
        let bytes = size(&self.checked(Path::new("work"))?)?;
        let free = limits::free_bytes(&self.root)?;
        check_budget(bytes, free)?;
        Ok((bytes, free))
    }
}

pub fn check_budget(used: u64, free: u64) -> io::Result<()> {
    if used >= limits::MAX_DISK_BYTES || free < limits::MIN_FREE_BYTES {
        Err(io::Error::other("disk resource ceiling reached"))
    } else {
        Ok(())
    }
}

fn size(path: &Path) -> io::Result<u64> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(io::Error::other("reparse point refused"));
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut sum = 0u64;
    for item in fs::read_dir(path)? {
        sum = sum
            .checked_add(size(&item?.path())?)
            .ok_or_else(|| io::Error::other("size overflow"))?;
    }
    Ok(sum)
}

pub fn write_new(path: &Path, data: &[u8]) -> io::Result<()> {
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(data)
}

pub fn hash(path: &Path) -> io::Result<String> {
    let mut digest = Sha256::new();
    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let len = file.read(&mut buffer)?;
        if len == 0 {
            break;
        }
        digest.update(&buffer[..len]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[derive(Serialize, PartialEq)]
pub struct SourceSnapshot {
    pub commit: String,
    pub dirty: bool,
    pub normalized_source_sha256: BTreeMap<String, String>,
    pub native_sha256: BTreeMap<String, String>,
}

pub fn snapshot(guard: &Guard) -> io::Result<SourceSnapshot> {
    let git = |args: &[&str]| -> io::Result<Vec<u8>> {
        let output = Command::new("git")
            .args(args)
            .current_dir(guard.root())
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other("source identity check failed"));
        }
        Ok(output.stdout)
    };
    let commit = String::from_utf8(git(&["rev-parse", "HEAD"])?)
        .map_err(io::Error::other)?
        .trim()
        .to_owned();
    let dirty = !git(&["status", "--porcelain"])?.is_empty();
    let listing = git(&[
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ])?;
    let mut sources = BTreeMap::new();
    for entry in listing.split(|b| *b == 0).filter(|b| !b.is_empty()) {
        let name = std::str::from_utf8(entry).map_err(io::Error::other)?;
        let path = Path::new(name);
        if !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("rs" | "sql" | "toml" | "lock" | "py" | "ps1" | "yml")
        ) {
            continue;
        }
        if !path.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err(io::Error::other("invalid source path"));
        }
        let absolute = guard.root().join(path);
        if fs::symlink_metadata(&absolute)?.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("source reparse point refused"));
        }
        let text = fs::read_to_string(absolute)?.replace("\r\n", "\n");
        sources.insert(
            name.to_owned(),
            format!("{:x}", Sha256::digest(text.as_bytes())),
        );
    }
    let mut native = BTreeMap::new();
    for name in ["sqlite3.c", "sqlcipher.lib", "libcrypto.lib"] {
        let path = guard.checked(&PathBuf::from("work/hotr-build/native").join(name))?;
        native.insert(name.to_owned(), hash(&path)?);
    }
    Ok(SourceSnapshot {
        commit,
        dirty,
        normalized_source_sha256: sources,
        native_sha256: native,
    })
}

pub fn seeded_record(seed: u64, index: u64) -> String {
    let digest = Sha256::digest([seed.to_le_bytes(), index.to_le_bytes()].concat());
    let token = format!("{digest:x}");
    format!(
        "synthetic-{index} namespace-{} 東京 café ../data [{}] {}",
        index % 10,
        seed,
        token.repeat(24)
    )
}

pub fn redact(bytes: &[u8], secrets: &[&str], truncated: bool) -> String {
    let complete = if truncated {
        &bytes[..bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1)]
    } else {
        bytes
    };
    let mut text = String::from_utf8_lossy(complete).into_owned();
    for secret in secrets {
        if !secret.is_empty() {
            text = text.replace(secret, "[REDACTED]");
        }
    }
    text.lines()
        .map(|line| {
            if line.to_ascii_lowercase().contains("authorization:") {
                "[REDACTED AUTHORIZATION]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn terminate_owned(child: &mut Child, requested_pid: u32) -> io::Result<()> {
    if child.id() != requested_pid || requested_pid == std::process::id() {
        return Err(io::Error::other("unrelated PID refused"));
    }
    child.kill()?;
    child.wait()?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub label: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub pid: u32,
    pub elapsed_ms: u128,
    pub exit_code: Option<i32>,
    pub failure: Option<String>,
    pub log_bytes: usize,
    pub stored_log_bytes: usize,
}

impl Outcome {
    pub fn ensure_pass(&self) -> io::Result<()> {
        if self.exit_code == Some(0) && self.failure.is_none() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "gate command failed: {}",
                self.label
            )))
        }
    }
}

pub fn ensure_required(outcomes: &[Outcome], labels: &[&str]) -> io::Result<()> {
    for label in labels {
        let matches: Vec<_> = outcomes
            .iter()
            .filter(|outcome| outcome.label == *label)
            .collect();
        if matches.len() != 1 {
            return Err(io::Error::other("required command skipped or duplicated"));
        }
        matches[0].ensure_pass()?;
    }
    Ok(())
}

pub fn run(
    guard: &Guard,
    directory: &Path,
    label: &str,
    executable: &Path,
    args: &[&str],
    timeout: Duration,
) -> io::Result<Outcome> {
    guard.budget()?;
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(io::Error::other("invalid command label"));
    }
    let relative = directory
        .strip_prefix(guard.root())
        .map_err(io::Error::other)?;
    guard.checked(relative)?;
    if !directory.join("SYNTHETIC-ONLY").is_file() {
        return Err(io::Error::other("unowned output directory refused"));
    }
    let start = Instant::now();
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(guard.root())
        .env(
            "CARGO_HOME",
            guard.checked(Path::new("work/hotr-tool-cache/cargo"))?,
        )
        .env("TEMP", guard.checked(Path::new("work/hotr-build/tmp"))?)
        .env("TMP", guard.checked(Path::new("work/hotr-build/tmp"))?)
        .env("CARGO_BUILD_JOBS", "4")
        .env("RUST_TEST_THREADS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let pid = child.id();
    let (send, receive) = mpsc::sync_channel(32);
    let streams: Vec<Box<dyn Read + Send>> = vec![
        Box::new(child.stdout.take().unwrap()),
        Box::new(child.stderr.take().unwrap()),
    ];
    for (index, mut stream) in streams.into_iter().enumerate() {
        let sender = send.clone();
        thread::spawn(move || {
            let mut buffer = [0; 8192];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(len) => {
                        if sender.send((index, buffer[..len].to_vec())).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }
    drop(send);
    let mut buffers = [Vec::new(), Vec::new()];
    let mut total = 0;
    let mut failure = None;
    let mut status = None;
    let mut last_budget = Instant::now();
    loop {
        match receive.recv_timeout(Duration::from_millis(10)) {
            Ok((index, data)) => {
                if total + data.len() > LOG_LIMIT {
                    failure = Some("log limit".to_owned());
                } else {
                    total += data.len();
                    buffers[index].extend_from_slice(&data);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) if status.is_some() => break,
            Err(_) => (),
        }
        if status.is_none() {
            status = child.try_wait()?;
        }
        if start.elapsed() > timeout {
            failure = Some("timeout".to_owned());
        }
        if last_budget.elapsed() > Duration::from_secs(5) {
            if guard.budget().is_err() {
                failure = Some("resource budget".to_owned());
            }
            last_budget = Instant::now();
        }
        if failure.is_some() {
            if status.is_none() {
                terminate_owned(&mut child, pid)?;
            }
            break;
        }
    }
    let mut logs = format!(
        "stdout:\n{}\nstderr:\n{}\n",
        redact(&buffers[0], &[FIXTURE_SECRET], failure.is_some()),
        redact(&buffers[1], &[FIXTURE_SECRET], failure.is_some())
    );
    if logs.len() > LOG_LIMIT {
        let end = logs.as_bytes()[..LOG_LIMIT]
            .iter()
            .rposition(|b| *b == b'\n')
            .unwrap_or(0);
        logs.truncate(end);
        failure = Some("log limit".to_owned());
    }
    write_new(&directory.join(format!("{label}.txt")), logs.as_bytes())?;
    Ok(Outcome {
        label: label.to_owned(),
        executable: executable
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        arguments: args.iter().map(|a| (*a).to_owned()).collect(),
        pid,
        elapsed_ms: start.elapsed().as_millis(),
        exit_code: status.and_then(|s| s.code()),
        failure,
        log_bytes: total,
        stored_log_bytes: logs.len(),
    })
}
