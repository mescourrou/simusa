//! Shared utility modules and helper types used across Simusa.
//!
//! This module provides serialization helpers plus common shared-pointer aliases.

pub mod barrier;
pub mod confy;
pub mod determinist_random_variable;
pub mod distributions;
pub mod enum_tools;
pub mod frame;
pub mod geometry;
pub(crate) mod log_rotation;
pub mod macros;
pub mod maths;
pub mod numbers;
pub mod occupancy_grid;
pub mod periodicity;
pub mod read_only_lock;

use std::str::FromStr;

use serde::Serializer;
use simusa_macros::config_derives;

/// Serialize an `f32` using a stable high-precision scientific representation.
pub fn format_f32<S>(val: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{:.40e}", val);
    let number: f64 = s.parse().unwrap();
    serializer.serialize_f64(number)
}

/// Serialize an optional `f32` using [`format_f32`] when present.
pub fn format_option_f32<S>(val: &Option<f32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match val {
        Some(inner) => format_f32(inner, serializer),
        None => serializer.serialize_none(),
    }
}

/// Shared trait-object read-only lock (Shorthand for `Arc<dyn read_only_lock::RoLock<T>>`).
pub type SharedRoLock<T> = std::sync::Arc<dyn read_only_lock::RoLock<T>>;
/// Shared read-write lock (Shorthand for `Arc<RwLock<T>>`).
pub type SharedRwLock<T> = std::sync::Arc<std::sync::RwLock<T>>;
/// Shared mutex lock (Shorthand for `Arc<Mutex<T>>`).
pub type SharedMutex<T> = std::sync::Arc<std::sync::Mutex<T>>;

#[config_derives(skip_to_vec, skip_check)]
pub struct GeneralValue(pub serde_json::Value);

impl Default for GeneralValue {
    fn default() -> Self {
        Self(serde_json::Value::Null)
    }
}

impl From<String> for GeneralValue {
    fn from(s: String) -> Self {
        match serde_json::from_str(&s) {
            Ok(value) => GeneralValue(value),
            Err(_) => GeneralValue(serde_json::Value::String(s)),
        }
    }
}

impl ToString for GeneralValue {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl From<GeneralValue> for String {
    fn from(gv: GeneralValue) -> Self {
        gv.0.to_string()
    }
}

impl FromStr for GeneralValue {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match serde_json::from_str(s) {
            Ok(value) => Ok(GeneralValue(value)),
            Err(_) => Ok(GeneralValue(serde_json::Value::String(s.to_string()))),
        }
    }
}
