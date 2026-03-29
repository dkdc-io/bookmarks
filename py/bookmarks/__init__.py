import sys

from bookmarks.core import Config, TomlStorage, UrlEntry, run_cli

__all__ = ["run", "run_cli", "main", "Config", "UrlEntry", "TomlStorage"]


def run(argv: list[str] | None = None) -> None:
    """Run the bookmarks CLI with the given arguments."""
    if argv is None:
        argv = sys.argv
    try:
        run_cli(argv)
    except KeyboardInterrupt:
        sys.exit(130)


def main() -> None:
    """CLI entry point."""
    run()
