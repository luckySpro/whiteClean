use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
    pub version: u32,
    pub operations: Vec<Operation>,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            version: 1,
            operations: Vec::new(),
        }
    }
}

impl Journal {
    pub fn load(path: &Path) -> Self {
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), ()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| ())?;
        }
        let tmp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| ())?;
        fs::write(&tmp, bytes).map_err(|_| ())?;
        fs::rename(&tmp, path).map_err(|_| ())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub created_ms: i64,
    pub undone: bool,
    pub items: Vec<JournalItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalItem {
    pub original_path: String,
    pub trash_path: String,
    pub size: u64,
    pub modified_ms: i64,
    pub restored: bool,
    pub missing: bool,
}

pub trait TruncateFront {
    fn truncate_front(&mut self, keep: usize);
}

impl TruncateFront for Vec<Operation> {
    fn truncate_front(&mut self, keep: usize) {
        if self.len() > keep {
            let drop_count = self.len() - keep;
            self.drain(0..drop_count);
        }
    }
}
