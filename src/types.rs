use std::path::PathBuf;

pub struct Item {
    pub inode: u64,
    pub dev: u64,
    pub rdev: u64,
    pub origin_location: PathBuf,
}

pub struct Tag {
    pub name: String,
    pub items: Vec<Item>,
}
