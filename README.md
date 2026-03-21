# Bookmarks

Bookmarks in your filesystem.

[![PyPI](https://img.shields.io/pypi/v/dkdc-bookmarks?color=blue)](https://pypi.org/project/dkdc-bookmarks/)
[![crates.io](https://img.shields.io/crates/v/dkdc-bookmarks?color=blue)](https://crates.io/crates/dkdc-bookmarks)
[![CI](https://img.shields.io/github/actions/workflow/status/lostmygithubaccount/bookmarks/ci.yml?branch=main&label=CI)](https://github.com/lostmygithubaccount/bookmarks/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/lostmygithubaccount/bookmarks/blob/main/LICENSE)

## Install

uv (recommended):

```bash
uv tool install --from dkdc-bookmarks bookmarks
```

cargo:

```bash
cargo install dkdc-bookmarks --features app,webapp
```

You can use `uvx` to run it without installing:

```bash
uvx --from dkdc-bookmarks bookmarks
```

## Usage

```bash
bookmarks [OPTIONS] [URLS]...
```

### Configuration

Bookmarks looks for a config file in this order:

1. `--bookmarks-file` / `-f` flag (explicit path)
2. `--local` / `-l` flag (creates `./bookmarks.toml` if missing)
3. `bookmarks.toml` in the current directory (must exist)
4. `$HOME/.config/bookmarks/bookmarks.toml` (global, auto-created)

Example:

```toml
[urls]
rust = "https://rust-lang.org"
github = { url = "https://github.com", aliases = ["gh"] }

[urls.linkedin]
url = "https://linkedin.com"
aliases = ["li"]

[groups]
socials = ["gh", "li"]
```

URLs can be plain strings, inline tables with aliases, or expanded tables. Groups reference url names or aliases.

Use the `--config` or `--app` or `--webapp` option to edit the configuration file.

### Open urls

Open urls by name, alias, or group:

```bash
bookmarks github
bookmarks gh linkedin
bookmarks socials
```

You can input multiple url names, aliases, or groups at once. They will be opened in the order they are provided.

### Options

Available options:

| Flag | Short | Description |
|------|-------|-------------|
| `--bookmarks-file <PATH>` | `-f` | Use a specific bookmarks file |
| `--global` | `-g` | Use global config, ignore local bookmarks.toml |
| `--local` | `-l` | Use local config (`./bookmarks.toml`), create if missing |
| `--config` | `-c` | Open active bookmarks file in `$EDITOR` (use `-gc` for global) |
| `--app` | `-a` | Open desktop app (requires `app` feature) |
| `--webapp` | `-w` | Open the web app in browser (requires `webapp` feature) |
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |
