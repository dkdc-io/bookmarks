use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const DEFAULT_EDITOR: &str = "vi";

/// A URL entry: either a plain URL string or a table with url + optional aliases.
///
/// In TOML this means all three forms work:
/// ```toml
/// [urls]
/// dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"
/// github = { url = "https://github.com", aliases = ["gh"] }
///
/// [urls.linkedin]
/// url = "https://linkedin.com"
/// aliases = ["li", "ln"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum UrlEntry {
    Simple(String),
    Full {
        url: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        aliases: Vec<String>,
    },
}

impl UrlEntry {
    pub fn url(&self) -> &str {
        match self {
            UrlEntry::Simple(url) => url,
            UrlEntry::Full { url, .. } => url,
        }
    }

    pub fn aliases(&self) -> &[String] {
        match self {
            UrlEntry::Simple(_) => &[],
            UrlEntry::Full { aliases, .. } => aliases,
        }
    }

    pub fn set_url(&mut self, new_url: String) {
        match self {
            UrlEntry::Simple(url) => *url = new_url,
            UrlEntry::Full { url, .. } => *url = new_url,
        }
    }

    pub fn add_alias(&mut self, alias: String) {
        match self {
            UrlEntry::Simple(url) => {
                *self = UrlEntry::Full {
                    url: url.clone(),
                    aliases: vec![alias],
                };
            }
            UrlEntry::Full { aliases, .. } => {
                if !aliases.contains(&alias) {
                    aliases.push(alias);
                }
            }
        }
    }

    pub fn remove_alias(&mut self, alias: &str) {
        if let UrlEntry::Full { aliases, .. } = self {
            aliases.retain(|a| a != alias);
        }
    }

    pub fn has_alias(&self, alias: &str) -> bool {
        self.aliases().iter().any(|a| a == alias)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub urls: HashMap<String, UrlEntry>,
    #[serde(default)]
    pub groups: HashMap<String, Vec<String>>,
}

pub const DEFAULT_CONFIG: &str = r#"# https://github.com/dkdc-io/bookmarks
# bookmarks config file

[urls]
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"
github = { url = "https://github.com", aliases = ["gh"] }

[urls.linkedin]
url = "https://linkedin.com"
aliases = ["li"]

[groups]
socials = ["gh", "linkedin"]
"#;

impl Config {
    /// Build a reverse lookup: alias → url_name.
    fn alias_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        for (name, entry) in &self.urls {
            for alias in entry.aliases() {
                map.insert(alias.as_str(), name.as_str());
            }
        }
        map
    }

    /// Resolve a name (url name or alias) to a URL string.
    pub fn resolve(&self, name: &str) -> Option<&str> {
        // Direct url name
        if let Some(entry) = self.urls.get(name) {
            return Some(entry.url());
        }
        // Alias lookup
        for entry in self.urls.values() {
            if entry.has_alias(name) {
                return Some(entry.url());
            }
        }
        None
    }

    /// Check if a name is a known url name or alias.
    pub fn contains(&self, name: &str) -> bool {
        self.resolve(name).is_some()
    }

    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for duplicate aliases across urls
        let mut seen_aliases: HashMap<&str, &str> = HashMap::new();
        for (url_name, entry) in &self.urls {
            for alias in entry.aliases() {
                if let Some(other) = seen_aliases.get(alias.as_str()) {
                    warnings.push(format!(
                        "alias '{alias}' is defined on both '{url_name}' and '{other}'"
                    ));
                } else {
                    seen_aliases.insert(alias.as_str(), url_name.as_str());
                }
                // Alias shadows a url name
                if self.urls.contains_key(alias.as_str()) {
                    warnings.push(format!(
                        "alias '{alias}' on '{url_name}' shadows url name '{alias}'"
                    ));
                }
            }
        }

        // Check group entries
        for (group, entries) in &self.groups {
            for entry in entries {
                if !self.contains(entry) {
                    warnings.push(format!(
                        "group '{group}' contains '{entry}' which is not a url name or alias"
                    ));
                }
            }
        }

