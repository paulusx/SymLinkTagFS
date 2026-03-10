use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::types::{Item, Tag};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    /// Primary storage — insertion order preserved.
    pub items: VecDeque<Item>,
    /// Tag → set of indices into `items`.
    pub index: BTreeMap<Tag, BTreeSet<usize>>,
}
