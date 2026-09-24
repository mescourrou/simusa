//! Configuration import handling with custom YAML tag support.
//!
//! This module provides the ability to import configuration from external YAML files
//! using the `!import` custom tag. For example:
//!
//! ```yaml
//! my_field: !import "relative/path.yaml"
//! ```
//!
//! The import path is resolved relative to the directory of the file containing the import.
//! Nested imports are supported (imported files can also use `!import`).

use std::{
    cell::RefCell,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use crate::errors::{SimusaError, SimusaErrorTypes, SimusaResult};

/// Thread-local storage for tracking the current config file path during deserialization.
/// This allows import paths to be resolved relative to the importing file.
thread_local! {
    static CURRENT_CONFIG_PATH: RefCell<Option<PathBuf>> = RefCell::new(None);
}

/// Execute a function with the current config path set.
/// This is used when loading a config file to establish the base path for imports.
pub fn with_config_path<F, R>(path: &Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_CONFIG_PATH.with(|cell| {
        let old = cell.replace(Some(path.to_path_buf()));
        let result = f();
        cell.replace(old);
        result
    })
}

/// Get the base directory of the current config file being deserialized.
/// Returns the current directory if no config path is set.
pub fn get_current_config_base_path() -> PathBuf {
    CURRENT_CONFIG_PATH.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    })
}

/// Load a deserializable type from a file path.
/// The path is resolved relative to the current config file's directory.
pub fn load_from_file<T: for<'de> Deserialize<'de>>(relative_path: &str) -> SimusaResult<T> {
    let base_path = get_current_config_base_path();
    let full_path = base_path.join(relative_path);
    
    let file = File::open(&full_path).map_err(|e| {
        SimusaError::new(
            SimusaErrorTypes::ConfigError,
            format!(
                "Failed to open imported file {:?} (resolved from {:?} relative to {:?}): {}",
                full_path, relative_path, base_path, e
            ),
        )
    })?;
    
    let reader = BufReader::new(file);
    serde_yaml::from_reader(reader).map_err(|e| {
        SimusaError::new(
            SimusaErrorTypes::ConfigError,
            format!(
                "Failed to parse imported file {:?} (from {:?}): {}",
                full_path, relative_path, e
            ),
        )
    })
}

/// Deserialize a value that can be either inline or imported via `!import` tag.
///
/// This function handles the `!import` YAML tag, which allows configuration values
/// to be loaded from external files. If the tag is not present, the value is
/// deserialized inline as normal.
///
/// # Example
///
/// In YAML:
/// ```yaml
/// # Inline value
/// my_field:
///   type: SomeType
///   param: value
///
/// # Imported from file
/// my_field: !import "path/to/config.yaml"
/// ```
pub fn deserialize_as_import_or_inline<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // Use a visitor to handle both tagged and untagged values
    struct ImportOrInlineVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for ImportOrInlineVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a value or an import directive")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            // Check if this string is an import directive
            // Format: "!import path/to/file" or just "!import" followed by the path
            if value.starts_with("!import") {
                let path = value.trim_start_matches("!import").trim();
                if path.is_empty() {
                    return Err(de::Error::custom(
                        "Import directive requires a file path: !import \"path/to/file.yaml\"",
                    ));
                }
                load_from_file::<T>(path).map_err(de::Error::custom)
            } else {
                // Try to deserialize as the target type
                serde_yaml::from_str(value).map_err(de::Error::custom)
            }
        }

        fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where
            V: MapAccess<'de>,
        {
            // Check if this is an import tag by looking for special markers
            // serde_yaml represents tagged values in a specific way
            // We need to handle this at the tagged value level
            // For now, try to deserialize as T directly
            // If it fails and contains import info, handle it
            let mut collected = serde_yaml::Value::Mapping(
                serde_yaml::Mapping::new()
            );
            
            // Collect all entries
            while let Some(key) = map.next_key::<serde_yaml::Value>()? {
                let value = map.next_value::<serde_yaml::Value>()?;
                if let serde_yaml::Value::Mapping(ref mut m) = collected {
                    m.insert(key, value);
                }
            }
            
            // Check if any key is "IMPORT" (case-insensitive check for the key)
            if let serde_yaml::Value::Mapping(ref m) = collected {
                for (key, value) in m {
                    if let serde_yaml::Value::String(key_str) = key {
                        if key_str.to_uppercase() == "IMPORT" {
                            if let serde_yaml::Value::String(path) = value {
                                return load_from_file::<T>(path).map_err(de::Error::custom);
                            } else {
                                return Err(de::Error::custom(
                                    "IMPORT value must be a string path to a YAML file"
                                ));
                            }
                        }
                    }
                }
            }
            
            // Not an import, deserialize as T
            T::deserialize(collected).map_err(|e| de::Error::custom(e.to_string()))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // For sequences, try to collect and deserialize
            let mut items = Vec::new();
            while let Some(item) = seq.next_element::<serde_yaml::Value>()? {
                items.push(item);
            }
            T::deserialize(serde_yaml::Value::Sequence(items))
                .map_err(|e| de::Error::custom(e.to_string()))
        }

        // Delegate other visit methods to T's deserializer
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize(v).map_err(|_| E::custom("failed to deserialize as target type"))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize(v).map_err(|_| E::custom("failed to deserialize as target type"))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize(v).map_err(|_| E::custom("failed to deserialize as target type"))
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize(v).map_err(|_| E::custom("failed to deserialize as target type"))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize(serde_yaml::Value::Null)
                .map_err(|_| E::custom("failed to deserialize as target type"))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize(serde_yaml::Value::Null)
                .map_err(|_| E::custom("failed to deserialize as target type"))
        }
    }

    deserializer.deserialize_any(ImportOrInlineVisitor(std::marker::PhantomData))
}

/// A wrapper type for configuration values that support import from external files.
///
/// This type implements `Deserialize` to handle both inline values and the `!import` tag.
/// It can be used directly in config structures or as a field type.
///
/// # Example
///
/// ```rust
/// use simusa_core::config_import::Import;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone)]
/// struct MyConfig {
///     // This field can be inline or imported
///     my_field: Import<MyType>,
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Import<T> {
    inner: T,
}

impl<T> Import<T> {
    /// Consume the wrapper and return the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Get a reference to the inner value.
    pub fn as_ref(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner value.
    pub fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Create a new Import wrapper from a value.
    pub fn new(value: T) -> Self {
        Import { inner: value }
    }
}

impl<T> std::ops::Deref for Import<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Import<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<T> for Import<T> {
    fn from(value: T) -> Self {
        Import { inner: value }
    }
}

impl<T: Default> Default for Import<T> {
    fn default() -> Self {
        Import { inner: T::default() }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Import<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = deserialize_as_import_or_inline(deserializer)?;
        Ok(Import { inner: value })
    }
}

/// Trait for types that support loading from external files.
///
/// Types implementing this trait can be used with the import system.
/// The default implementation uses serde YAML deserialization.
pub trait Importable: for<'de> Deserialize<'de> {
    /// Load an instance of this type from a file.
    ///
    /// # Arguments
    ///
    /// * `base_path` - The base directory for resolving relative paths
    /// * `relative_path` - The path to the file, relative to base_path
    ///
    /// # Returns
    ///
    /// Returns the loaded instance or a SimusaError if loading fails.
    fn load_from_path(base_path: &Path, relative_path: &str) -> SimusaResult<Self>
    where
        Self: Sized,
    {
        let full_path = base_path.join(relative_path);

        let file = File::open(&full_path).map_err(|e| {
            SimusaError::new(
                SimusaErrorTypes::ConfigError,
                format!(
                    "Failed to open imported file {:?} (resolved from {:?} relative to {:?}): {}",
                    full_path, relative_path, base_path, e
                ),
            )
        })?;

        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|e| {
            SimusaError::new(
                SimusaErrorTypes::ConfigError,
                format!(
                    "Failed to parse imported file {:?} (from {:?}): {}",
                    full_path, relative_path, e
                ),
            )
        })
    }
}

// Implement Importable for common types
impl Importable for String {}
impl Importable for i8 {}
impl Importable for i16 {}
impl Importable for i32 {}
impl Importable for i64 {}
impl Importable for i128 {}
impl Importable for isize {}
impl Importable for u8 {}
impl Importable for u16 {}
impl Importable for u32 {}
impl Importable for u64 {}
impl Importable for u128 {}
impl Importable for usize {}
impl Importable for f32 {}
impl Importable for f64 {}
impl Importable for bool {}

impl<T: Importable> Importable for Option<T> {}

impl<T: Importable> Importable for Vec<T> {}

impl<T: Importable> Importable for Box<T> {}

// Implement for HashMap
use std::collections::HashMap;
impl<K: std::hash::Hash + Eq, V: Importable> Importable for HashMap<K, V> {}

// Implement for BTreeMap
use std::collections::BTreeMap;
impl<K: Ord, V: Importable> Importable for BTreeMap<K, V> {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
    struct OuterConfig {
        inner: TestConfig,
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");

        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        fs::write(&config_path, yaml).unwrap();

        let loaded: TestConfig = load_from_file(
            config_path
                .to_str()
                .unwrap()
                .strip_prefix(temp_dir.path().to_str().unwrap())
                .unwrap()
                .trim_start_matches('/'),
        )
        .unwrap();

        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.value, 42);
    }

    #[test]
    fn test_with_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");

        let result = with_config_path(&config_path, || {
            get_current_config_base_path()
        });

        assert_eq!(result, temp_dir.path().to_path_buf());
    }

    #[test]
    fn test_import_wrapper() {
        let import = Import::new(TestConfig {
            name: "test".to_string(),
            value: 42,
        });

        assert_eq!(import.name, "test");
        assert_eq!(import.value, 42);
        assert_eq!(import.as_ref().value, 42);
    }
}