//! Small Windows ownership boundary. No impersonation survives an await.
use std::{
    ffi::c_void,
    fs::File,
    io, mem,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetNamedSecurityInfoW, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, GetAce, GetLengthSid, GetSecurityDescriptorControl,
        GetSecurityDescriptorDacl, GetSecurityDescriptorOwner, GetTokenInformation, IsValidAcl,
        IsValidSid, OWNER_SECURITY_INFORMATION, RevertToSelf, SECURITY_ATTRIBUTES, TOKEN_QUERY,
        TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::{
        CREATE_NEW, CreateDirectoryW, CreateFileW, FILE_ALL_ACCESS, FILE_ATTRIBUTE_NORMAL,
        FILE_FLAG_OPEN_REPARSE_POINT,
    },
    System::{
        Pipes::{GetNamedPipeServerProcessId, ImpersonateNamedPipeClient},
        Threading::{
            GetCurrentProcess, GetCurrentThread, OpenProcess, OpenProcessToken, OpenThreadToken,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(Some(0)).collect()
}

unsafe fn sid_string(sid: *mut c_void) -> io::Result<String> {
    // SAFETY: caller supplies a valid SID belonging to a live token/descriptor.
    unsafe {
        let mut text = ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut text) == 0 {
            return Err(io::Error::last_os_error());
        }
        let result = take_string(text);
        LocalFree(text.cast());
        result
    }
}

unsafe fn take_string(text: *const u16) -> io::Result<String> {
    // SAFETY: Windows provides a terminated allocated string; retain a bound.
    unsafe {
        for length in 0..4096 {
            if *text.add(length) == 0 {
                return String::from_utf16(std::slice::from_raw_parts(text, length))
                    .map_err(io::Error::other);
            }
        }
        Err(io::Error::other("Windows security string exceeded bound"))
    }
}

fn token_sid(token: HANDLE) -> io::Result<String> {
    // SAFETY: token is live, aligned output storage is sized by the OS query.
    unsafe {
        let mut needed = 0;
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed);
        if needed == 0 || needed > 4096 {
            return Err(io::Error::other("invalid token metadata size"));
        }
        let mut storage = vec![0usize; (needed as usize).div_ceil(mem::size_of::<usize>())];
        if GetTokenInformation(
            token,
            TokenUser,
            storage.as_mut_ptr().cast(),
            needed,
            &mut needed,
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        sid_string((*(storage.as_ptr().cast::<TOKEN_USER>())).User.Sid)
    }
}

fn process_sid(process: HANDLE) -> io::Result<String> {
    // SAFETY: supplied process handle is live; newly opened token is closed once.
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return Err(io::Error::last_os_error());
        }
        let result = token_sid(token);
        CloseHandle(token);
        result
    }
}

pub fn current_sid() -> io::Result<String> {
    // SAFETY: this pseudo-handle always identifies the current process.
    process_sid(unsafe { GetCurrentProcess() })
}

pub fn pipe_server_sid(pipe: &impl AsRawHandle) -> io::Result<String> {
    // SAFETY: pipe remains live; the process handle is owned and closed by RAII.
    unsafe {
        let mut pid = 0;
        if GetNamedPipeServerProcessId(pipe.as_raw_handle().cast(), &mut pid) == 0 {
            return Err(io::Error::last_os_error());
        }
        let raw = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        let process = OwnedHandle::from_raw_handle(raw.cast());
        process_sid(process.as_raw_handle().cast())
    }
}

pub fn pipe_client_sid(pipe: &impl AsRawHandle) -> io::Result<String> {
    // SAFETY: impersonation and reversion are synchronous on this thread. There
    // are no awaits/callbacks while a client identity is active.
    unsafe {
        if ImpersonateNamedPipeClient(pipe.as_raw_handle().cast()) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut token = ptr::null_mut();
        let result = if OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) == 0 {
            Err(io::Error::last_os_error())
        } else {
            let result = token_sid(token);
            CloseHandle(token);
            result
        };
        if RevertToSelf() == 0 {
            std::process::exit(1);
        }
        result
    }
}

pub struct Descriptor {
    pointer: *mut c_void,
}

impl Descriptor {
    pub fn owner_only(directory: bool) -> io::Result<Self> {
        let sid = current_sid()?;
        let flags = if directory { "OICI" } else { "" };
        let sddl = format!("O:{sid}D:P(A;{flags};FA;;;{sid})(A;{flags};FA;;;SY)");
        Self::from_sddl(&sddl)
    }
    fn from_sddl(sddl: &str) -> io::Result<Self> {
        let text = wide(sddl.as_ref());
        let mut pointer = ptr::null_mut();
        // SAFETY: terminated SDDL, valid output; LocalFree in Drop owns allocation.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                1,
                &mut pointer,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { pointer })
    }
    pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.pointer,
            bInheritHandle: 0,
        }
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.pointer);
        }
    }
}

