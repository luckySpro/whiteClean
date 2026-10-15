use policy::cache_component_denied;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileFact {
    pub path: PathBuf,
    pub size: u64,
    pub modified_ms: i64,
    pub dev: u64,
    pub ino: u64,
}

pub fn collect_files(roots: &[PathBuf], user_folders: bool) -> Vec<FileFact> {
    let mut files = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let walker = WalkDir::new(root).follow_links(false).into_iter().filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy();
                return !should_skip_dir(&name, user_folders);
            }
            true
        });
        for entry in walker {
            let Ok(entry) = entry else {
                continue;
            };
            if entry.file_type().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            if user_folders && name.starts_with('.') {
                continue;
            }
            if !user_folders && cache_component_denied(&name) {
                continue;
            }
            let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.file_type().is_symlink() || !meta.is_file() {
                continue;
            }
            if is_dataless(&meta) {
                continue;
            }
            let path = entry.path().to_path_buf();
            if path_has_skipped_dir(&path, user_folders) {
                continue;
            }
            files.push(FileFact {
                path,
                size: meta.len(),
                modified_ms: modified_ms(&meta),
                dev: std::os::unix::fs::MetadataExt::dev(&meta),
                ino: std::os::unix::fs::MetadataExt::ino(&meta),
            });
        }
    }
    files
}

fn path_has_skipped_dir(path: &Path, user_folders: bool) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        should_skip_dir(&name, user_folders)
    })
}

fn should_skip_dir(name: &str, user_folders: bool) -> bool {
    if !user_folders {
        return cache_component_denied(name);
    }
    let lower = name.to_lowercase();
    lower.starts_with('.')
        || lower == "node_modules"
        || lower == "mail"
        || lower == "messages"
        || lower.ends_with(".photoslibrary")
        || lower.ends_with(".app")
}

fn modified_ms(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time: SystemTime| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(target_os = "macos")]
fn is_dataless(meta: &Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    const SF_DATALESS: u32 = 0x4000_0000;
    meta.st_flags() & SF_DATALESS != 0
}

#[cfg(not(target_os = "macos"))]
fn is_dataless(_meta: &Metadata) -> bool {
    false
}
