# CLAUDE.md

## Project Overview

**SymLinkTagFS** is a read-only FUSE (Filesystem in Userspace) filesystem written in Rust that presents a flat, deduplicated view of a nested directory hierarchy. Files from a source tree are exposed in a single directory using the shortest unambiguous name possible, with path components joined by `__` when disambiguation is needed.

**License:** GNU GPLv3

## Repository Structure

```
SymLinkTagFS/
├── Cargo.toml       # Project manifest (edition 2024)
├── Cargo.lock       # Dependency lockfile
├── LICENSE          # GNU GPLv3
├── .gitignore       # Ignores /target
└── src/
    └── main.rs      # Entire implementation (~297 lines)
```

This is a single-file Rust crate. All logic lives in `src/main.rs`.

## Architecture

### Core Concepts

- **FlatFS** — The main FUSE filesystem struct. Holds an in-memory mapping of inodes to real file paths and display names.
- **FileEntry** — Maps an inode to its real filesystem path and its computed display name.
- **resolve_display_names()** — The naming algorithm that assigns the shortest unambiguous flat name to each file. Files start at depth 1 (just the filename); collisions are resolved by prepending parent directory components separated by `__`.

### Naming Algorithm Example

```
"a/foo.txt"     → "foo.txt"          (unique at depth 1)
"a/x/dup.txt"   → "a__x__dup.txt"   (collision resolved at depth 3)
"b/x/dup.txt"   → "b__x__dup.txt"
```

### FUSE Operations Implemented

| Method    | Purpose                          |
|-----------|----------------------------------|
| `lookup`  | Find files by name in root dir   |
| `getattr` | Get file/directory attributes     |
| `read`    | Read file contents (with seek)    |
| `readdir` | List all files in flat directory  |

The filesystem is **read-only** (mounted with `MountOption::RO`) and auto-unmounts on disconnect.

### Key Constants

- `TTL` = 1 second (attribute cache timeout)
- `ROOT_INO` = 1 (root directory inode)
- Files are assigned inodes starting at 2
- Root directory permissions: `0o555`, file permissions: `0o444`

## Build & Run

### Prerequisites

- Rust toolchain (edition 2024, Rust 1.85+)
- FUSE development headers (`libfuse-dev` on Debian/Ubuntu, `fuse-devel` on Fedora)

### Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo clippy             # Lint
cargo fmt --check        # Check formatting
```

### Usage

```bash
symlinktagfs <root> <mountpoint>
```

- `root` — source directory to flatten
- `mountpoint` — empty directory to mount the flat view into

## Dependencies

| Crate    | Version | Purpose                                  |
|----------|---------|------------------------------------------|
| `fuser`  | 0.14    | FUSE filesystem bindings                 |
| `walkdir`| 2       | Recursive directory traversal            |
| `clap`   | 4       | CLI argument parsing (with derive macro) |
| `libc`   | 0.2     | POSIX getuid/getgid for file ownership   |

## Code Conventions

- **Rust edition 2024** — use modern Rust idioms and features
- **Snake case** for functions and variables, **PascalCase** for types, **SCREAMING_CASE** for constants
- **Single-file architecture** — keep everything in `src/main.rs` unless the codebase grows significantly
- **Error handling** — use `?` operator where possible; FUSE callbacks return errors via `reply.error()` with libc constants (`ENOENT`, `ENOTDIR`, `EIO`)
- **Unsafe** — only used for `libc::getuid()` / `libc::getgid()` calls required by FUSE
- **No tests yet** — `resolve_display_names()` is the primary candidate for unit tests

## Git Workflow

- **Main branch:** `master`
- **Commit style:** Imperative mood, concise subject line (e.g., "Use shortest unambiguous flat name for each file")
- PRs are merged into `master`

## Notes for AI Assistants

- This is a small, focused project — avoid over-engineering or adding unnecessary abstractions
- The naming disambiguation algorithm in `resolve_display_names()` is the core logic — changes here need careful consideration
- The filesystem is intentionally read-only; do not add write support unless explicitly requested
- System-level FUSE dependency means `cargo build` requires `libfuse-dev` installed on the host
- There is no CI/CD pipeline — run `cargo build`, `cargo clippy`, and `cargo fmt --check` locally to validate changes
