//! Pure cleanup decisions. No disk access.

mod cache;
mod installer;
mod keeper;
mod protect;
mod sensitive;
mod version;

pub use cache::{cache_component_denied, is_browser_profile_cache, is_regenerable_cache_path};
pub use installer::{is_old_installer, INSTALLER_MAX_AGE_MS};
pub use keeper::{keeper_index, location_rank, RankedFile};
pub use protect::{deletion_allowed, is_protected_system, is_user_data_file};
pub use sensitive::{is_sensitive, SensitiveHit};
pub use version::version_stem;

/// Duplicate groups smaller than this stay off the home suggestion.
pub const MIN_DUPLICATE_FILE_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceKind {
    ExactDuplicate,
    OlderVersion,
    Regenerable,
    OldInstaller,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteCode {
    ContentDiffers,
    Sensitive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn keeps_the_newer_file() {
        let downloads = Path::new("/Users/a/Downloads/Setup.dmg");
        let desktop = Path::new("/Users/a/Desktop/Setup.dmg");
        let files = [
            RankedFile { path: downloads, modified_ms: 10 },
            RankedFile { path: desktop, modified_ms: 50 },
        ];
        assert_eq!(keeper_index(&files), 1);
    }

    #[test]
    fn same_time_prefers_documents_over_downloads() {
        let downloads = Path::new("/Users/a/Downloads/Photo.bin");
        let documents = Path::new("/Users/a/Documents/Photo.bin");
        let files = [
            RankedFile { path: downloads, modified_ms: 20 },
            RankedFile { path: documents, modified_ms: 20 },
        ];
        assert_eq!(keeper_index(&files), 1);
        assert!(location_rank(documents) > location_rank(downloads));
    }

    #[test]
    fn version_stems_collapse_copy_markers() {
        assert_eq!(version_stem("报告.pdf"), version_stem("报告 副本.pdf"));
        assert_eq!(version_stem("报告.pdf"), version_stem("报告 (1).pdf"));
        assert_eq!(version_stem("报告.pdf"), version_stem("报告-v2.pdf"));
        assert_eq!(version_stem("Report.pdf"), version_stem("Report copy.pdf"));
        assert_ne!(version_stem("简历.pdf"), version_stem("合同.pdf"));
    }

    #[test]
    fn contract_name_is_sensitive_and_cache_bin_is_not() {
        assert!(is_sensitive(Path::new("/Users/a/Downloads/合同.pdf")).is_some());
        assert!(is_sensitive(Path::new("/Users/a/.ssh/id_rsa")).is_some());
        assert!(is_sensitive(Path::new("/Users/a/Photos/Library.photoslibrary/x")).is_some());
        assert!(is_sensitive(Path::new("/Users/a/Library/Caches/com.example/cache.bin")).is_none());
    }

    #[test]
    fn regenerable_cache_allows_logs_and_rejects_chrome_cookies() {
        let home = Path::new("/Users/a");
        assert!(is_regenerable_cache_path(
            Path::new("/Users/a/Library/Logs/app.log"),
            home
        ));
        assert!(is_regenerable_cache_path(
            Path::new("/Users/a/Library/Caches/com.apple.Safari/Cache.db"),
            home
        ));
        assert!(is_regenerable_cache_path(
            Path::new("/Users/a/Library/Application Support/Google/Chrome/Default/Cache/a.bin"),
            home
        ));
        assert!(!is_regenerable_cache_path(
            Path::new("/Users/a/Library/Application Support/Google/Chrome/Default/Cookies"),
            home
        ));
        assert!(!is_regenerable_cache_path(
            Path::new("/Users/a/Library/Caches/huggingface/model.bin"),
            home
        ));
        assert!(!is_regenerable_cache_path(Path::new("/Users/a/Documents/笔记.txt"), home));
    }

    #[test]
    fn old_installer_must_live_in_downloads_or_desktop_and_be_old() {
        let home = Path::new("/Users/a");
        let now = 40 * 24 * 60 * 60 * 1000;
        let old = now - INSTALLER_MAX_AGE_MS - 1;
        let recent = now - 1000;
        assert!(is_old_installer(
            Path::new("/Users/a/Downloads/Setup.dmg"),
            home,
            old,
            now
        ));
        assert!(!is_old_installer(
            Path::new("/Users/a/Downloads/Setup.dmg"),
            home,
            recent,
            now
        ));
        assert!(!is_old_installer(
            Path::new("/Users/a/Documents/Setup.dmg"),
            home,
            old,
            now
        ));
        assert!(!is_old_installer(
            Path::new("/Users/a/Downloads/合同.dmg"),
            home,
            old,
            now
        ));
        assert!(!is_old_installer(
            Path::new("/Users/a/Downloads/notes.txt"),
            home,
            old,
            now
        ));
    }

    #[test]
    fn system_paths_and_home_roots_cannot_be_trashed() {
        let home = Path::new("/Users/a");
        assert!(is_protected_system(Path::new("/System/Library/Kernels/kernel")));
        assert!(!deletion_allowed(Path::new("/System/Library/Caches/x"), home));
        assert!(!deletion_allowed(home, home));
        assert!(!deletion_allowed(Path::new("/Users/a/Downloads"), home));
        assert!(!deletion_allowed(Path::new("/Users/a/Documents"), home));
        assert!(deletion_allowed(Path::new("/Users/a/Downloads/Setup.dmg"), home));
        assert!(deletion_allowed(Path::new("/Users/a/Library/Logs/app.log"), home));
        assert!(!deletion_allowed(Path::new("/Users/a/Downloads/合同.pdf"), home));
        assert!(!deletion_allowed(Path::new("/Volumes/Backup/Setup.dmg"), home));
    }
}
