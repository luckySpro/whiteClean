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

const PROTECTED_HOME_FOLDERS: &[&str] = &[
    "Desktop",
    "Documents",
    "Downloads",
    "Library",
    "Applications",
    "Movies",
    "Music",
    "Pictures",
    "Public",
];

/// A file may be moved to the Trash only when it sits inside the user's home,
/// in a user folder or a regenerable cache, and is not sensitive.
pub fn deletion_allowed(path: &Path, home: &Path) -> bool {
    if home == Path::new("/") || path == home || is_sensitive(path).is_some() {
        return false;
    }
    if !path.starts_with(home) {
        return false;
    }
    for folder in PROTECTED_HOME_FOLDERS {
        if path == home.join(folder) {
            return false;
        }
    }
    let lib = home.join("Library");
    if path.starts_with(&lib) {
        return is_regenerable_cache_path(path, home);
    }
    true
}
