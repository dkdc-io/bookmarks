import tempfile
from pathlib import Path

from bookmarks import Config, TomlStorage, UrlEntry

TEST_CONFIG = """\
[urls]
github = { url = "https://github.com", aliases = ["gh"] }
dkdc = "https://dkdc.io"

[groups]
dev = ["gh", "dkdc"]
"""


def test_url_entry_create():
    e = UrlEntry("https://github.com", ["gh", "github"])
    assert e.url == "https://github.com"
    assert e.aliases == ["gh", "github"]
    assert e.has_alias("gh")
    assert not e.has_alias("nope")


def test_url_entry_no_aliases():
    e = UrlEntry("https://example.com")
    assert e.url == "https://example.com"
    assert e.aliases == []


def test_url_entry_mutate():
    e = UrlEntry("https://example.com")
    e.url = "https://new.com"
    assert e.url == "https://new.com"
    e.add_alias("ex")
    assert e.has_alias("ex")
    e.remove_alias("ex")
    assert not e.has_alias("ex")


def test_config_from_toml():
    c = Config.from_toml(TEST_CONFIG)
    assert len(c.urls) == 2
    assert "github" in c.urls
    assert "dkdc" in c.urls
    assert c.groups == {"dev": ["gh", "dkdc"]}


def test_config_resolve():
    c = Config.from_toml(TEST_CONFIG)
    assert c.resolve("github") == "https://github.com"
    assert c.resolve("gh") == "https://github.com"
    assert c.resolve("dkdc") == "https://dkdc.io"
    assert c.resolve("nope") is None


def test_config_contains():
    c = Config.from_toml(TEST_CONFIG)
    assert c.contains("github")
    assert c.contains("gh")
    assert not c.contains("nope")


def test_config_validate():
    toml = """\
[urls]
a = { url = "https://a.com", aliases = ["x"] }
b = { url = "https://b.com", aliases = ["x"] }
"""
    c = Config.from_toml(toml)
    warnings = c.validate()
    assert len(warnings) == 1
    assert "x" in warnings[0]


def test_config_add_url():
    c = Config()
    c.add_url("example", "https://example.com", ["ex"])
    assert c.resolve("ex") == "https://example.com"
    assert c.resolve("example") == "https://example.com"


def test_config_rename_url():
    c = Config.from_toml(TEST_CONFIG)
    c.rename_url("dkdc", "dkdc-io")
    assert c.resolve("dkdc-io") == "https://dkdc.io"
    assert c.resolve("dkdc") is None
    # cascades to groups
    assert "dkdc-io" in c.groups["dev"]


def test_config_delete_url():
    c = Config.from_toml(TEST_CONFIG)
    c.delete_url("github")
    assert not c.contains("github")
    assert not c.contains("gh")
    # group should only have "dkdc" left
    assert c.groups["dev"] == ["dkdc"]


def test_config_to_toml_roundtrip():
    c = Config.from_toml(TEST_CONFIG)
    toml_str = c.to_toml()
    c2 = Config.from_toml(toml_str)
    assert len(c.urls) == len(c2.urls)
    assert c.resolve("github") == c2.resolve("github")
    assert c.resolve("gh") == c2.resolve("gh")
    assert c.resolve("dkdc") == c2.resolve("dkdc")
    assert c.groups == c2.groups


def test_toml_storage_roundtrip():
    with tempfile.TemporaryDirectory() as tmpdir:
        path = str(Path(tmpdir) / "bookmarks.toml")
        storage = TomlStorage(path)
        storage.init()

        config = storage.load()
        assert len(config.urls) > 0  # default config has entries

        config.add_url("test", "https://test.com")
        storage.save(config)

        reloaded = storage.load()
        assert reloaded.resolve("test") == "https://test.com"


def test_toml_storage_default_path():
    path = TomlStorage.default_path()
    assert "bookmarks" in path
    assert path.endswith(".toml")
