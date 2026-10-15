use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const SAMPLE: usize = 64 * 1024;

pub fn partial_hash(path: &Path, size: u64) -> Option<[u8; 32]> {
    let mut file = File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(&size.to_le_bytes());
    let mut buf = vec![0_u8; SAMPLE];
    let read = file.read(&mut buf).ok()?;
    hasher.update(&buf[..read]);
    if size > SAMPLE as u64 {
        file.seek(SeekFrom::Start(size.saturating_sub(SAMPLE as u64)))
            .ok()?;
        let read = file.read(&mut buf).ok()?;
        hasher.update(&buf[..read]);
    }
    Some(*hasher.finalize().as_bytes())
}

pub fn full_hash(path: &Path) -> Option<[u8; 32]> {
    let mut file = File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buf).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Some(*hasher.finalize().as_bytes())
}
