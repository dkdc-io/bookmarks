use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use bookmarks_core::config::{edit_config, print_config};
use bookmarks_core::open::open_links;
use bookmarks_core::storage::Storage;
use bookmarks_core::toml_storage::TomlStorage;

#[derive(Parser, Debug)]
#[command(name = "bookmarks")]
#[command(about = "Bookmarks in your filesystem")]
#[command(version)]
pub struct Args {
    /// Path to bookmarks file (overrides cwd and global)
    #[arg(short = 'f', long = "bookmarks-file", conflicts_with = "global")]
    pub bookmarks_file: Option<PathBuf>,

    /// Use global config (~/.config/bookmarks/bookmarks.toml), ignore local
    #[arg(short, long, conflicts_with_all = ["bookmarks_file", "local"])]
    pub global: bool,

    /// Use local config (./bookmarks.toml), create if missing
    #[arg(short, long, conflicts_with_all = ["bookmarks_file", "global"])]
    pub local: bool,

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

    /// Things to open (url names, aliases, or groups)
    pub urls: Vec<String>,
}

/// Resolve which bookmarks file to use and ensure it exists:
/// 1. --bookmarks-file flag (explicit, must exist)
/// 2. --local flag (cwd, auto-created)
/// 3. --global flag (skip cwd, use global)
/// 4. bookmarks.toml in cwd (local, must exist)
/// 5. ~/.config/bookmarks/bookmarks.toml (global, auto-created)
fn resolve_storage(
    bookmarks_file: Option<PathBuf>,
    global: bool,
    local: bool,
) -> Result<TomlStorage> {
    if let Some(path) = bookmarks_file {
        anyhow::ensure!(
            path.exists(),
            "bookmarks file not found: {}",
            path.display()
        );
        return Ok(TomlStorage::new(path));
    }

    if local {
        let cwd_path = TomlStorage::cwd_path().context("failed to get current directory")?;
        let storage = TomlStorage::new(cwd_path);
        storage.init()?;
        return Ok(storage);
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

pub fn run_cli<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = Args::parse_from(args);

    let storage = resolve_storage(args.bookmarks_file, args.global, args.local)?;

    #[cfg(feature = "app")]
    if args.app {
        return bookmarks_app::run_app(Box::new(storage)).map_err(|e| anyhow::anyhow!("{e}"));
    }

    #[cfg(feature = "webapp")]
    if args.webapp {
        return bookmarks_webapp::run_webapp(Box::new(storage));
    }

    if args.config {
        match storage.path() {
            Some(path) => edit_config(path)?,
            None => anyhow::bail!("no bookmarks file found to edit"),
        }
        return Ok(());
    }

    let config = storage.load()?;

    if args.urls.is_empty() {
        print_config(&config);
    } else {
        open_links(&args.urls, &config)?;
    }

    if let Some(path) = storage.path() {
        println!(
            "(using {}, use --bookmarks-file to override)",
            path.display()
        );
    }

    Ok(())
}
