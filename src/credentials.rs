//! User-scoped Windows DPAPI application credentials; no machine-wide scope.
use crate::{owner, windows_security as security};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom, CRYPT_INTEGER_BLOB,
        CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialProfile {
    pub format: u32,
    pub client_id: String,
    pub port: u16,
    pub protected_token: Vec<u8>,
}

pub fn random_hex(bytes: usize) -> io::Result<Zeroizing<String>> {
    if !(16..=32).contains(&bytes) {
        return Err(io::Error::other("random input size rejected"));
    }
    let mut random = Zeroizing::new(vec![0u8; bytes]);
    // SAFETY: writable buffer has the declared bounded length; null provider
    // selects Windows' system-preferred cryptographic RNG.
    let status = unsafe {
        BCryptGenRandom(
            ptr::null_mut(),
            random.as_mut_ptr(),
            bytes as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(io::Error::other("Windows random generator unavailable"));
    }
    Ok(Zeroizing::new(
        random.iter().map(|byte| format!("{byte:02x}")).collect(),
    ))
}

pub fn token_hash(token: &str) -> Option<[u8; 32]> {
    (token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then(|| Sha256::digest(token.as_bytes()).into())
}

fn dpapi(input: &[u8], protect: bool) -> io::Result<Zeroizing<Vec<u8>>> {
    if input.is_empty() || input.len() > 4096 {
        return Err(io::Error::other("credential size rejected"));
    }
    let source = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr().cast_mut(),
    };
    let mut result = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    // SAFETY: input is a live bounded buffer, output is OS allocated, optional
    // pointers are null. The output is copied, zeroed and freed exactly once.
    unsafe {
        let ok = if protect {
            CryptProtectData(
                &source,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut result,
            )
        } else {
            CryptUnprotectData(
                &source,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut result,
            )
        };
        if ok == 0 {
            return Err(io::Error::other("Windows credential protection rejected"));
        }
        if result.pbData.is_null() {
            return Err(io::Error::other("Windows credential result rejected"));
        }
        let output = std::slice::from_raw_parts_mut(result.pbData, result.cbData as usize);
        let value = if output.len() <= 4096 {
            Ok(Zeroizing::new(output.to_vec()))
        } else {
            Err(io::Error::other("Windows credential result too large"))
        };
        output.zeroize();
        LocalFree(result.pbData.cast());
        value
    }
}

pub(crate) fn protect(token: &str, client_id: String, port: u16) -> io::Result<CredentialProfile> {
    if token_hash(token).is_none() || port == 0 {
        return Err(io::Error::other("credential fields rejected"));
    }
    Ok(CredentialProfile {
        format: 1,
        client_id,
        port,
        protected_token: dpapi(token.as_bytes(), true)?.to_vec(),
    })
}

pub fn unprotect(profile: &CredentialProfile) -> io::Result<Zeroizing<String>> {
    if profile.format != 1
        || profile.port == 0
        || !crate::schema::valid_identifier(&profile.client_id, false)
    {
        return Err(io::Error::other("credential profile rejected"));
    }
    let bytes = dpapi(&profile.protected_token, false)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| io::Error::other("credential encoding rejected"))?;
    if token_hash(value).is_none() {
        return Err(io::Error::other("credential token rejected"));
    }
    Ok(Zeroizing::new(value.to_owned()))
}

pub fn save(path: &Path, profile: &CredentialProfile) -> io::Result<()> {
    let path = owner::safe_absolute(path)?;
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(io::Error::other("credential parent must exist"));
    }
    let bytes = serde_json::to_vec(profile)
        .map_err(|_| io::Error::other("credential serialization failed"))?;
    if bytes.len() > 8192 {
        return Err(io::Error::other("credential profile too large"));
    }
    let mut file = security::create_file(&path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    security::verify_file_owner(&path, false)
}

pub fn load(path: &Path) -> io::Result<CredentialProfile> {
    let path = owner::safe_absolute(path)?;
    if !fs::symlink_metadata(&path)?.file_type().is_file() {
        return Err(io::Error::other("credential file type rejected"));
    }
    security::verify_file_owner(&path, false)?;
    let mut bytes = Vec::new();
    fs::File::open(&path)?.take(8193).read_to_end(&mut bytes)?;
    if bytes.len() > 8192 {
        return Err(io::Error::other("credential profile too large"));
    }
    serde_json::from_slice(&bytes).map_err(|_| io::Error::other("credential profile rejected"))
}
