# bookmarks

Bookmarks in your filesystem.

## Commands

```bash
bin/build          # Build all (Rust + Python)
bin/build-rs       # Build Rust crate
bin/build-py       # Build Python bindings (maturin develop)
bin/check          # Run all checks (format, lint, test)
bin/check-rs       # Rust checks (fmt, clippy, test)
bin/check-py       # Python checks (ruff, ty)
bin/test           # Run all tests
bin/test-rs        # Rust tests
bin/format         # Format all code
bin/install        # Install CLI (Rust + Python)
bin/bump-version   # Bump version (--patch, --minor (default), --major)
```

## Architecture

```
crates/bookmarks-core/    # Core library (config, storage, open, strings)
  src/lib.rs              # Library root — re-exports Config, Storage, TomlStorage
  src/config.rs           # Config struct, validation, parsing, editing
  src/storage.rs          # Storage trait (backend-agnostic)
  src/toml_storage.rs     # TOML file storage implementation
  src/open.rs             # URL resolution (name/alias → URL) and opening
  src/strings.rs          # Shared string constants and error templates
crates/bookmarks-app/     # iced desktop app
  src/lib.rs              # Desktop UI (depends on bookmarks-core)
  assets/icon.png         # App window icon
crates/bookmarks-webapp/  # Axum HTMX webapp (port 1414)
  src/lib.rs              # Web UI (depends on bookmarks-core)
crates/bookmarks-cli/     # CLI binary (dkdc-bookmarks on crates.io)
  src/main.rs             # Binary entry point
  src/lib.rs              # Re-exports core + run_cli
  src/cli.rs              # CLI (clap) with -f, -g, -l, --app, --webapp flags
crates/bookmarks-py/      # PyO3 bindings (cdylib)
py/bookmarks/             # Python wrapper + type stubs (core.pyi, py.typed)
```

Feature flags on `bookmarks-cli`: `app` (pulls in bookmarks-app), `webapp` (pulls in bookmarks-webapp).

Config resolution: `-f` flag > `-l` (local, auto-create) > `./bookmarks.toml` (cwd, must exist) > `~/.config/bookmarks/bookmarks.toml` (global, auto-created).

Config structure: `[urls]` maps names to URLs (plain string or `{ url, aliases }` table), `[groups]` expand to multiple url names/aliases. Aliases are defined inline on their parent URL entry.
