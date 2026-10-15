use crate::cache::is_regenerable_cache_path;
use crate::installer::under_named_folder;
use crate::sensitive::is_sensitive;
use std::path::Path;

const SYSTEM_PREFIXES: &[&str] = &[
    "/System",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var",
    "/Library",
    "/Applications",
    "/private",
    "/opt",
    "/Volumes",
];

pub fn is_protected_system(path: &Path) -> bool {
    if path == Path::new("/") {
        return true;
    }
    SYSTEM_PREFIXES.iter().any(|prefix| {
        let prefix = Path::new(prefix);
        path == prefix || path.starts_with(prefix)
    })
}

pub fn is_user_data_file(path: &Path, home: &Path) -> bool {
    ["Desktop", "Documents", "Downloads"]
        .into_iter()
        .any(|folder| under_named_folder(path, home, folder))
}

/// A file may be moved to the Trash only when it sits inside the user's home,
/// in a scanned folder or a regenerable cache.
pub fn deletion_allowed(path: &Path, home: &Path) -> bool {
    if home == Path::new("/") || path == home || is_sensitive(path).is_some() {
        return false;
    }
    if !path.starts_with(home) {
        return false;
    }
    is_regenerable_cache_path(path, home) || is_user_data_file(path, home)
}
