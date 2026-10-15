use platform::{apply_cleanup, undo_last, PlannedDelete};
use std::fs;
use std::time::UNIX_EPOCH;

#[test]
fn trash_and_undo_round_trip_and_missing_trash_is_reported() {
    let root = std::env::temp_dir().join(format!("whiteclean-trash-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let home = root.join("home");
    let downloads = home.join("Downloads");
    fs::create_dir_all(&downloads).unwrap();
    let file = downloads.join("note.txt");
    fs::write(&file, b"hello").unwrap();
    let modified_ms = fs::metadata(&file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let trash = root.join("trash");
    let journal = root.join("journal.json");
    let plan = [PlannedDelete {
        path: file.clone(),
        size: 5,
        modified_ms,
        is_dir: false,
    }];
    let cleaned = apply_cleanup(&home, &plan, &trash, &journal);
    assert_eq!(cleaned.trashed, 1);
    assert!(!file.exists());
    assert!(trash.join("note.txt").exists());

    let undone = undo_last(&home, &trash, &journal);
    assert_eq!(undone.restored, 1);
    assert_eq!(fs::read(&file).unwrap(), b"hello");

    fs::write(&file, b"hello").unwrap();
    let again = apply_cleanup(
        &home,
        &[PlannedDelete {
            path: file.clone(),
            size: 5,
            modified_ms: fs::metadata(&file)
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            is_dir: false,
        }],
        &trash,
        &journal,
    );
    assert_eq!(again.trashed, 1);
    fs::remove_dir_all(&trash).unwrap();
    let missing = undo_last(&home, &trash, &journal);
    assert_eq!(missing.missing, 1);
    assert_eq!(missing.restored, 0);
    let _ = fs::remove_dir_all(&root);
}

#[cfg(target_os = "macos")]
#[test]
fn real_trash_round_trip() {
    let user_home = dirs::home_dir().expect("home");
    let root = std::env::temp_dir().join(format!("whiteclean-real-{}", std::process::id()));
    let home = root.join("user");
    let downloads = home.join("Downloads");
    fs::create_dir_all(&downloads).unwrap();
    let file = downloads.join("whiteclean-undo-check.txt");
    fs::write(&file, b"put me back").unwrap();
    let modified_ms = fs::metadata(&file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let trash = user_home.join(".Trash");
    let journal = root.join("journal.json");
    let cleaned = apply_cleanup(
        &home,
        &[PlannedDelete {
            path: file.clone(),
            size: b"put me back".len() as u64,
            modified_ms,
            is_dir: false,
        }],
        &trash,
        &journal,
    );
    assert_eq!(cleaned.trashed, 1, "temp file should move into the real Trash");
    let undone = undo_last(&home, &trash, &journal);
    assert_eq!(undone.restored, 1);
    assert_eq!(fs::read(&file).unwrap(), b"put me back");
    let _ = fs::remove_dir_all(&root);
}
