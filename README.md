# Bookmarks

[![GitHub Release](https://img.shields.io/github/v/release/dkdc-io/bookmarks?color=blue)](https://github.com/dkdc-io/bookmarks/releases)
[![PyPI](https://img.shields.io/pypi/v/dkdc-bookmarks?color=blue)](https://pypi.org/project/dkdc-bookmarks/)
[![crates.io](https://img.shields.io/crates/v/dkdc-bookmarks?color=blue)](https://crates.io/crates/dkdc-bookmarks)
[![CI](https://img.shields.io/github/actions/workflow/status/dkdc-io/bookmarks/ci.yml?branch=main&label=CI)](https://github.com/dkdc-io/bookmarks/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/dkdc-io/bookmarks/blob/main/LICENSE)

Bookmarks in your filesystem.

```text
bookmarks.toml
(filesystem config)
       |
       v
bookmarks-core
(config + storage + open)
       |
       +--> bookmarks CLI
       |       +--> opens names, aliases, groups
       |       +--> --webapp starts Axum webapp
       |       +--> --app starts Tauri desktop app
       |
       +--> bookmarks-webapp
       |    (Axum UI/routes on localhost:1414)
       |
       +--> bookmarks-app
            (Tauri shell)
              |
              v
       127.0.0.1:0 loopback webapp
              |
              v
       Tauri WebView
              |
              +--> local app routes stay in the WebView
              +--> bookmark URLs open in the OS/browser
```

Screenshot of the web/desktop app:

![Bookmarks web and desktop application](https://raw.githubusercontent.com/dkdc-io/bookmarks/main/assets/bookmarks-webapp.png)

_Web & desktop application._

## Install

Recommended:

```bash
curl -LsSf https://dkdc.sh/bookmarks/install.sh | sh
```

Pre-built binaries are available for Linux and macOS via Python (`uv`). Windows users should install via `cargo` or use macOS/Linux.

uv:

```bash
uv tool install dkdc-bookmarks
```

cargo:

```bash
cargo install dkdc-bookmarks --features app,webapp
```

Verify installation:

```bash
bookmarks --version
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
dkdc-bookmarks = "https://github.com/dkdc-io/bookmarks"
github = { url = "https://github.com", aliases = ["gh"] }

[urls.linkedin]
url = "https://linkedin.com"
aliases = ["li"]

[groups]
socials = ["gh", "linkedin"]
```

URLs can be plain strings, inline tables with aliases, or expanded tables. Groups reference url names or aliases.

Use `--config` to edit the configuration file in `$EDITOR`, or use `--app` / `--webapp` for the local GUI.

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
| `--app` | `-a` | Open Tauri desktop app (requires `app` feature, which includes `webapp`) |
| `--webapp` | `-w` | Open the web app in browser (requires `webapp` feature) |
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |
