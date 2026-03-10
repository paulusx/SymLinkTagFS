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
// Index — category / kind of index key.
// IndexValue — the concrete lookup value for that category.
//
// Database stores: BTreeMap<Index, BTreeMap<IndexValue, BTreeSet<usize>>>
//   e.g. index[Fs][Fs{inode,dev,rdev}] → positions
//        index[Tag("foo")][Tag]         → positions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Index {
    Tag(String),
    Fs,
    Location,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexValue {
    /// Unit — Tag's string is already in the outer Index key.
    Tag,
    Fs { inode: u64, dev: u64, rdev: u64 },
    Location(PathBuf),
}