pub fn create_directory(path: &Path) -> io::Result<()> {
    let descriptor = Descriptor::owner_only(true)?;
    let attributes = descriptor.attributes();
    let name = wide(path.as_os_str());
    // SAFETY: initialized attributes live throughout exclusive directory creation.
    if unsafe { CreateDirectoryW(name.as_ptr(), &attributes) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn create_file(path: &Path) -> io::Result<File> {
    let descriptor = Descriptor::owner_only(false)?;
    let attributes = descriptor.attributes();
    let name = wide(path.as_os_str());
    // SAFETY: exclusive creation; a returned valid handle is owned by File.
    unsafe {
        let handle = CreateFileW(
            name.as_ptr(),
            0x80000000 | 0x40000000,
            0,
            &attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(File::from_raw_handle(handle.cast()))
        }
    }
}

/// Verify the exact protected owner/SYSTEM ACL before opening any vault data.
pub fn verify_file_owner(path: &Path, directory: bool) -> io::Result<()> {
    let name = wide(path.as_os_str());
    let mut descriptor = ptr::null_mut();
    let information = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    // SAFETY: Windows allocates a valid descriptor, owned by Descriptor below.
    unsafe {
        let result = GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            information,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut descriptor,
        );
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
    }
    let descriptor = Descriptor {
        pointer: descriptor,
    };
    // SAFETY: the OS-owned descriptor stays live throughout inspection.
    unsafe { verify_descriptor(descriptor.pointer, directory, &current_sid()?) }
}

unsafe fn verify_descriptor(
    descriptor: *mut c_void,
    directory: bool,
    expected_sid: &str,
) -> io::Result<()> {
    let rejected = || io::Error::other("vault ownership or protected ACL rejected");
    // SAFETY: caller provides a live OS-validated security descriptor. Windows
    // validates its ACL and ACE lookup; SID length is checked against ACE bounds.
    unsafe {
        let mut control = 0;
        let mut revision = 0;
        let mut owner = ptr::null_mut();
        let mut defaulted = 0;
        let mut present = 0;
        let mut acl = ptr::null_mut();
        if GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) == 0
            || control & 0x1000 == 0
            || GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted) == 0
            || owner.is_null()
            || sid_string(owner)? != expected_sid
            || GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted) == 0
            || present == 0
            || acl.is_null()
            || IsValidAcl(acl) == 0
            || (*acl).AceCount != 2
        {
            return Err(rejected());
        }
        let flags = if directory { 3 } else { 0 };
        let mut trustees = Vec::new();
        for index in 0..2 {
            let mut raw = ptr::null_mut();
            if GetAce(acl, index, &mut raw) == 0 || raw.is_null() {
                return Err(rejected());
            }
            let header = &*(raw.cast::<ACE_HEADER>());
            // Header + access mask + the fixed eight-byte SID prefix.
            if header.AceType != 0 || header.AceFlags != flags || header.AceSize < 16 {
                return Err(rejected());
            }
            let ace = &*(raw.cast::<ACCESS_ALLOWED_ACE>());
            let sid_bytes = raw.cast::<u8>().add(8);
            let sid_length = 8 + 4 * usize::from(*sid_bytes.add(1));
            if ace.Mask != FILE_ALL_ACCESS || sid_length + 8 > usize::from(header.AceSize) {
                return Err(rejected());
            }
            let sid = ptr::addr_of!(ace.SidStart).cast_mut().cast();
            if IsValidSid(sid) == 0
                || GetLengthSid(sid) as usize + 8 > usize::from(ace.Header.AceSize)
            {
                return Err(rejected());
            }
            trustees.push(sid_string(sid)?);
        }
        trustees.sort();
        let mut expected = vec![expected_sid.to_owned(), "S-1-5-18".to_owned()];
        expected.sort();
        if trustees != expected {
            return Err(rejected());
        }
        Ok(())
    }
}

#[cfg(test)]
mod acl_tests {
    use super::*;
    #[test]
    fn structural_acl_checks_accept_aliases_and_order_but_reject_extra_access() {
        // Builtin Administrators has the SDDL alias BA. Matching its literal
        // rendered string to a numeric SID is not a security policy check.
        let sid = "S-1-5-32-544";
        for sddl in [
            "O:BAD:P(A;;FA;;;BA)(A;;FA;;;SY)",
            "O:BAD:P(A;;FA;;;S-1-5-18)(A;;FA;;;S-1-5-32-544)",
        ] {
            let descriptor = Descriptor::from_sddl(sddl).unwrap();
            assert!(unsafe { verify_descriptor(descriptor.pointer, false, sid) }.is_ok());
        }
        for sddl in [
            "O:BAD:P(A;;FA;;;BA)(A;;FA;;;SY)(A;;FR;;;WD)",
            "O:BAD:(A;;FA;;;BA)(A;;FA;;;SY)",
            "O:BAD:P(A;;FR;;;BA)(A;;FA;;;SY)",
            "O:BAD:P(A;OI;FA;;;BA)(A;;FA;;;SY)",
            "O:SYD:P(A;;FA;;;BA)(A;;FA;;;SY)",
            "O:BAD:P(A;;FA;;;BA)(A;;FA;;;BA)",
            "O:BAD:NO_ACCESS_CONTROL",
        ] {
            let descriptor = Descriptor::from_sddl(sddl).unwrap();
            assert!(unsafe { verify_descriptor(descriptor.pointer, false, sid) }.is_err());
        }
    }
}
