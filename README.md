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
bookmarks [OPTIONS] [LINKS]...
```

### Configuration

Bookmarks looks for a config file in this order:

1. `--bookmarks-file` / `-f` flag (explicit path)
2. `bookmarks.toml` in the current directory
3. `$HOME/.config/bookmarks/bookmarks.toml` (global, auto-created)

Example:

```toml
[links]
github = "https://github.com"
linkedin = "https://linkedin.com"

[aliases]
gh = "github"
li = "linkedin"

[groups]
socials = ["gh", "linkedin"]
```

Links map to URLs, aliases map to links, and groups map to a list of aliases or links.

Use the `--config` or `--app` or `--webapp` option to edit the configuration file.

### Open links

Open links by name or alias or group:

```bash
bookmarks github
bookmarks gh linkedin
bookmarks socials
```

You can input multiple links, aliases, or groups at once. They will be opened in the order they are provided.

### Options

Available options:

| Flag | Short | Description |
|------|-------|-------------|
| `--bookmarks-file <PATH>` | `-f` | Use a specific bookmarks file |
| `--global` | `-g` | Use global config, ignore local bookmarks.toml |
| `--config` | `-c` | Open active bookmarks file in `$EDITOR` (use `-gc` for global) |
| `--app` | `-a` | Open desktop app (requires `app` feature) |
| `--webapp` | `-w` | Open the web app in browser (requires `webapp` feature) |
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |
