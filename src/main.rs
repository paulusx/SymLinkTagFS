use clap::Parser;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::{ENOENT, ENOTDIR};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const TTL: Duration = Duration::from_secs(1);
const ROOT_INO: u64 = 1;

#[derive(Parser)]
#[command(
    name = "symlinktagfs",
    about = "FUSE filesystem that presents a flat view of a directory hierarchy"
)]
struct Args {
    /// Root of the source directory hierarchy
    root: PathBuf,

    /// Directory to mount the flat view into
    mountpoint: PathBuf,
}

struct FileEntry {
    real_path: PathBuf,
    display_name: String,
}

struct FlatFS {
    /// inode -> file entry
    files: HashMap<u64, FileEntry>,
    /// display name -> inode (for lookup)
    name_to_ino: HashMap<String, u64>,
    root_attr: FileAttr,
}

impl FlatFS {
    fn new(root: &Path) -> std::io::Result<Self> {
        let mut files: HashMap<u64, FileEntry> = HashMap::new();
        let mut name_to_ino: HashMap<String, u64> = HashMap::new();
        let mut next_ino: u64 = 2;

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let real_path = entry.path().to_path_buf();
            let rel = real_path.strip_prefix(root).unwrap();

            // Build a flat name from the relative path: "a/b/c.txt" -> "a__b__c.txt"
            // This guarantees uniqueness while keeping the name human-readable.
            let display_name = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("__");

            name_to_ino.insert(display_name.clone(), next_ino);
            files.insert(next_ino, FileEntry { real_path, display_name });
            next_ino += 1;
        }

        let now = SystemTime::now();
        let root_attr = FileAttr {
            ino: ROOT_INO,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::Directory,
            perm: 0o555,
            nlink: 2,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 512,
            flags: 0,
        };

        eprintln!("Indexed {} file(s) from {}", files.len(), root.display());

        Ok(FlatFS { files, name_to_ino, root_attr })
    }

    fn file_attr(&self, ino: u64, entry: &FileEntry) -> Option<FileAttr> {
        let m = std::fs::metadata(&entry.real_path).ok()?;
        let atime = m.accessed().unwrap_or_else(|_| UNIX_EPOCH.into());
        let mtime = m.modified().unwrap_or_else(|_| UNIX_EPOCH.into());
        Some(FileAttr {
            ino,
            size: m.len(),
            blocks: (m.len() + 511) / 512,
            atime,
            mtime,
            ctime: mtime,
            crtime: mtime,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 512,
            flags: 0,
        })
    }
}

impl Filesystem for FlatFS {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent != ROOT_INO {
            reply.error(ENOENT);
            return;
        }
        let name_str = name.to_string_lossy();
        match self.name_to_ino.get(name_str.as_ref()).copied() {
            Some(ino) => match self.files.get(&ino).and_then(|e| self.file_attr(ino, e)) {
                Some(attr) => reply.entry(&TTL, &attr, 0),
                None => reply.error(ENOENT),
            },
            None => reply.error(ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if ino == ROOT_INO {
            reply.attr(&TTL, &self.root_attr);
            return;
        }
        match self.files.get(&ino).and_then(|e| self.file_attr(ino, e)) {
            Some(attr) => reply.attr(&TTL, &attr),
            None => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let Some(entry) = self.files.get(&ino) else {
            reply.error(ENOENT);
            return;
        };
        match std::fs::File::open(&entry.real_path) {
            Ok(mut file) => {
                if file.seek(SeekFrom::Start(offset as u64)).is_err() {
                    reply.error(libc::EIO);
                    return;
                }
                let mut buf = vec![0u8; size as usize];
                match file.read(&mut buf) {
                    Ok(n) => reply.data(&buf[..n]),
                    Err(_) => reply.error(libc::EIO),
                }
            }
            Err(_) => reply.error(libc::EIO),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != ROOT_INO {
            reply.error(ENOTDIR);
            return;
        }

        // "." and ".." are at offsets 0 and 1; files follow.
        let dot_entries = [
            (ROOT_INO, FileType::Directory, "."),
            (ROOT_INO, FileType::Directory, ".."),
        ];

        // Collect files in a stable order so offsets are consistent.
        let mut file_entries: Vec<(u64, &str)> = self
            .files
            .iter()
            .map(|(&ino, e)| (ino, e.display_name.as_str()))
            .collect();
        file_entries.sort_by_key(|&(ino, _)| ino);

        for (i, (ftype_ino, kind, name)) in dot_entries
            .iter()
            .map(|&(ino, kind, name)| (ino, kind, name))
            .chain(file_entries.iter().map(|&(ino, name)| (ino, FileType::RegularFile, name)))
            .enumerate()
            .skip(offset as usize)
        {
            // reply.add returns true when the buffer is full.
            if reply.add(ftype_ino, (i + 1) as i64, kind, name) {
                break;
            }
        }
        reply.ok();
    }
}

fn main() {
    let args = Args::parse();

    if !args.root.is_dir() {
        eprintln!("Error: root '{}' is not a directory", args.root.display());
        std::process::exit(1);
    }
    if !args.mountpoint.is_dir() {
        eprintln!(
            "Error: mountpoint '{}' is not a directory",
            args.mountpoint.display()
        );
        std::process::exit(1);
    }

    let fs = FlatFS::new(&args.root).expect("Failed to build filesystem index");

    let options = vec![
        MountOption::RO,
        MountOption::FSName("flatfs".to_string()),
        MountOption::AutoUnmount,
    ];

    fuser::mount2(fs, &args.mountpoint, &options).expect("Failed to mount filesystem");
}