        warnings
    }

    /// Rename a url key and cascade to group entries that reference it by name.
    pub fn rename_url(&mut self, old: &str, new: &str) -> Result<()> {
        if old == new {
            anyhow::ensure!(self.urls.contains_key(old), "url '{old}' not found");
            return Ok(());
        }
        if self.urls.contains_key(new) {
            anyhow::bail!("url '{new}' already exists");
        }
        // Check if new name collides with an existing alias
        let alias_map = self.alias_map();
        if alias_map.contains_key(new) {
            anyhow::bail!("'{new}' already exists as an alias");
        }
        let entry = self
            .urls
            .remove(old)
            .with_context(|| format!("url '{old}' not found"))?;
        self.urls.insert(new.to_string(), entry);

        // Update group entries that reference the old url name
        for entries in self.groups.values_mut() {
            for e in entries.iter_mut() {
                if e == old {
                    *e = new.to_string();
                }
            }
        }

        Ok(())
    }

    /// Rename an alias and cascade to group entries.
    pub fn rename_alias(&mut self, old: &str, new: &str) -> Result<()> {
        if old == new {
            return Ok(());
        }
        if self.urls.contains_key(new) {
            anyhow::bail!("'{new}' already exists as a url name");
        }
        let alias_map = self.alias_map();
        if alias_map.contains_key(new) {
            anyhow::bail!("alias '{new}' already exists");
        }

        // Find which url owns this alias
        let url_name = alias_map
            .get(old)
            .with_context(|| format!("alias '{old}' not found"))?
            .to_string();

        let entry = self.urls.get_mut(&url_name).unwrap();
        entry.remove_alias(old);
        entry.add_alias(new.to_string());

        // Update group entries
        for entries in self.groups.values_mut() {
            for e in entries.iter_mut() {
                if e == old {
                    *e = new.to_string();
                }
            }
        }

        Ok(())
    }

    /// Delete a url and clean up group references to it and its aliases.
    pub fn delete_url(&mut self, name: &str) -> Result<()> {
        let entry = self
            .urls
            .remove(name)
            .with_context(|| format!("url '{name}' not found"))?;
        // Collect the url name + all its aliases for group cleanup
        let mut to_remove: Vec<String> = vec![name.to_string()];
        to_remove.extend(entry.aliases().iter().cloned());
        for entries in self.groups.values_mut() {
            entries.retain(|e| !to_remove.contains(e));
        }
        self.groups.retain(|_, entries| !entries.is_empty());
        Ok(())
    }

    /// Delete an alias from its parent url and clean up group references.
    pub fn delete_alias(&mut self, alias: &str) -> Result<()> {
        let alias_map = self.alias_map();
        let url_name = alias_map
            .get(alias)
            .with_context(|| format!("alias '{alias}' not found"))?
            .to_string();

        self.urls.get_mut(&url_name).unwrap().remove_alias(alias);

        for entries in self.groups.values_mut() {
            entries.retain(|e| e != alias);
        }
        self.groups.retain(|_, entries| !entries.is_empty());
        Ok(())
    }

    /// Rename a group key.
    pub fn rename_group(&mut self, old: &str, new: &str) -> Result<()> {
        if old != new && self.groups.contains_key(new) {
            anyhow::bail!("group '{new}' already exists");
        }
        let entries = self
            .groups
            .remove(old)
            .with_context(|| format!("group '{old}' not found"))?;
        self.groups.insert(new.to_string(), entries);
        Ok(())
    }

    /// Delete a group.
    pub fn delete_group(&mut self, name: &str) -> Result<()> {
        self.groups
            .remove(name)
            .with_context(|| format!("group '{name}' not found"))?;
        Ok(())
    }
}

pub fn edit_config(config_path: &Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

    println!("Opening {} with {}...", config_path.display(), editor);

    let status = std::process::Command::new(&editor)
        .arg(config_path)
        .status()
        .with_context(|| format!("Editor {editor} not found in PATH"))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    Ok(())
}

