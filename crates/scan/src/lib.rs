//! Scan user folders and turn them into a short list of suggested deletions.

mod clonefile;
mod hash;
mod walk;

use policy::{
    deletion_allowed, is_old_installer, is_regenerable_cache_path, is_sensitive, keeper_index,
    version_stem, RankedFile, MIN_DUPLICATE_FILE_BYTES,
};
pub use policy::{AdviceKind, NoteCode};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walk::FileFact;

#[derive(Debug, Clone)]
pub struct ScanRequest {
    pub home: PathBuf,
    pub duplicate_roots: Vec<PathBuf>,
    pub installer_roots: Vec<PathBuf>,
    pub cache_roots: Vec<PathBuf>,
    pub now_ms: i64,
    pub min_duplicate_file_bytes: u64,
}

impl ScanRequest {
    pub fn for_home(home: PathBuf) -> Self {
        Self {
            duplicate_roots: vec![
                home.join("Downloads"),
                home.join("Desktop"),
                home.join("Documents"),
            ],
            installer_roots: vec![home.join("Downloads"), home.join("Desktop")],
            cache_roots: vec![home.join("Library/Caches"), home.join("Library/Logs")],
            now_ms: now_ms(),
            min_duplicate_file_bytes: MIN_DUPLICATE_FILE_BYTES,
            home,
        }
    }

