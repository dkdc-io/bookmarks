use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::config::{edit_config, print_config};
use crate::open::open_links;
use crate::storage::Storage;
use crate::toml_storage::TomlStorage;

#[derive(Parser, Debug)]
#[command(name = "bookmarks")]
#[command(about = "Bookmarks in your filesystem")]
#[command(version)]
pub struct Args {
    /// Path to bookmarks file (overrides cwd and global)
    #[arg(short = 'f', long = "bookmarks-file", conflicts_with = "global")]
    pub bookmarks_file: Option<PathBuf>,

    /// Use global config (~/.config/bookmarks/bookmarks.toml), ignore local
    #[arg(short, long, conflicts_with = "bookmarks_file")]
    pub global: bool,

    /// Open active bookmarks file in $EDITOR (use -gc for global)
    #[arg(short, long)]
    pub config: bool,

    /// Open the desktop app
    #[cfg(feature = "app")]
    #[arg(short = 'a', long)]
    pub app: bool,

    /// Open the webapp
    #[cfg(feature = "webapp")]
    #[arg(short = 'w', long)]
    pub webapp: bool,

    /// Things to open
    pub links: Vec<String>,
}

/// Resolve which bookmarks file to use and ensure it exists:
/// 1. --bookmarks-file flag (explicit, must exist)
/// 2. --global flag (skip cwd, use global)
/// 3. bookmarks.toml in cwd (local, must exist)
/// 4. ~/.config/bookmarks/bookmarks.toml (global, auto-created)
fn resolve_storage(bookmarks_file: Option<PathBuf>, global: bool) -> Result<TomlStorage> {
    if let Some(path) = bookmarks_file {
        anyhow::ensure!(
            path.exists(),
            "bookmarks file not found: {}",
            path.display()
        );
        return Ok(TomlStorage::new(path));
    }

    if !global
        && let Some(cwd_path) = TomlStorage::cwd_path()
        && cwd_path.exists()
    {
        return Ok(TomlStorage::new(cwd_path));
    }

    let storage = TomlStorage::with_default_path()?;
    storage.init()?;
    Ok(storage)
}

pub fn run<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = Args::parse_from(args);

    let storage = resolve_storage(args.bookmarks_file, args.global)?;

    #[cfg(feature = "app")]
    if args.app {
        return crate::app::run(Box::new(storage)).map_err(|e| anyhow::anyhow!("{e}"));
    }

    #[cfg(feature = "webapp")]
    if args.webapp {
        return crate::webapp::run(Box::new(storage));
    }

    if args.config {
        if let Some(path) = storage.path() {
            edit_config(path)?;
        }
        return Ok(());
    }

    let config = storage.load()?;

    if args.links.is_empty() {
        print_config(&config);
    } else {
        open_links(&args.links, &config)?;
    }

    if let Some(path) = storage.path() {
        println!(
            "(using {}, use --bookmarks-file to override)",
            path.display()
        );
    }

    Ok(())
}
