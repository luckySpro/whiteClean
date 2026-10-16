//! Desktop commands. The scan policy stays in `scan` and `policy`.

use platform::{
    apply_cleanup, disk_usage, journal_path, open_permission_settings as open_settings,
    permission_missing, trash_dir, undo_last, CleanOutcome, DiskUsage, PlannedDelete, UndoOutcome,
};
use scan::{scan as scan_home, ScanReport, ScanRequest};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    home: PathBuf,
    last: Mutex<Option<ScanReport>>,
}

#[derive(Debug, Clone, Serialize)]
struct ScanPayload {
    freeable_bytes: u64,
    delete_count: u32,
    skipped_sensitive_count: u32,
    groups: Vec<scan::ReportGroup>,
    skipped: Vec<scan::SkippedItem>,
    disk: DiskUsage,
    permission_missing: bool,
}

fn resolve_home() -> PathBuf {
    std::env::var("WHITECLEAN_FIXTURE")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn payload(home: &Path, report: ScanReport) -> ScanPayload {
    let disk = disk_usage(home).unwrap_or(DiskUsage {
        total_bytes: 0,
        free_bytes: 0,
    });
    let missing = permission_missing(home);
    ScanPayload {
        freeable_bytes: report.freeable_bytes,
        delete_count: report.delete_count,
        skipped_sensitive_count: report.skipped_sensitive_count,
        groups: report.groups,
        skipped: report.skipped,
        disk,
        permission_missing: missing,
    }
}

pub fn planned_deletes(report: &ScanReport, paths: &[String]) -> Vec<PlannedDelete> {
    let wanted: HashSet<&str> = paths.iter().map(String::as_str).collect();
    report
        .selected_items()
        .into_iter()
        .filter(|item| wanted.contains(item.path.as_str()))
        .map(|item| PlannedDelete {
            path: PathBuf::from(item.path),
            size: item.size,
            modified_ms: item.modified_ms,
            is_dir: item.is_dir,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub kind: String, // "default" | "cache" | "custom"
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPayload {
    pub home: String,
    pub folders: Vec<FolderItem>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScanOptions {
    pub folders: Option<Vec<String>>,
    pub include_caches: Option<bool>,
}

#[tauri::command]
fn get_default_folders(state: State<'_, AppState>) -> ConfigPayload {
    let home = &state.home;
    let mut folders = Vec::new();
    folders.push(FolderItem {
        id: "downloads".into(),
        name: "下载".into(),
        path: home.join("Downloads").to_string_lossy().into_owned(),
        kind: "default".into(),
        enabled: true,
    });
    folders.push(FolderItem {
        id: "desktop".into(),
        name: "桌面".into(),
        path: home.join("Desktop").to_string_lossy().into_owned(),
        kind: "default".into(),
        enabled: true,
    });
    folders.push(FolderItem {
        id: "documents".into(),
        name: "文稿".into(),
        path: home.join("Documents").to_string_lossy().into_owned(),
        kind: "default".into(),
        enabled: true,
    });
    folders.push(FolderItem {
        id: "caches".into(),
        name: "系统缓存与日志".into(),
        path: home.join("Library/Caches").to_string_lossy().into_owned(),
        kind: "cache".into(),
        enabled: true,
    });
    ConfigPayload {
        home: home.to_string_lossy().into_owned(),
        folders,
    }
}

#[tauri::command]
fn pick_folder() -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"try
POSIX path of (choose folder with prompt "选择要检查的文件夹")
on error
return ""
end try"#;
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| e.to_string())?;
        let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if res.is_empty() {
            Ok(None)
        } else {
            let clean = res.trim_end_matches('/').to_string();
            Ok(Some(clean))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(None)
    }
}

#[tauri::command]
async fn scan(options: Option<ScanOptions>, state: State<'_, AppState>) -> Result<ScanPayload, String> {
    let home = state.home.clone();
    let request = if let Some(opts) = options {
        let roots: Vec<PathBuf> = opts
            .folders
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .filter(|p| p.exists() && p.is_dir())
            .collect();
        let inc_caches = opts.include_caches.unwrap_or(true);
        if roots.is_empty() {
            ScanRequest::for_home(home.clone())
        } else {
            ScanRequest::with_folders(home.clone(), roots, inc_caches)
        }
    } else {
        ScanRequest::for_home(home.clone())
    };

    let report = tauri::async_runtime::spawn_blocking(move || scan_home(&request))
        .await
        .map_err(|error| error.to_string())?;
    let view = payload(&state.home, report.clone());
    *state.last.lock().unwrap_or_else(|error| error.into_inner()) = Some(report);
    Ok(view)
}

#[tauri::command]
fn clean(paths: Vec<String>, state: State<'_, AppState>) -> Result<CleanOutcome, String> {
    let report = state
        .last
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .ok_or_else(|| "还没有检查结果".to_string())?;
    let plan = planned_deletes(&report, &paths);
    Ok(apply_cleanup(
        &state.home,
        &plan,
        &trash_dir(),
        &journal_path(),
    ))
}

#[tauri::command]
fn undo(state: State<'_, AppState>) -> Result<UndoOutcome, String> {
    Ok(undo_last(&state.home, &trash_dir(), &journal_path()))
}

#[tauri::command]
fn open_permission_settings() -> bool {
    open_settings()
}

pub fn scanned_path(report: &ScanReport, path: &str) -> bool {
    report.groups.iter().any(|group| {
        group.items.iter().any(|item| item.path == path)
            || group.keep_path.as_deref() == Some(path)
    }) || report.skipped.iter().any(|item| item.path == path)
}

#[tauri::command]
fn reveal_in_finder(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let known = state
        .last
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .as_ref()
        .is_some_and(|report| scanned_path(report, &path));
    if !known {
        return Err("这个文件不在这次检查里".to_string());
    }
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err("文件已经不在了".to_string());
    }
    Command::new("/usr/bin/open")
        .arg("-R")
        .arg(&target)
        .spawn()
        .map(|_| ())
        .map_err(|_| "打不开访达".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            home: resolve_home(),
            last: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            scan,
            clean,
            undo,
            open_permission_settings,
            reveal_in_finder,
            get_default_folders,
            pick_folder
        ])
        .run(tauri::generate_context!())
        .expect("WhiteClean failed to start");
}

#[cfg(test)]
mod tests {
    use super::{planned_deletes, scanned_path};
    use scan::{AdviceKind, ReportGroup, ScanReport, TrashItem};

    #[test]
    fn only_paths_from_the_last_scan_can_be_cleaned() {
        let report = ScanReport {
            freeable_bytes: 4,
            delete_count: 1,
            skipped_sensitive_count: 0,
            groups: vec![ReportGroup {
                kind: AdviceKind::OldInstaller,
                keep_name: None,
                keep_path: None,
                note: None,
                delete_count: 1,
                freeable_bytes: 4,
                samples: vec!["Setup.dmg".into()],
                items: vec![TrashItem {
                    path: "/tmp/home/Downloads/Setup.dmg".into(),
                    name: "Setup.dmg".into(),
                    label: None,
                    size: 4,
                    modified_ms: 1,
                    is_dir: false,
                }],
            }],
            skipped: vec![],
        };
        let plan = planned_deletes(
            &report,
            &[
                "/tmp/home/Downloads/Setup.dmg".into(),
                "/tmp/home/Documents/合同.pdf".into(),
            ],
        );
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].path.as_os_str(), "/tmp/home/Downloads/Setup.dmg");
    }

    #[test]
    fn reveal_only_accepts_a_path_from_the_scan() {
        let report = ScanReport {
            freeable_bytes: 4,
            delete_count: 1,
            skipped_sensitive_count: 1,
            groups: vec![ReportGroup {
                kind: AdviceKind::OldInstaller,
                keep_name: None,
                keep_path: Some("/tmp/home/Downloads/Kept.dmg".into()),
                note: None,
                delete_count: 1,
                freeable_bytes: 4,
                samples: vec!["Setup.dmg".into()],
                items: vec![TrashItem {
                    path: "/tmp/home/Downloads/Setup.dmg".into(),
                    name: "Setup.dmg".into(),
                    label: None,
                    size: 4,
                    modified_ms: 1,
                    is_dir: false,
                }],
            }],
            skipped: vec![scan::SkippedItem {
                name: "合同.pdf".into(),
                path: "/tmp/home/Downloads/合同.pdf".into(),
                note: scan::NoteCode::Sensitive,
            }],
        };
        assert!(scanned_path(&report, "/tmp/home/Downloads/Setup.dmg"));
        assert!(scanned_path(&report, "/tmp/home/Downloads/Kept.dmg"));
        assert!(scanned_path(&report, "/tmp/home/Downloads/合同.pdf"));
        assert!(!scanned_path(&report, "/tmp/home/Documents/secret.pdf"));
    }
}
