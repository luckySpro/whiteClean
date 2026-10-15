//! Trash, an operation journal, and the permission prompt. No scan policy lives here.

mod disk;
mod journal;
mod permission;
mod trash;

use journal::{Journal, JournalItem, Operation, TruncateFront};
use policy::deletion_allowed;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub use disk::{disk_usage, DiskUsage};
pub use permission::{open_permission_settings, permission_missing};

#[derive(Debug, Clone)]
pub struct PlannedDelete {
    pub path: PathBuf,
    pub size: u64,
    pub modified_ms: i64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CleanOutcome {
    pub trashed: u32,
    pub bytes: u64,
    pub failed: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UndoOutcome {
    pub had_operation: bool,
    pub restored: u32,
    pub missing: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUndo {
    pub bytes: u64,
    pub files: u32,
}

pub fn trash_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("WHITECLEAN_TRASH_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".Trash")
}

pub fn app_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("WHITECLEAN_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("WhiteClean")
}

pub fn journal_path() -> PathBuf {
    app_data_dir().join("journal.json")
}

pub fn pending_undo(journal_path: &Path) -> Option<PendingUndo> {
    let journal = Journal::load(journal_path);
    let operation = journal.operations.iter().rev().find(|op| !op.undone)?;
    let files = operation
        .items
        .iter()
        .filter(|item| !item.restored && !item.missing)
        .count() as u32;
    if files == 0 {
        return None;
    }
    let bytes = operation
        .items
        .iter()
        .filter(|item| !item.restored && !item.missing)
        .map(|item| item.size)
        .sum();
    Some(PendingUndo { bytes, files })
}

pub fn apply_cleanup(
    home: &Path,
    plan: &[PlannedDelete],
    trash_dir: &Path,
    journal_path: &Path,
) -> CleanOutcome {
    let mut outcome = CleanOutcome {
        trashed: 0,
        bytes: 0,
        failed: 0,
        paths: Vec::new(),
    };
    let mut moved = Vec::new();
    for item in plan {
        match trash_one(home, item, trash_dir) {
            Ok(trash_path) => {
                outcome.trashed += 1;
                outcome.bytes += item.size;
                outcome.paths.push(item.path.to_string_lossy().into_owned());
                moved.push(JournalItem {
                    original_path: item.path.to_string_lossy().into_owned(),
                    trash_path: trash_path.to_string_lossy().into_owned(),
                    size: item.size,
                    modified_ms: item.modified_ms,
                    restored: false,
                    missing: false,
                });
            }
            Err(_) => outcome.failed += 1,
        }
    }
    if !moved.is_empty() {
        let mut journal = Journal::load(journal_path);
        journal.operations.push(Operation {
            created_ms: now_ms(),
            undone: false,
            items: moved,
        });
        journal.operations.truncate_front(20);
        if journal.save(journal_path).is_err() {
            // The files are already in the Trash. Keep the in-memory record attempt visible as success.
        }
    }
    outcome
}

pub fn undo_last(home: &Path, trash_dir: &Path, journal_path: &Path) -> UndoOutcome {
    let mut journal = Journal::load(journal_path);
    let Some(operation) = journal.operations.iter_mut().rev().find(|op| !op.undone) else {
        return UndoOutcome {
            had_operation: false,
            restored: 0,
            missing: 0,
            failed: 0,
        };
    };
    let mut outcome = UndoOutcome {
        had_operation: true,
        restored: 0,
        missing: 0,
        failed: 0,
    };
    for item in &mut operation.items {
        if item.restored || item.missing {
            continue;
        }
        match trash::restore(Path::new(&item.trash_path), Path::new(&item.original_path), trash_dir, home)
        {
            Ok(_) => {
                item.restored = true;
                outcome.restored += 1;
            }
            Err(trash::TrashError::Missing) => {
                item.missing = true;
                outcome.missing += 1;
            }
            Err(_) => outcome.failed += 1,
        }
    }
    operation.undone = operation.items.iter().all(|item| item.restored || item.missing);
    let _ = journal.save(journal_path);
    outcome
}

fn trash_one(home: &Path, item: &PlannedDelete, trash_dir: &Path) -> Result<PathBuf, ()> {
    if !item.path.is_absolute() || !deletion_allowed(&item.path, home) {
        return Err(());
    }
    let meta = fs::symlink_metadata(&item.path).map_err(|_| ())?;
    if meta.file_type().is_symlink() {
        return Err(());
    }
    if item.is_dir {
        if !meta.is_dir() {
            return Err(());
        }
    } else if !meta.is_file() || meta.len() != item.size {
        return Err(());
    }
    if modified_ms(&meta) != item.modified_ms {
        return Err(());
    }
    trash::move_to_trash(&item.path, trash_dir).map_err(|_| ())
}

fn modified_ms(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
