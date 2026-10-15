use crate::sensitive::is_sensitive;
use std::path::Path;

pub const INSTALLER_MAX_AGE_MS: i64 = 30 * 24 * 60 * 60 * 1000;

const EXTENSIONS: &[&str] = &["dmg", "pkg", "mpkg", "iso", "xip"];

pub fn is_old_installer(path: &Path, home: &Path, modified_ms: i64, now_ms: i64) -> bool {
    if is_sensitive(path).is_some() {
        return false;
    }
    if now_ms.saturating_sub(modified_ms) < INSTALLER_MAX_AGE_MS {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    if !EXTENSIONS.iter().any(|item| ext.eq_ignore_ascii_case(item)) {
        return false;
    }
    under_named_folder(path, home, "Downloads") || under_named_folder(path, home, "Desktop")
}

pub(crate) fn under_named_folder(path: &Path, home: &Path, folder: &str) -> bool {
    let root = home.join(folder);
    path.starts_with(&root) && path != root
}
