use filetime::{set_file_mtime, FileTime};
use platform::{apply_cleanup, undo_last, PlannedDelete};
use scan::{scan, AdviceKind, ScanRequest};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn write_aged(path: &Path, bytes: &[u8], modified_ms: i64) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
    let seconds = modified_ms / 1000;
    set_file_mtime(path, FileTime::from_unix_time(seconds, 0)).unwrap();
}

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "whiteclean-scan-{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Downloads")).unwrap();
    fs::create_dir_all(root.join("Desktop")).unwrap();
    fs::create_dir_all(root.join("Documents")).unwrap();
    fs::create_dir_all(root.join("Library/Caches/com.example")).unwrap();
    fs::create_dir_all(root.join("Library/Logs")).unwrap();
    root
}

fn request(home: &Path) -> ScanRequest {
    ScanRequest {
        duplicate_roots: vec![home.join("Downloads"), home.join("Desktop"), home.join("Documents")],
        installer_roots: vec![home.join("Downloads"), home.join("Desktop")],
        cache_roots: vec![home.join("Library/Caches"), home.join("Library/Logs")],
        now_ms: now_ms(),
        min_duplicate_file_bytes: 1,
        home: home.to_path_buf(),
    }
}

#[test]
fn suggests_old_copy_installer_and_cache_but_keeps_the_contract() {
    let home = fixture("keep-contract");
    let now = now_ms();
    let old = now - 40 * 24 * 60 * 60 * 1000;
    let body = b"same-installer-bytes";
    write_aged(&home.join("Downloads/Setup.dmg"), body, old);
    write_aged(&home.join("Downloads/Setup (1).dmg"), body, now);
    write_aged(&home.join("Documents/报告.pdf"), b"new-report", now);
    write_aged(&home.join("Documents/报告 副本.pdf"), b"old-report", old);
    write_aged(&home.join("Downloads/合同.pdf"), b"id-card", now);
    write_aged(&home.join("Downloads/合同 副本.pdf"), b"id-card-old", old);
    write_aged(
        &home.join("Library/Caches/com.example/cache.bin"),
        b"cache",
        now,
    );
    write_aged(&home.join("Library/Logs/app.log"), b"log", now);

    let report = scan(&request(&home));
    let selected: Vec<_> = report
        .groups
        .iter()
        .flat_map(|group| group.items.iter().map(|item| item.path.clone()))
        .collect();

    assert!(selected.iter().any(|path| path.ends_with("Setup.dmg")));
    assert!(!selected.iter().any(|path| path.ends_with("Setup (1).dmg")));
    assert!(selected.iter().any(|path| path.ends_with("报告 副本.pdf")));
    assert!(!selected.iter().any(|path| path.ends_with("/报告.pdf")));
    assert!(selected.iter().any(|path| path.ends_with("com.example")));
    assert!(selected.iter().any(|path| path.ends_with("app.log")));
    assert!(!selected.iter().any(|path| path.contains("合同")));
    assert!(report.skipped.iter().any(|item| item.name.contains("合同")));

    let version = report
        .groups
        .iter()
        .find(|group| group.kind == AdviceKind::OlderVersion)
        .expect("version group");
    assert_eq!(version.note, Some(scan::NoteCode::ContentDiffers));

    let trash = home.join("trash");
    let journal = home.join("journal.json");
    let plan: Vec<_> = report
        .selected_items()
        .into_iter()
        .map(|item| PlannedDelete {
            path: PathBuf::from(item.path),
            size: item.size,
            modified_ms: item.modified_ms,
            is_dir: item.is_dir,
        })
        .collect();
    let cleaned = apply_cleanup(&home, &plan, &trash, &journal);
    assert_eq!(cleaned.failed, 0);
    assert!(cleaned.trashed >= 4);
    assert!(!home.join("Downloads/Setup.dmg").exists());
    assert!(home.join("Downloads/合同.pdf").exists());

    let undone = undo_last(&home, &trash, &journal);
    assert!(undone.restored >= 4);
    assert!(home.join("Downloads/Setup.dmg").exists());
    assert!(home.join("Library/Caches/com.example/cache.bin").exists());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn tiny_exact_copies_stay_off_the_suggestion() {
    let home = fixture("tiny");
    let mut options = request(&home);
    options.min_duplicate_file_bytes = 5 * 1024 * 1024;
    let now = now_ms();
    write_aged(&home.join("Downloads/a.bin"), b"tiny", now);
    write_aged(&home.join("Desktop/a.bin"), b"tiny", now - 1000);
    let report = scan(&options);
    assert!(report
        .groups
        .iter()
        .all(|group| group.kind != AdviceKind::ExactDuplicate));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn same_name_in_different_folders_is_not_a_version() {
    let home = fixture("same-name");
    let now = now_ms();
    write_aged(&home.join("Downloads/简历.pdf"), b"one", now);
    write_aged(&home.join("Documents/简历.pdf"), b"two", now - 1000);
    let report = scan(&request(&home));
    assert!(report.groups.is_empty());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn hardlinks_are_not_duplicates() {
    let home = fixture("hardlink");
    let now = now_ms();
    let original = home.join("Downloads/movie.bin");
    write_aged(&original, b"same-inode", now);
    fs::hard_link(&original, home.join("Desktop/movie.bin")).unwrap();
    let report = scan(&request(&home));
    assert!(report
        .groups
        .iter()
        .all(|group| group.kind != AdviceKind::ExactDuplicate));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn equal_time_keeps_the_documents_copy() {
    let home = fixture("equal-time");
    let now = now_ms();
    write_aged(&home.join("Downloads/Photo.bin"), b"photo", now);
    write_aged(&home.join("Documents/Photo.bin"), b"photo", now);
    let report = scan(&request(&home));
    let group = report
        .groups
        .iter()
        .find(|group| group.kind == AdviceKind::ExactDuplicate)
        .expect("duplicate");
    assert!(group.keep_path.as_deref().unwrap().contains("/Documents/"));
    assert!(group.items.iter().all(|item| item.path.contains("/Downloads/")));
    let _ = fs::remove_dir_all(&home);
}
