use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, DEFAULT_CONFIG};
use crate::storage::Storage;

const CONFIG_DIR: &str = ".config";
const APP_NAME: &str = "bookmarks";
const CONFIG_FILENAME: &str = "bookmarks.toml";

pub struct TomlStorage {
    path: PathBuf,
}

impl TomlStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Default config path: ~/.config/bookmarks/bookmarks.toml
    pub fn default_path() -> Result<PathBuf> {
        // Intentionally use ~/.config/ rather than dirs::config_dir(), which
        // returns ~/Library/Application Support/ on macOS. We want a single
        // consistent dotfile location across platforms.
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(CONFIG_DIR).join(APP_NAME).join(CONFIG_FILENAME))
    }

    /// Local config path: ./bookmarks.toml in the current working directory.
    pub fn cwd_path() -> Option<PathBuf> {
        std::env::current_dir()
            .ok()
            .map(|d| d.join(CONFIG_FILENAME))
    }

    pub fn with_default_path() -> Result<Self> {
        Ok(Self::new(Self::default_path()?))
    }
}

impl Storage for TomlStorage {
    fn load(&self) -> Result<Config> {
        let contents = fs::read_to_string(&self.path).context("Failed to read config file")?;
        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        for warning in config.validate() {
            eprintln!("[bookmarks] warning: {warning}");
        }

        Ok(config)
    }

    fn save(&self, config: &Config) -> Result<()> {
        let contents = toml::to_string(config).context("Failed to serialize config")?;
        fs::write(&self.path, contents).context("Failed to write config file")?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        if !self.path.exists() {
            let config_dir = self
                .path
                .parent()
                .context("Invalid config path: no parent directory")?;
            fs::create_dir_all(config_dir).context("Failed to create config directory")?;
            fs::write(&self.path, DEFAULT_CONFIG).context("Failed to write default config")?;
        }
        Ok(())
    }

    fn backend_name(&self) -> &str {
        "toml"
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_path() {
        let path = TomlStorage::default_path().unwrap();
        assert!(path.ends_with(".config/bookmarks/bookmarks.toml"));
    }

    #[test]
    fn test_load_save_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookmarks.toml");

        let storage = TomlStorage::new(path.clone());

        // Write a config manually
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[urls]
github = {{ url = "https://github.com", aliases = ["gh"] }}
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"

[groups]
dev = ["gh"]
"#
        )
        .unwrap();

        let config = storage.load().unwrap();
        assert_eq!(config.urls.get("github").unwrap().aliases(), &["gh"]);
        assert_eq!(
            config.urls.get("dkdc-bookmarks").unwrap().url(),
            "https://github.com/dkdc-io/bookmarks"
        );

        // Save and reload
        storage.save(&config).unwrap();
        let reloaded = storage.load().unwrap();
        assert_eq!(config.urls.len(), reloaded.urls.len());
        assert_eq!(config.groups, reloaded.groups);
    }

    #[test]
    fn test_init_creates_default_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("bookmarks.toml");

        let storage = TomlStorage::new(path.clone());
        storage.init().unwrap();

        assert!(path.exists());
        let config = storage.load().unwrap();
        assert!(!config.urls.is_empty());
    }

    #[test]
    fn test_init_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookmarks.toml");

        fs::write(
            &path,
            "[urls]\ndkdc-bookmarks = \"https://github.com/dkdc-io/bookmarks\"\n",
        )
        .unwrap();

        let storage = TomlStorage::new(path);
        storage.init().unwrap();

        let config = storage.load().unwrap();
        assert_eq!(
            config.urls.get("dkdc-bookmarks").unwrap().url(),
            "https://github.com/dkdc-io/bookmarks"
        );
    }

    #[test]
    fn test_backend_name() {
        let storage = TomlStorage::new(PathBuf::from("/tmp/test.toml"));
        assert_eq!(storage.backend_name(), "toml");
    }
}
