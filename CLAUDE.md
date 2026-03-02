# CLAUDE.md — SymLinkTagFS

AI assistant guidance for working in this repository.

## Project Overview

**SymLinkTagFS** is a Rust FUSE (Filesystem in Userspace) filesystem that presents a **flat, read-only view** of a nested directory hierarchy. All files from an arbitrarily deep source tree appear as a single flat directory at a mount point, with automatically disambiguated names using `__` as the path separator.

**Use case:** When you have a deep directory tree and want to browse or process all files as if they were in one flat directory, without any copies or symlinks.

## Repository Structure

```
SymLinkTagFS/
├── src/
│   └── main.rs       # Complete implementation (~297 lines, single source file)
├── Cargo.toml        # Package manifest and dependencies
├── Cargo.lock        # Locked dependency versions (committed, do not delete)
├── LICENSE           # GNU GPL v3
└── .gitignore        # Excludes /target
```

This is intentionally a single-file Rust project. Do not split it into modules unless the file grows significantly larger.

## Build & Run

```bash
# Build (debug)
cargo build

# Build (release, recommended for actual use)
cargo build --release

# Run
./target/release/symlinktagfs <root-dir> <mountpoint-dir>

# Example
mkdir /tmp/flat
./target/release/symlinktagfs ~/Documents /tmp/flat
ls /tmp/flat   # all nested files appear here with flat names
```

Both `<root-dir>` and `<mountpoint-dir>` must already exist and be directories. The filesystem auto-unmounts when the process terminates (via `AutoUnmount` mount option).

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `fuser` | 0.14 | Rust FUSE bindings |
| `walkdir` | 2 | Recursive directory traversal |
| `clap` | 4 (derive) | CLI argument parsing |
| `libc` | 0.2 | `getuid()`/`getgid()` system calls |

**System requirement:** Linux with FUSE kernel module and `libfuse` installed.

## Key Data Structures

### `FileEntry` (line 30)
Represents a single file in the flat namespace:
- `real_path: PathBuf` — absolute path to the actual file on disk
- `display_name: String` — the flat name shown at the mount point

### `FlatFS` (line 35)
The FUSE filesystem implementation:
- `files: HashMap<u64, FileEntry>` — inode → file entry (O(1) lookup by inode)
- `name_to_ino: HashMap<String, u64>` — display name → inode (O(1) lookup by name)
- `root_attr: FileAttr` — cached attributes for the root directory

## Core Algorithm: Name Disambiguation

`resolve_display_names()` (line 55) assigns the **shortest unambiguous flat name** to every file using an iterative depth-expansion approach:

1. Every file starts at `depth = 1` (just the filename).
2. Build a map from candidate name → list of files that would share it.
3. Any name with more than one file is a **conflict**; increment `depth` for all conflicting files.
4. Repeat until no conflicts remain.

Path components are joined with `__`. Example:

```
Source tree:          Flat names:
a/
  foo.txt          -> foo.txt          (unique at depth 1)
  x/
    bar.txt        -> bar.txt          (unique at depth 1)
    dup.txt  \
b/             >  -> a__x__dup.txt and b__x__dup.txt  (resolved at depth 3)
  x/
    dup.txt  /
```

**Do not change the `__` separator** without updating all related logic and documentation.

## FUSE Operations Implemented

Only the minimum set needed for a read-only flat filesystem:

| Method | Behavior |
|--------|----------|
| `lookup` | Find file by name under the root inode only |
| `getattr` | Return attributes for root dir or any file |
| `read` | Open real file, seek to offset, read bytes |
| `readdir` | Return `.`, `..`, then all flat files sorted by inode |

The filesystem is **strictly read-only** (`MountOption::RO`, permissions `0o444` for files, `0o555` for root). Do not add write operations.

## Constants

- `TTL: Duration = 1s` — FUSE attribute/entry cache TTL
- `ROOT_INO: u64 = 1` — inode number reserved for the root directory
- File inodes start at `2` and increment per file

## Development Conventions

### Code Style
- Follow standard `rustfmt` formatting (`cargo fmt`).
- Keep `main.rs` as the single source file unless the project grows substantially.
- Prefer explicit error handling; avoid `unwrap()` in FUSE handlers (use `reply.error(...)` instead).
- The only permitted `unsafe` blocks are the `libc::getuid()` / `libc::getgid()` calls — these are unavoidable as Rust has no safe API for them.

### Error Handling
- FUSE handler errors: always call `reply.error(ERRNO)` and return early.
- Startup errors: print to `stderr` and `std::process::exit(1)`.
- Use `libc::ENOENT` for not-found, `libc::ENOTDIR` for directory misuse, `libc::EIO` for I/O failures.

### Testing
There are no automated tests. Manual testing procedure:
1. Create a source tree with known structure including name collisions.
2. Mount the filesystem.
3. Verify flat names via `ls` and check file content with `cat`.
4. Unmount (process exit triggers `AutoUnmount`).

When adding features, consider adding a `#[cfg(test)]` module with unit tests for pure functions (especially `resolve_display_names`).

### Cargo.toml Notes
- `edition = "2024"` is set; ensure your Rust toolchain supports it.
- `Cargo.lock` is committed — this is correct for a binary crate.

## What This Filesystem Does NOT Support

- Write, create, delete, or rename operations (read-only by design)
- Subdirectories at the mount point (flat namespace only)
- Watching for changes in the source tree after mounting (snapshot at mount time)
- Hard links or symlinks in the flat view
- Extended attributes (xattrs)

Do not add these unless there is a clear requirement. Keep the implementation minimal.

## Git Workflow

- Main branch: `master`
- Feature branches follow the pattern: `claude/<description>-<id>`
- Commit messages are imperative and descriptive (e.g., "Use shortest unambiguous flat name for each file")
- PR merges are squash-or-merge into `master`
