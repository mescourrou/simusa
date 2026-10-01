//! Definitions of message types and transport envelopes used in the networking layer.

use simusa_macros::{EnumToString, config_derives};

use crate::{
    navigators::go_to::GoToMessage,
    scenario::config::EventRecord,
    sensors::{
        Observation, ObservationRecord,
        sensor_manager::{SensorFrameMessage, SensorTriggerMessage},
    },
};

/// Payload variants that can transit through the network.
///
/// This enum is exposed to Python through `pyo3` and is serializable for transport.
#[config_derives]
pub enum MessageType {
    /// Arbitrary UTF-8 textual payload.
    String(String),
    /// Navigation payload used by [`GoToMessage`].
    GoTo(GoToMessage),
    /// Sensor event payload used by [`SensorTriggerMessage`].
    SensorTrigger(SensorTriggerMessage),
    /// Observations payload used by [`Observation`].
    Observations(Vec<ObservationRecord>),
    /// Setting the frame of a sensor, used by [`SensorManager`](crate::sensors::sensor_manager::SensorManager).
    SensorFrame(SensorFrameMessage),
    /// Scenario event record payload used by [`EventRecord`].
    Event(EventRecord),
    /// Custom payloads that can be serialized and deserialized as needed.
    Custom(Vec<u8>),
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::String(String::new())
    }
}

macro_rules! impl_message_types_from {
    ($($variant:ident, $type:ty),* $(,)?) => {
        $(
            impl From<$type> for MessageType {
                fn from(value: $type) -> Self {
                    MessageType::$variant(value)
                }
            }

            impl From<MessageType> for $type {
                fn from(value: MessageType) -> Self {
                    match value {
                        MessageType::$variant(inner) => inner,
                        _ => panic!("Cannot convert MessageType::{} into {}", value, stringify!($type)),
                    }
                }
            }
        )*
    };
}

impl_message_types_from!(
    String,
    String,
    GoTo,
    GoToMessage,
    SensorTrigger,
    SensorTriggerMessage,
    SensorFrame,
    SensorFrameMessage,
    Observations,
    Vec<ObservationRecord>,
    Event,
    EventRecord,
    Custom,
    Vec<u8>,
);

impl From<&str> for MessageType {
    fn from(value: &str) -> Self {
        MessageType::String(value.to_string())
    }
}
