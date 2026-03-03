use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Tag-based filesystem using symlinks")]
struct Args {
    /// Source directories to index (must exist)
    #[arg(long, num_args = 1.., value_name = "DIR")]
    sources: Vec<PathBuf>,

    /// Mount point (must be an empty directory or must not exist)
    #[arg(long, value_name = "DIR")]
    mount: PathBuf,

    /// Database file path (will be created if it does not exist)
    #[arg(long, value_name = "FILE")]
    database: PathBuf,
}

fn validate_args(args: &Args) -> Result<(), String> {
    for src in &args.sources {
        if !src.is_dir() {
            return Err(format!("source is not an existing directory: {}", src.display()));
        }
    }

    if args.mount.exists() {
        if !args.mount.is_dir() {
            return Err(format!("mount path exists but is not a directory: {}", args.mount.display()));
        }
        let is_empty = args.mount.read_dir()
            .map_err(|e| format!("cannot read mount directory: {e}"))?
            .next()
            .is_none();
        if !is_empty {
            return Err(format!("mount directory is not empty: {}", args.mount.display()));
        }
    }

    if let Some(parent) = args.database.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!("database parent directory does not exist: {}", parent.display()));
        }
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(e) = validate_args(&args) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }

    println!("sources:  {:?}", args.sources);
    println!("mount:    {}", args.mount.display());
    println!("database: {}", args.database.display());
}
