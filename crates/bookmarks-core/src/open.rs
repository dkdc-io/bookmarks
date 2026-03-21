use anyhow::{Context, Result};

use crate::config::Config;

pub fn resolve_uri<'a>(name: &str, config: &'a Config) -> Result<&'a str> {
    config
        .resolve(name)
        .with_context(|| format!("'{name}' not found in [urls] names or aliases"))
}

fn open_it(link: &str) -> Result<()> {
    open::that(link).with_context(|| format!("failed to open {link}"))?;
    println!("opening {link}...");
    Ok(())
}

pub fn expand_groups<'a>(names: &'a [String], config: &'a Config) -> Vec<&'a str> {
    let mut seen = std::collections::HashSet::new();
    let mut expanded = Vec::new();
    for name in names {
        if let Some(group_items) = config.groups.get(name.as_str()) {
            for item in group_items {
                if seen.insert(item.as_str()) {
                    expanded.push(item.as_str());
                }
            }
        } else if seen.insert(name.as_str()) {
            expanded.push(name.as_str());
        }
    }
    expanded
}

pub fn open_links(names: &[String], config: &Config) -> Result<()> {
    let expanded = expand_groups(names, config);
    let mut errors = Vec::new();

    for name in &expanded {
        match resolve_uri(name, config) {
            Ok(uri) => {
                if let Err(e) = open_it(uri) {
                    eprintln!("[bookmarks] failed to open {name}: {e}");
                    errors.push(format!("{name}: {e}"));
                }
            }
            Err(e) => {
                eprintln!("[bookmarks] skipping {name}: {e}");
                errors.push(format!("{name}: {e}"));
            }
        }
    }

    if !errors.is_empty() && errors.len() == expanded.len() {
        anyhow::bail!("all urls failed to open");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UrlEntry;
    use std::collections::HashMap;

    fn test_config() -> Config {
        let mut urls = HashMap::new();
        urls.insert(
            "github".to_string(),
            UrlEntry::Full {
                url: "https://github.com".to_string(),
                aliases: vec!["gh".to_string()],
            },
        );
        urls.insert(
            "google".to_string(),
            UrlEntry::Full {
                url: "https://google.com".to_string(),
                aliases: vec!["g".to_string()],
            },
        );
        urls.insert(
            "rust".to_string(),
            UrlEntry::Simple("https://rust-lang.org".to_string()),
        );

        let mut groups = HashMap::new();
        groups.insert(
            "dev".to_string(),
            vec!["gh".to_string(), "rust".to_string()],
        );

        Config { urls, groups }
    }

    #[test]
    fn test_alias_resolves_to_uri() {
        let config = test_config();
        let uri = resolve_uri("gh", &config).unwrap();
        assert_eq!(uri, "https://github.com");
    }

    #[test]
    fn test_url_name_resolves_to_uri() {
        let config = test_config();
        let uri = resolve_uri("rust", &config).unwrap();
        assert_eq!(uri, "https://rust-lang.org");
    }

    #[test]
    fn test_url_name_with_aliases_resolves() {
        let config = test_config();
        let uri = resolve_uri("github", &config).unwrap();
        assert_eq!(uri, "https://github.com");
    }

    #[test]
    fn test_unknown_name_errors() {
        let config = test_config();
        let result = resolve_uri("unknown", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_expand_group() {
        let config = test_config();
        let names = vec!["dev".to_string()];
        let expanded = expand_groups(&names, &config);
        assert_eq!(expanded, vec!["gh", "rust"]);
    }

    #[test]
    fn test_mixed_groups_and_names() {
        let config = test_config();
        let names = vec!["dev".to_string(), "google".to_string()];
        let expanded = expand_groups(&names, &config);
        assert_eq!(expanded, vec!["gh", "rust", "google"]);
    }

    #[test]
    fn test_expand_groups_deduplicates() {
        let config = test_config();
        // "gh" appears in group "dev" and also directly
        let names = vec!["dev".to_string(), "gh".to_string()];
        let expanded = expand_groups(&names, &config);
        assert_eq!(expanded, vec!["gh", "rust"]);
    }
}
