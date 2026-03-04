use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

// ---------------------------------------------------------------------------
// ItemId — opaque handle; serializes as a string for JSON map-key compat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(u64);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for ItemId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for ItemId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<u64>().map(ItemId).map_err(de::Error::custom)
    }
}

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
// Tag — stores references (ItemId) to items, not owned copies
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub items: HashSet<ItemId>,
}

// ---------------------------------------------------------------------------
// DatabaseData — serialization-only shape (no derived indices)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct DatabaseData {
    next_id: u64,
    items: HashMap<ItemId, Item>,
    tags: HashMap<String, HashSet<ItemId>>,
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

pub struct Database {
    next_id: u64,
    items: HashMap<ItemId, Item>,
    /// tag name → set of item IDs it references
    tags: HashMap<String, HashSet<ItemId>>,
    /// derived: item ID → set of tag names  (reverse index, fast tags_of)
    item_tags: HashMap<ItemId, HashSet<String>>,
    /// derived: origin_location → item ID   (fast item_by_path)
    path_index: HashMap<PathBuf, ItemId>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            next_id: 0,
            items: HashMap::new(),
            tags: HashMap::new(),
            item_tags: HashMap::new(),
            path_index: HashMap::new(),
        }
    }
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    // -- internal --

    fn rebuild_indices(&mut self) {
        self.item_tags.clear();
        for (name, ids) in &self.tags {
            for &id in ids {
                self.item_tags.entry(id).or_default().insert(name.clone());
            }
        }
        self.path_index.clear();
        for (&id, item) in &self.items {
            self.path_index.insert(item.origin_location.clone(), id);
        }
    }

    // -- items --

    /// Insert an item and return its new ID.
    pub fn add_item(&mut self, item: Item) -> ItemId {
        let id = ItemId(self.next_id);
        self.next_id += 1;
        self.path_index.insert(item.origin_location.clone(), id);
        self.items.insert(id, item);
        id
    }

    /// Remove an item and detach it from every tag. Returns the item if it existed.
    pub fn remove_item(&mut self, id: ItemId) -> Option<Item> {
        let item = self.items.remove(&id)?;
        self.path_index.remove(&item.origin_location);
        if let Some(tag_names) = self.item_tags.remove(&id) {
            for name in &tag_names {
                if let Some(ids) = self.tags.get_mut(name) {
                    ids.remove(&id);
                }
            }
        }
        Some(item)
    }

    pub fn item(&self, id: ItemId) -> Option<&Item> {
        self.items.get(&id)
    }

    // -- tags --

    /// Create an empty tag. Returns `false` if the name already exists.
    pub fn add_tag(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.tags.contains_key(&name) {
            return false;
        }
        self.tags.insert(name, HashSet::new());
        true
    }

    /// Remove a tag and all its item associations. Returns `false` if not found.
    pub fn remove_tag(&mut self, name: &str) -> bool {
        let Some(ids) = self.tags.remove(name) else { return false };
        for id in &ids {
            if let Some(names) = self.item_tags.get_mut(id) {
                names.remove(name);
            }
        }
        true
    }

    /// Associate an item with a tag. Both must already exist; returns `false` otherwise.
    pub fn tag_item(&mut self, tag: &str, id: ItemId) -> bool {
        if !self.items.contains_key(&id) {
            return false;
        }
        let Some(ids) = self.tags.get_mut(tag) else { return false };
        if ids.insert(id) {
            self.item_tags.entry(id).or_default().insert(tag.to_string());
        }
        true
    }

    /// Remove an item–tag association. Returns `false` if tag not found or item not in tag.
    pub fn untag_item(&mut self, tag: &str, id: ItemId) -> bool {
        let Some(ids) = self.tags.get_mut(tag) else { return false };
        if ids.remove(&id) {
            if let Some(names) = self.item_tags.get_mut(&id) {
                names.remove(tag);
            }
            true
        } else {
            false
        }
    }

    // -- queries --

    /// All tag names associated with `id`. O(1).
    pub fn tags_of(&self, id: ItemId) -> Option<&HashSet<String>> {
        self.item_tags.get(&id)
    }

    /// All item IDs in `tag`. O(1).
    pub fn items_in_tag(&self, tag: &str) -> Option<&HashSet<ItemId>> {
        self.tags.get(tag)
    }

    /// Look up an item by its origin path. O(1).
    pub fn item_by_path(&self, path: &Path) -> Option<(ItemId, &Item)> {
        let &id = self.path_index.get(path)?;
        Some((id, self.items.get(&id)?))
    }
}

// Custom Serialize/Deserialize: persist only canonical data; rebuild indices on load.

impl Serialize for Database {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        DatabaseData {
            next_id: self.next_id,
            items: self.items.clone(),
            tags: self.tags.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Database {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = DatabaseData::deserialize(d)?;
        let mut db = Database {
            next_id: data.next_id,
            items: data.items,
            tags: data.tags,
            item_tags: HashMap::new(),
            path_index: HashMap::new(),
        };
        db.rebuild_indices();
        Ok(db)
    }
}
