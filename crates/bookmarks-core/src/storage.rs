use anyhow::Result;

use crate::config::Config;

/// Backend-agnostic storage for link data.
///
/// Operates on `Config` as a whole — fine-grained CRUD can be added
/// as default methods later.
pub trait Storage: Send + Sync {
    /// Load the full config (urls, groups).
    fn load(&self) -> Result<Config>;

    /// Save the full config, replacing whatever was stored.
    fn save(&self, config: &Config) -> Result<()>;

    /// Initialize storage if it doesn't exist yet.
    fn init(&self) -> Result<()>;

    /// Human-readable backend name ("toml", "database", etc).
    fn backend_name(&self) -> &str;

    /// Path to the underlying storage (if file-based).
    fn path(&self) -> Option<&std::path::Path> {
        None
    }
}
