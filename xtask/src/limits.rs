use std::{io, mem, os::windows::ffi::OsStrExt, path::Path, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Storage::FileSystem::GetDiskFreeSpaceExW,
    System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
            JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
            JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation,
            QueryInformationJobObject, SetInformationJobObject,
        },
        Threading::{GetActiveProcessorCount, GetCurrentProcess},
    },
};

pub const MEMORY_BYTES: usize = 8 * 1024 * 1024 * 1024;
pub const MAX_DISK_BYTES: u64 = 20 * 1024 * 1024 * 1024;
pub const MIN_FREE_BYTES: u64 = 25 * 1024 * 1024 * 1024;

/// Enroll this dedicated runner before it creates children. Membership is then
/// inherited atomically by normal child creation, avoiding a spawn/assign race.
pub fn install_job_limits() -> io::Result<u32> {
    // SAFETY: all Windows structures are initialized, correctly sized, and live
    // across the calls. The job is unnamed and only this runner is enrolled.
    unsafe {
        let job = CreateJobObjectW(ptr::null(), ptr::null());
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let configure = || -> io::Result<u32> {
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | JOB_OBJECT_LIMIT_JOB_MEMORY
                | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            limits.BasicLimitInformation.ActiveProcessLimit = 32;
            limits.JobMemoryLimit = MEMORY_BYTES;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                mem::size_of_val(&limits) as u32,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            let processors = GetActiveProcessorCount(0xffff).max(1);
            let rate = (40_000 / processors).clamp(1, 10_000);
            let mut cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = mem::zeroed();
            cpu.ControlFlags =
                JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
            cpu.Anonymous.CpuRate = rate;
            if SetInformationJobObject(
                job,
                JobObjectCpuRateControlInformation,
                (&cpu as *const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast(),
                mem::size_of_val(&cpu) as u32,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            verify_limits(job)?;
            if AssignProcessToJobObject(job, GetCurrentProcess()) == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(rate)
        };
        match configure() {
            Ok(rate) => {
                // Intentionally retain the non-inheritable handle until process
                // exit. Closing it here would terminate the enrolled runner.
                // OS exit closes it and kills any remaining owned descendants.
                Ok(rate)
            }
            Err(error) => {
                CloseHandle(job);
                Err(error)
            }
        }
    }
}

unsafe fn verify_limits(job: HANDLE) -> io::Result<()> {
    // SAFETY: callers supply a live job handle; output is a correctly sized struct.
    unsafe {
        let mut observed: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
        if QueryInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&mut observed as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            mem::size_of_val(&observed) as u32,
            ptr::null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        if observed.JobMemoryLimit != MEMORY_BYTES
            || observed.BasicLimitInformation.ActiveProcessLimit != 32
        {
            return Err(io::Error::other("Windows job limits were not applied"));
        }
        Ok(())
    }
}

pub fn free_bytes(root: &Path) -> io::Result<u64> {
    let path: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0;
    // SAFETY: null-terminated path and valid output pointer, optional outputs null.
    if unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(available)
    }
}
