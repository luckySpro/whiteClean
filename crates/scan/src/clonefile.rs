use std::ffi::{c_char, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[cfg(target_os = "macos")]
#[repr(C)]
struct AttrList {
    bitmapcount: u16,
    reserved: u16,
    commonattr: u32,
    volattr: u32,
    dirattr: u32,
    fileattr: u32,
    forkattr: u32,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn getattrlist(
        path: *const c_char,
        attr_list: *mut AttrList,
        attr_buf: *mut u8,
        attr_buf_size: usize,
        options: u32,
    ) -> i32;
}

/// Shared APFS clone identity. `None` means the identity could not be read.
/// A refcount of 1 means this file is not sharing blocks with another file.
#[cfg(target_os = "macos")]
pub fn clone_identity(path: &Path) -> Option<(u64, u32)> {
    const ATTR_BIT_MAP_COUNT: u16 = 5;
    const ATTR_CMN_RETURNED_ATTRS: u32 = 0x8000_0000;
    const ATTR_CMNEXT_CLONEID: u32 = 0x0000_0100;
    const ATTR_CMNEXT_CLONE_REFCNT: u32 = 0x0000_1000;
    const FSOPT_ATTR_CMN_EXTENDED: u32 = 0x0000_0020;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut list = AttrList {
        bitmapcount: ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: ATTR_CMN_RETURNED_ATTRS,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: ATTR_CMNEXT_CLONEID | ATTR_CMNEXT_CLONE_REFCNT,
    };
    let mut buf = [0_u8; 128];
    let rc = unsafe {
        getattrlist(
            c_path.as_ptr(),
            &mut list,
            buf.as_mut_ptr(),
            buf.len(),
            FSOPT_ATTR_CMN_EXTENDED,
        )
    };
    if rc != 0 {
        return None;
    }
    // length(4) + attribute_set(20) + clone id(8) + refcount(4), measured on APFS.
    let length = u32::from_le_bytes(buf[0..4].try_into().ok()?);
    if length < 36 {
        return None;
    }
    let id = u64::from_le_bytes(buf[24..32].try_into().ok()?);
    let refs = u32::from_le_bytes(buf[32..36].try_into().ok()?);
    Some((id, refs))
}

#[cfg(not(target_os = "macos"))]
pub fn clone_identity(_path: &Path) -> Option<(u64, u32)> {
    None
}
