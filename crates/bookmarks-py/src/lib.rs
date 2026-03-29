use std::collections::HashMap;
use std::path::PathBuf;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use bookmarks_core::storage::Storage;

fn to_py_err(e: anyhow::Error) -> PyErr {
    PyErr::new::<PyRuntimeError, _>(e.to_string())
}

// -- UrlEntry -----------------------------------------------------------------

#[pyclass(name = "UrlEntry", skip_from_py_object)]
#[derive(Clone)]
struct PyUrlEntry {
    inner: bookmarks_core::UrlEntry,
}

#[pymethods]
impl PyUrlEntry {
    #[new]
    #[pyo3(signature = (url, aliases=None))]
    fn new(url: String, aliases: Option<Vec<String>>) -> Self {
        let inner = match aliases {
            Some(aliases) if !aliases.is_empty() => {
                bookmarks_core::UrlEntry::Full { url, aliases }
            }
            _ => bookmarks_core::UrlEntry::Simple(url),
        };
        Self { inner }
    }

    #[getter]
    fn url(&self) -> &str {
        self.inner.url()
    }

    #[setter]
    fn set_url(&mut self, url: String) {
        self.inner.set_url(url);
    }

    #[getter]
    fn aliases(&self) -> Vec<String> {
        self.inner.aliases().to_vec()
    }

    fn add_alias(&mut self, alias: String) {
        self.inner.add_alias(alias);
    }

    fn remove_alias(&mut self, alias: &str) {
        self.inner.remove_alias(alias);
    }

    fn has_alias(&self, alias: &str) -> bool {
        self.inner.has_alias(alias)
    }

    fn __repr__(&self) -> String {
        let aliases = self.inner.aliases();
        if aliases.is_empty() {
            format!("UrlEntry(url={:?})", self.inner.url())
        } else {
            format!(
                "UrlEntry(url={:?}, aliases={:?})",
                self.inner.url(),
                aliases
            )
        }
    }
}

// -- Config -------------------------------------------------------------------

#[pyclass(name = "Config", from_py_object)]
#[derive(Clone)]
struct PyConfig {
    inner: bookmarks_core::Config,
}

#[pymethods]
impl PyConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: bookmarks_core::Config::default(),
        }
    }

    #[staticmethod]
    fn from_toml(s: &str) -> PyResult<Self> {
        let inner: bookmarks_core::Config =
            toml::from_str(s).map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;
        Ok(Self { inner })
    }

    fn to_toml(&self) -> PyResult<String> {
        toml::to_string(&self.inner)
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))
    }

    #[getter]
    fn urls(&self) -> HashMap<String, PyUrlEntry> {
        self.inner
            .urls
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    PyUrlEntry {
                        inner: v.clone(),
                    },
                )
            })
            .collect()
    }

    #[getter]
    fn groups(&self) -> HashMap<String, Vec<String>> {
        self.inner.groups.clone()
    }

    fn resolve(&self, name: &str) -> Option<String> {
        self.inner.resolve(name).map(|s| s.to_string())
    }

    fn contains(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    fn validate(&self) -> Vec<String> {
        self.inner.validate()
    }

    #[pyo3(signature = (name, url, aliases=None))]
    fn add_url(&mut self, name: String, url: String, aliases: Option<Vec<String>>) {
        let entry = match aliases {
            Some(aliases) if !aliases.is_empty() => {
                bookmarks_core::UrlEntry::Full { url, aliases }
            }
            _ => bookmarks_core::UrlEntry::Simple(url),
        };
        self.inner.urls.insert(name, entry);
    }

    fn rename_url(&mut self, old: &str, new: &str) -> PyResult<()> {
        self.inner.rename_url(old, new).map_err(to_py_err)
    }

    fn rename_alias(&mut self, old: &str, new: &str) -> PyResult<()> {
        self.inner.rename_alias(old, new).map_err(to_py_err)
    }

    fn delete_url(&mut self, name: &str) -> PyResult<()> {
        self.inner.delete_url(name).map_err(to_py_err)
    }

    fn delete_alias(&mut self, alias: &str) -> PyResult<()> {
        self.inner.delete_alias(alias).map_err(to_py_err)
    }

    fn rename_group(&mut self, old: &str, new: &str) -> PyResult<()> {
        self.inner.rename_group(old, new).map_err(to_py_err)
    }

    fn delete_group(&mut self, name: &str) -> PyResult<()> {
        self.inner.delete_group(name).map_err(to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "Config(urls={}, groups={})",
            self.inner.urls.len(),
            self.inner.groups.len()
        )
    }
}

// -- TomlStorage --------------------------------------------------------------

#[pyclass(name = "TomlStorage")]
struct PyTomlStorage {
    inner: bookmarks_core::TomlStorage,
}

#[pymethods]
impl PyTomlStorage {
    #[new]
    fn new(path: String) -> Self {
        Self {
            inner: bookmarks_core::TomlStorage::new(PathBuf::from(path)),
        }
    }

    #[staticmethod]
    fn default_path() -> PyResult<String> {
        bookmarks_core::TomlStorage::default_path()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(to_py_err)
    }

    #[staticmethod]
    fn with_default_path() -> PyResult<Self> {
        let inner = bookmarks_core::TomlStorage::with_default_path().map_err(to_py_err)?;
        Ok(Self { inner })
    }

    fn load(&self) -> PyResult<PyConfig> {
        let inner = self.inner.load().map_err(to_py_err)?;
        Ok(PyConfig { inner })
    }

    fn save(&self, config: &PyConfig) -> PyResult<()> {
        self.inner.save(&config.inner).map_err(to_py_err)
    }

    fn init(&self) -> PyResult<()> {
        self.inner.init().map_err(to_py_err)
    }

    fn backend_name(&self) -> &str {
        self.inner.backend_name()
    }

    fn path(&self) -> Option<String> {
        self.inner.path().map(|p| p.to_string_lossy().into_owned())
    }
}

// -- Module -------------------------------------------------------------------

#[pyfunction]
fn run_cli(argv: Vec<String>) -> PyResult<()> {
    bookmarks::run_cli(argv.iter().map(|s| s.as_str())).map_err(to_py_err)
}

#[pymodule]
mod core {
    use super::*;

    #[pymodule_init]
    fn module_init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(run_cli, m)?)?;
        m.add_class::<PyUrlEntry>()?;
        m.add_class::<PyConfig>()?;
        m.add_class::<PyTomlStorage>()?;
        Ok(())
    }
}
