use std::fs;
use std::io::Write;

use tempfile::tempdir;

const TEST_CONFIG: &str = r#"[urls]
github = { url = "https://github.com", aliases = ["gh"] }
dkdc-bookmarks = "https://github.com/lostmygithubaccount/bookmarks"

[groups]
dev = ["gh", "dkdc-bookmarks"]
"#;

fn write_config(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("bookmarks.toml");
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(TEST_CONFIG.as_bytes()).unwrap();
    path
}

#[test]
fn test_print_config() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path());
    let result = bookmarks::run_cli(["bookmarks", "-f", path.to_str().unwrap()]);
    assert!(result.is_ok());
}

#[test]
fn test_file_not_found() {
    let result = bookmarks::run_cli(["bookmarks", "-f", "/nonexistent/bookmarks.toml"]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "unexpected error: {err}");
}

#[test]
fn test_unknown_bookmark() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path());
    let result = bookmarks::run_cli(["bookmarks", "-f", path.to_str().unwrap(), "nonexistent"]);
    assert!(result.is_err());
}

#[test]
fn test_local_creates_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("bookmarks.toml");
    assert!(!config_path.exists());

    // Set cwd to temp dir so --local creates bookmarks.toml there
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let result = bookmarks::run_cli(["bookmarks", "--local"]);
    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    assert!(config_path.exists());
}
