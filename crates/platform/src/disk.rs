use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub free_bytes: u64,
}

pub fn disk_usage(path: &Path) -> Option<DiskUsage> {
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = unsafe { std::mem::zeroed::<libc::statfs>() };
    if unsafe { libc::statfs(c_path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    let block = u64::try_from(stat.f_bsize).ok()?;
    Some(DiskUsage {
        total_bytes: stat.f_blocks.saturating_mul(block),
        free_bytes: stat.f_bavail.saturating_mul(block),
    })
}
