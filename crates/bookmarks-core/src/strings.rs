//! Shared string constants used across CLI, app, and webapp.

// -- Project ----------------------------------------------------------------

pub const PROJECT_URL: &str = "https://dkdc.io/bookmarks/";

// -- Placeholders ------------------------------------------------------------

pub const PH_URL_NAME: &str = "url name";
pub const PH_URL: &str = "https://...";
pub const PH_ALIAS: &str = "alias (optional)";
pub const PH_GROUP_NAME: &str = "group name";
pub const PH_GROUP_ENTRIES: &str = "url name, alias, ...";
pub const PH_FILTER: &str = "filter...";

// -- Error templates ---------------------------------------------------------

pub fn err_group_entries_missing(missing: &[&str]) -> String {
    format!("group entries not found: {}", missing.join(", "))
}