pub fn print_config(config: &Config) {
    if !config.urls.is_empty() {
        println!("urls:");
        println!();

        let mut entries: Vec<_> = config.urls.iter().collect();
        entries.sort_unstable_by_key(|(k, _)| k.as_str());

        let max_key_len = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

        for (name, entry) in &entries {
            let aliases = entry.aliases();
            if aliases.is_empty() {
                println!("• {name:<max_key_len$} | {}", entry.url());
            } else {
                println!(
                    "• {name:<max_key_len$} | {} (aliases: {})",
                    entry.url(),
                    aliases.join(", ")
                );
            }
        }

        println!();
    }

    if !config.groups.is_empty() {
        println!("groups:");
        println!();

        let mut entries: Vec<_> = config.groups.iter().collect();
        entries.sort_unstable_by_key(|(k, _)| k.as_str());

        let max_key_len = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

        for (name, group_entries) in &entries {
            println!("• {name:<max_key_len$} | [{}]", group_entries.join(", "));
        }

        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh"] }

[groups]
dev = ["gh"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let entry = config.urls.get("github").unwrap();
        assert_eq!(entry.url(), "https://github.com");
        assert_eq!(entry.aliases(), &["gh"]);
        assert_eq!(config.groups.get("dev"), Some(&vec!["gh".to_string()]));
    }

    #[test]
    fn test_parse_simple_url() {
        let toml = r#"
[urls]
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let entry = config.urls.get("dkdc-bookmarks").unwrap();
        assert_eq!(entry.url(), "https://github.com/dkdc-io/bookmarks");
        assert!(entry.aliases().is_empty());
    }

    #[test]
    fn test_parse_expanded_table() {
        let toml = r#"
[urls.linkedin]
url = "https://linkedin.com"
aliases = ["li", "ln"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let entry = config.urls.get("linkedin").unwrap();
        assert_eq!(entry.url(), "https://linkedin.com");
        assert_eq!(entry.aliases(), &["li", "ln"]);
    }

    #[test]
    fn test_parse_hybrid_config() {
        let toml = r#"
[urls]
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"
github = { url = "https://github.com", aliases = ["gh"] }

[urls.linkedin]
url = "https://linkedin.com"
aliases = ["li"]

[groups]
socials = ["gh", "linkedin"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.urls.len(), 3);
        assert_eq!(
            config.urls.get("dkdc-bookmarks").unwrap().url(),
            "https://github.com/dkdc-io/bookmarks"
        );
        assert_eq!(config.urls.get("github").unwrap().aliases(), &["gh"]);
        assert_eq!(config.urls.get("linkedin").unwrap().aliases(), &["li"]);
        assert!(config.validate().is_empty());
    }

    #[test]
    fn test_parse_empty_config() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.urls.is_empty());
        assert!(config.groups.is_empty());
    }

    #[test]
    fn test_config_roundtrip() {
        let mut config = Config::default();
        config.urls.insert(
            "example".to_string(),
            UrlEntry::Full {
                url: "https://example.com".to_string(),
                aliases: vec!["ex".to_string()],
            },
        );
        config
            .groups
            .insert("g".to_string(), vec!["ex".to_string()]);

        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(config.urls.len(), deserialized.urls.len());
        assert_eq!(config.groups, deserialized.groups);
    }

    #[test]
    fn test_default_config_parses() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert!(!config.urls.is_empty());
        assert!(!config.groups.is_empty());
    }

    #[test]
    fn test_valid_config_has_no_warnings() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert!(config.validate().is_empty());
    }

    #[test]
    fn test_resolve_by_url_name() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(
            config.resolve("dkdc-bookmarks"),
            Some("https://github.com/dkdc-io/bookmarks")
        );
    }

    #[test]
    fn test_resolve_by_alias() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.resolve("gh"), Some("https://github.com"));
    }

    #[test]
    fn test_resolve_unknown() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.resolve("nope"), None);
    }

    #[test]
    fn test_duplicate_alias_warns() {
        let toml = r#"
[urls]
a = { url = "https://a.com", aliases = ["x"] }
b = { url = "https://b.com", aliases = ["x"] }
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("x"));
    }

    #[test]
    fn test_alias_shadows_url_name_warns() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["dkdc-bookmarks"] }
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let warnings = config.validate();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("shadows")));
    }

    #[test]
    fn test_broken_group_entry_warns() {
        let toml = r#"
[urls]
real = "https://example.com"

[groups]
dev = ["real", "ghost"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("ghost"));
    }

    #[test]
    fn test_rename_url_cascades_groups() {
        let toml = r#"
[urls]
github = "https://github.com"

[groups]
dev = ["github"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.rename_url("github", "gh-link").unwrap();
        assert!(config.urls.contains_key("gh-link"));
        assert!(!config.urls.contains_key("github"));
        assert_eq!(config.groups.get("dev"), Some(&vec!["gh-link".to_string()]));
    }

    #[test]
    fn test_rename_alias_cascades_groups() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh"] }

[groups]
dev = ["gh"]
all = ["gh", "other"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.rename_alias("gh", "github-alias").unwrap();
        let entry = config.urls.get("github").unwrap();
        assert!(entry.has_alias("github-alias"));
        assert!(!entry.has_alias("gh"));
        assert_eq!(
            config.groups.get("dev"),
            Some(&vec!["github-alias".to_string()])
        );
        let all = config.groups.get("all").unwrap();
        assert!(all.contains(&"github-alias".to_string()));
        assert!(all.contains(&"other".to_string()));
    }

    #[test]
    fn test_rename_nonexistent_url_errors() {
        let mut config = Config::default();
        assert!(config.rename_url("nope", "new").is_err());
    }

    #[test]
    fn test_rename_nonexistent_alias_errors() {
        let mut config = Config::default();
        assert!(config.rename_alias("nope", "new").is_err());
    }

    #[test]
    fn test_rename_url_collision_errors() {
        let toml = r#"
[urls]
a = "https://a.com"
b = "https://b.com"
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        let result = config.rename_url("a", "b");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        assert!(config.urls.contains_key("a"));
        assert!(config.urls.contains_key("b"));
    }

    #[test]
    fn test_rename_alias_collision_errors() {
        let toml = r#"
[urls]
a = { url = "https://a.com", aliases = ["x"] }
b = { url = "https://b.com", aliases = ["y"] }
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        let result = config.rename_alias("x", "y");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_rename_url_to_existing_alias_errors() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh"] }
other = "https://other.com"
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        let result = config.rename_url("other", "gh");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already exists as an alias")
        );
        assert!(config.urls.contains_key("other"));
    }

    #[test]
    fn test_rename_alias_to_existing_url_errors() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh"] }