    pub fn with_folders(home: PathBuf, scan_roots: Vec<PathBuf>, include_caches: bool) -> Self {
        let cache_roots = if include_caches {
            vec![home.join("Library/Caches"), home.join("Library/Logs")]
        } else {
            Vec::new()
        };
        Self {
            installer_roots: scan_roots.clone(),
            duplicate_roots: scan_roots,
            cache_roots,
            now_ms: now_ms(),
            min_duplicate_file_bytes: MIN_DUPLICATE_FILE_BYTES,
            home,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    pub freeable_bytes: u64,
    pub delete_count: u32,
    pub skipped_sensitive_count: u32,
    pub groups: Vec<ReportGroup>,
    pub skipped: Vec<SkippedItem>,
}

impl ScanReport {
    pub fn selected_items(&self) -> Vec<TrashItem> {
        self.groups.iter().flat_map(|group| group.items.clone()).collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportGroup {
    pub kind: AdviceKind,
    pub keep_name: Option<String>,
    pub keep_path: Option<String>,
    pub note: Option<NoteCode>,
    pub delete_count: u32,
    pub freeable_bytes: u64,
    pub samples: Vec<String>,
    pub items: Vec<TrashItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrashItem {
    pub path: String,
    pub name: String,
    pub label: Option<String>,
    pub size: u64,
    pub modified_ms: i64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedItem {
    pub name: String,
    pub path: String,
    pub note: NoteCode,
}

pub fn scan(request: &ScanRequest) -> ScanReport {
    let mut user_roots = request.duplicate_roots.clone();
    for root in &request.installer_roots {
        if !user_roots.iter().any(|existing| existing == root) {
            user_roots.push(root.clone());
        }
    }
    let user_files = walk::collect_files(&user_roots, true);
    let cache_files = walk::collect_files(&request.cache_roots, false)
        .into_iter()
        .filter(|file| is_regenerable_cache_path(&file.path, &request.home))
        .collect::<Vec<_>>();

    let duplicate_indexes = indexes_under(&user_files, &request.duplicate_roots);
    let mut groups = Vec::new();
    let mut skipped = Vec::new();
    let mut consumed = std::collections::HashSet::<PathBuf>::new();

    for cluster in exact_clusters(&user_files, &duplicate_indexes, request.min_duplicate_file_bytes)
    {
        push_cluster(
            &user_files,
            &cluster,
            AdviceKind::ExactDuplicate,
            None,
            &mut groups,
            &mut skipped,
            &mut consumed,
        );
    }

    for cluster in version_clusters(&user_files, &duplicate_indexes, &consumed) {
        let note = if content_differs(&user_files, &cluster) {
            Some(NoteCode::ContentDiffers)
        } else {
            None
        };
        push_cluster(
            &user_files,
            &cluster,
            AdviceKind::OlderVersion,
            note,
            &mut groups,
            &mut skipped,
            &mut consumed,
        );
    }

    let mut installers = Vec::new();
    for file in user_files.iter().filter(|file| {
        request
            .installer_roots
            .iter()
            .any(|root| file.path.starts_with(root))
    }) {
        if consumed.contains(&file.path) {
            continue;
        }
        if is_old_installer(&file.path, &request.home, file.modified_ms, request.now_ms) {
            installers.push(file);
        }
    }
    if !installers.is_empty() {
        let items = installers
            .iter()
            .map(|file| trash_item(file))
            .collect::<Vec<_>>();
        groups.push(group_from_items(AdviceKind::OldInstaller, None, None, None, items));
    }

    let cache_items = cache_suggestions(&cache_files, &request.home, &mut skipped);
    if !cache_items.is_empty() {
        groups.push(group_from_items(
            AdviceKind::Regenerable,
            None,
            None,
            None,
            cache_items,
        ));
    }

    finish(groups, skipped)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn indexes_under(files: &[FileFact], roots: &[PathBuf]) -> Vec<usize> {
    files
        .iter()
        .enumerate()
        .filter(|(_, file)| roots.iter().any(|root| file.path.starts_with(root)))
        .map(|(index, _)| index)
        .collect()
}

fn exact_clusters(files: &[FileFact], indexes: &[usize], min_size: u64) -> Vec<Vec<usize>> {
    let big: Vec<usize> = indexes
        .iter()
        .copied()
        .filter(|&index| files[index].size >= min_size && files[index].size > 0)
        .collect();
    let reps = hardlink_representatives(files, &big);
    let mut by_size: HashMap<u64, Vec<usize>> = HashMap::new();
    for index in reps {
        by_size.entry(files[index].size).or_default().push(index);
    }
    let mut clusters = Vec::new();
    for group in by_size.into_values() {
        if group.len() < 2 {
            continue;
        }
        let mut by_partial: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
        for index in group {
            let Some(partial) = hash::partial_hash(&files[index].path, files[index].size) else {
                continue;
            };
            by_partial.entry(partial).or_default().push(index);
        }
        for partial_group in by_partial.into_values() {
            if partial_group.len() < 2 {
                continue;
            }
            let mut by_full: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
            for index in partial_group {
                let Some(full) = hash::full_hash(&files[index].path) else {
                    continue;
                };
                by_full.entry(full).or_default().push(index);
            }
            for full_group in by_full.into_values() {
                if full_group.len() < 2 {
                    continue;
                }
                let logical = collapse_clones(files, &full_group);
                if logical.len() >= 2 {
                    clusters.push(logical);
                }
            }
        }
    }
    clusters
}

fn hardlink_representatives(files: &[FileFact], indexes: &[usize]) -> Vec<usize> {
    let mut groups: HashMap<(u64, u64), Vec<usize>> = HashMap::new();
    for &index in indexes {
        groups
            .entry((files[index].dev, files[index].ino))
            .or_default()
            .push(index);
    }
    groups
        .into_values()
        .map(|members| pick(files, &members))
        .collect()
}

fn collapse_clones(files: &[FileFact], indexes: &[usize]) -> Vec<usize> {
    let mut identities = Vec::with_capacity(indexes.len());
    for &index in indexes {
        identities.push(clonefile::clone_identity(&files[index].path));
    }
    let mut by_clone: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut alone = Vec::new();
    for (offset, &index) in indexes.iter().enumerate() {
        if let Some((id, refs)) = identities[offset] {
            if refs > 1 && id != 0 {
                by_clone.entry(id).or_default().push(index);
                continue;
            }
        }
        alone.push(index);
    }
    for members in by_clone.into_values() {
        alone.push(pick(files, &members));
    }
    alone
}

fn version_clusters(
    files: &[FileFact],
    indexes: &[usize],
    consumed: &std::collections::HashSet<PathBuf>,
) -> Vec<Vec<usize>> {
    let mut groups: HashMap<(PathBuf, String), Vec<usize>> = HashMap::new();
    for &index in indexes {
        let file = &files[index];
        if consumed.contains(&file.path) {
            continue;
        }
        let Some(parent) = file.path.parent() else {
            continue;
        };
        let Some(name) = file.path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let stem = version_stem(name);
        if stem.is_empty() {
            continue;
        }
        groups
            .entry((parent.to_path_buf(), stem))
            .or_default()
            .push(index);
    }
    groups
        .into_values()
        .filter(|members| members.len() >= 2)
        .collect()
}

fn content_differs(files: &[FileFact], indexes: &[usize]) -> bool {
    let size = files[indexes[0]].size;
    if indexes.iter().any(|&index| files[index].size != size) {
        return true;
    }
    let Some(first) = hash::full_hash(&files[indexes[0]].path) else {
        return true;
    };
    indexes.iter().skip(1).any(|&index| {
        hash::full_hash(&files[index].path)
            .map(|hash| hash != first)
            .unwrap_or(true)
    })
}

fn push_cluster(
    files: &[FileFact],
    cluster: &[usize],
    kind: AdviceKind,
    note: Option<NoteCode>,
    groups: &mut Vec<ReportGroup>,
    skipped: &mut Vec<SkippedItem>,
    consumed: &mut std::collections::HashSet<PathBuf>,
) {
    if cluster.is_empty() {
        return;
    }
    let keep_at = pick(files, cluster);
    let keep_name = file_name(&files[keep_at].path);
    let mut items = Vec::new();
    for &index in cluster {
        consumed.insert(files[index].path.clone());
        if index == keep_at {
            continue;
        }
        let file = &files[index];
        if is_sensitive(&file.path).is_some() {
            skipped.push(SkippedItem {
                name: file_name(&file.path),
                path: file.path.to_string_lossy().into_owned(),
                note: NoteCode::Sensitive,
            });
            continue;
        }
        items.push(trash_item(file));
    }
    if items.is_empty() {
        return;
    }
    let keep_path = files[keep_at].path.to_string_lossy().into_owned();
    let mut group = group_from_items(kind, Some(keep_name), Some(keep_path), note, items);
    group.note = note;
    groups.push(group);
}

fn pick(files: &[FileFact], indexes: &[usize]) -> usize {
    let ranked = indexes
        .iter()
        .map(|&index| RankedFile {
            path: &files[index].path,
            modified_ms: files[index].modified_ms,
        })
        .collect::<Vec<_>>();
    indexes[keeper_index(&ranked)]
}

fn trash_item(file: &FileFact) -> TrashItem {
    TrashItem {
        path: file.path.to_string_lossy().into_owned(),
        name: file_name(&file.path),
        label: None,
        size: file.size,
        modified_ms: file.modified_ms,
        is_dir: false,
    }
}

fn cache_suggestions(
    files: &[FileFact],
    home: &Path,
    skipped: &mut Vec<SkippedItem>,
) -> Vec<TrashItem> {
    let mut buckets: HashMap<PathBuf, Vec<&FileFact>> = HashMap::new();
    for file in files {
        if !deletion_allowed(&file.path, home) && is_sensitive(&file.path).is_none() {
            continue;
        }
        buckets
            .entry(cache_bucket(&file.path, home))
            .or_default()
            .push(file);
    }
    let mut items = Vec::new();
    for (bucket, members) in buckets {
        let sensitive: Vec<_> = members
            .iter()
            .copied()
            .filter(|file| is_sensitive(&file.path).is_some())
            .collect();
        for file in &sensitive {
            skipped.push(SkippedItem {
                name: file_name(&file.path),
                path: file.path.to_string_lossy().into_owned(),
                note: NoteCode::Sensitive,
            });
        }
        if !sensitive.is_empty() {
            items.extend(members.iter().filter(|file| is_sensitive(&file.path).is_none()).map(|file| trash_item(file)));
            continue;
        }
        if members.is_empty() {
            continue;
        }
        let directory = bucket.is_dir() && members.iter().any(|file| file.path != bucket);
        if directory {
            let Some(modified_ms) = directory_modified_ms(&bucket) else {
                continue;
            };
            items.push(TrashItem {
                name: file_name(&bucket),
                label: Some(cache_label(&bucket)),
                path: bucket.to_string_lossy().into_owned(),
                size: members.iter().map(|file| file.size).sum(),
                modified_ms,
                is_dir: true,
            });
        } else {
            items.extend(members.iter().map(|file| trash_item(file)));
        }
    }
    items.sort_by(|left, right| right.size.cmp(&left.size).then_with(|| left.name.cmp(&right.name)));
    items
}

fn cache_bucket(path: &Path, home: &Path) -> PathBuf {
    let mut bucket = path.to_path_buf();
    while let Some(parent) = bucket.parent() {
        if !is_regenerable_cache_path(parent, home) {
            break;
        }
        bucket = parent.to_path_buf();
    }
    bucket
}

fn cache_label(path: &Path) -> String {
    match file_name(path).as_str() {
        "Google" | "com.google.Chrome" => "Chrome 浏览器缓存".to_string(),
        "com.apple.Safari" => "Safari 缓存".to_string(),
        "com.tencent.xinWeChat" => "微信缓存".to_string(),
        "com.spotify.client" => "Spotify 缓存".to_string(),
        "DiagnosticReports" => "应用日志".to_string(),
        "com.apple.QuickLook.thumbnailcache" => "预览缩略图".to_string(),
        other => other.to_string(),
    }
}

fn directory_modified_ms(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
}

fn group_from_items(
    kind: AdviceKind,
    keep_name: Option<String>,
    keep_path: Option<String>,
    note: Option<NoteCode>,
    items: Vec<TrashItem>,
) -> ReportGroup {
    let delete_count = items.len() as u32;
    let freeable_bytes = items.iter().map(|item| item.size).sum();
    let samples = items.iter().take(3).map(|item| item.name.clone()).collect();
    ReportGroup {
        kind,
        keep_name,
        keep_path,
        note,
        delete_count,
        freeable_bytes,
        samples,
        items,
    }
}

fn finish(groups: Vec<ReportGroup>, skipped: Vec<SkippedItem>) -> ScanReport {
    let delete_count = groups.iter().map(|group| group.delete_count).sum();
    let freeable_bytes = groups.iter().map(|group| group.freeable_bytes).sum();
    ScanReport {
        freeable_bytes,
        delete_count,
        skipped_sensitive_count: skipped.len() as u32,
        groups,
        skipped,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
