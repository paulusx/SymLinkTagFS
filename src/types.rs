use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub inode: u64,
    pub dev: u64,
    pub rdev: u64,
    pub origin_location: PathBuf,
}

// ---------------------------------------------------------------------------
// Index — discriminated index key; one variant per indexable field plus a
// free-form Tag(String) for user-supplied labels.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Index {
    Tag(String),
    Fs { inode: u64, dev: u64, rdev: u64 },
    Location(PathBuf),
}