other = "https://other.com"
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        let result = config.rename_alias("gh", "other");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already exists as a url name")
        );
        assert!(config.urls.get("github").unwrap().has_alias("gh"));
    }

    #[test]
    fn test_rename_url_same_name_is_noop() {
        let toml = r#"
[urls]
a = "https://a.com"
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.rename_url("a", "a").unwrap();
        assert_eq!(config.urls.get("a").unwrap().url(), "https://a.com");
    }

    #[test]
    fn test_delete_url_cascades() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh", "g"] }
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"

[groups]
dev = ["gh", "github"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.delete_url("github").unwrap();
        assert!(!config.urls.contains_key("github"));
        assert!(config.urls.contains_key("dkdc-bookmarks"));
        // Group entries for both the url name and its aliases are removed
        assert!(!config.groups.contains_key("dev"));
    }

    #[test]
    fn test_delete_url_partial_group_cleanup() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh"] }
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"

[groups]
dev = ["gh", "dkdc-bookmarks"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.delete_url("github").unwrap();
        let dev = config.groups.get("dev").unwrap();
        assert_eq!(dev, &vec!["dkdc-bookmarks".to_string()]);
    }

    #[test]
    fn test_delete_alias_cascades_to_groups() {
        let toml = r#"
[urls]
github = { url = "https://github.com", aliases = ["gh"] }

[groups]
dev = ["gh"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.delete_alias("gh").unwrap();
        // Alias removed from url entry
        assert!(config.urls.get("github").unwrap().aliases().is_empty());
        // Group with only "gh" is now empty and removed
        assert!(!config.groups.contains_key("dev"));
    }

    #[test]
    fn test_delete_group() {
        let toml = r#"
[groups]
dev = ["gh"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.delete_group("dev").unwrap();
        assert!(!config.groups.contains_key("dev"));
    }

    #[test]
    fn test_rename_group_collision_errors() {
        let toml = r#"
[groups]
a = ["x"]
b = ["y"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        let result = config.rename_group("a", "b");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        assert!(config.groups.contains_key("a"));
        assert!(config.groups.contains_key("b"));
    }

    #[test]
    fn test_rename_group_cascades() {
        let toml = r#"
[groups]
dev = ["gh", "dkdc-bookmarks"]
"#;
        let mut config: Config = toml::from_str(toml).unwrap();
        config.rename_group("dev", "development").unwrap();
        assert!(!config.groups.contains_key("dev"));
        assert_eq!(
            config.groups.get("development"),
            Some(&vec!["gh".to_string(), "dkdc-bookmarks".to_string()])
        );
    }

    #[test]
    fn test_delete_nonexistent_errors() {
        let mut config = Config::default();
        assert!(config.delete_url("nope").is_err());
        assert!(config.delete_alias("nope").is_err());
        assert!(config.delete_group("nope").is_err());
    }
}
