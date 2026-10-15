use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum TrashError {
    Missing,
    Refused,
    Io,
}

pub fn move_to_trash(src: &Path, trash_dir: &Path) -> Result<PathBuf, TrashError> {
    let name = src.file_name().ok_or(TrashError::Refused)?;
    if name.to_string_lossy().contains('/') || name.to_string_lossy().contains('\\') {
        return Err(TrashError::Refused);
    }
    fs::create_dir_all(trash_dir).map_err(|_| TrashError::Io)?;
    let dest = unique_path(trash_dir, name);
    fs::rename(src, &dest).map_err(|error| {
        if error.raw_os_error() == Some(18) {
            TrashError::Refused
        } else {
            TrashError::Io
        }
    })?;
    Ok(dest)
}

pub fn restore(
    trash_path: &Path,
    original: &Path,
    trash_dir: &Path,
    home: &Path,
) -> Result<PathBuf, TrashError> {
    if !trash_path.starts_with(trash_dir) || !original.starts_with(home) || original == home {
        return Err(TrashError::Refused);
    }
    if !trash_path.exists() {
        return Err(TrashError::Missing);
    }
    let dest = if original.exists() {
        restored_name(original)
    } else {
        original.to_path_buf()
    };
    if !dest.starts_with(home) {
        return Err(TrashError::Refused);
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|_| TrashError::Io)?;
    }
    fs::rename(trash_path, &dest).map_err(|_| TrashError::Io)?;
    Ok(dest)
}

fn unique_path(trash_dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let direct = trash_dir.join(name);
    if !direct.exists() {
        return direct;
    }
    let parsed = Path::new(name);
    let stem = parsed
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let ext = parsed
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned());
    for index in 2..10_000 {
        let candidate = match &ext {
            Some(ext) => trash_dir.join(format!("{stem} {index}.{ext}")),
            None => trash_dir.join(format!("{stem} {index}")),
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    trash_dir.join(format!("{stem} {}", now_suffix()))
}

fn restored_name(original: &Path) -> PathBuf {
    let parent = original.parent().unwrap_or_else(|| Path::new("."));
    let stem = original
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let ext = original
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned());
    for index in 0..1_000 {
        let marked = if index == 0 {
            format!("{stem}（已恢复）")
        } else {
            format!("{stem}（已恢复） {index}")
        };
        let candidate = match &ext {
            Some(ext) => parent.join(format!("{marked}.{ext}")),
            None => parent.join(marked),
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}（已恢复） {}", now_suffix()))
}

fn now_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
