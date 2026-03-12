use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::types::{Index, Item};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Database {
    /// Primary storage — insertion order preserved.
    pub items: VecDeque<Item>,
    /// Index key → set of positions into `items`.
    pub index: BTreeMap<Index, BTreeSet<usize>>,
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an item and return its index.
    pub fn add_item(&mut self, item: Item) -> usize {
        let idx = self.items.len();
        self.items.push_back(item);
        idx
    }

    /// Remove the item at `idx`. All index entries are updated to reflect the
    /// shift caused by the removal. Returns the item, or `None` if out of bounds.
    pub fn remove_item(&mut self, idx: usize) -> Option<Item> {
        if idx >= self.items.len() {
            return None;
        }
        let item = self.items.remove(idx)?;

        for set in self.index.values_mut() {
            set.remove(&idx);
            // Every stored index above the removed slot shifts down by one.
            let shifted: Vec<usize> = set.iter().filter(|&&i| i > idx).copied().collect();
            for i in shifted {
                set.remove(&i);
                set.insert(i - 1);
            }
        }
        // Drop tag entries that became empty.
        self.index.retain(|_, set| !set.is_empty());

        Some(item)
    }
}
