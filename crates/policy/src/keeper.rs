use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct RankedFile<'a> {
    pub path: &'a Path,
    pub modified_ms: i64,
}

/// Documents outrank Pictures, then Desktop, then Downloads.
pub fn location_rank(path: &Path) -> u8 {
    for component in path.components() {
        match component.as_os_str().to_string_lossy().as_ref() {
            "Documents" => return 3,
            "Pictures" => return 2,
            "Desktop" => return 1,
            "Downloads" => return 0,
            _ => {}
        }
    }
    0
}

/// Newest modification time wins. Equal times keep the higher-ranked folder.
pub fn keeper_index(files: &[RankedFile<'_>]) -> usize {
    debug_assert!(!files.is_empty());
    let mut best = 0;
    for (index, file) in files.iter().enumerate().skip(1) {
        let winner = files[best];
        let newer = file.modified_ms > winner.modified_ms;
        let same_time = file.modified_ms == winner.modified_ms;
        let better_place = location_rank(file.path) > location_rank(winner.path);
        let same_place = location_rank(file.path) == location_rank(winner.path);
        let stabler_path = file.path < winner.path;
        if newer || (same_time && better_place) || (same_time && same_place && stabler_path) {
            best = index;
        }
    }
    best
}
